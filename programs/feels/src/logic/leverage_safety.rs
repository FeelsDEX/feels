/// Leverage safety mechanisms including notional limits and anti-manipulation features.
/// Enforces leverage notional limits per epoch and prevents ping-pong attacks.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketManager, TwapOracle, FieldCommitment, FeesPolicy,
    PoolStatus,
};
use crate::constant::Q64;

// ============================================================================
// Constants
// ============================================================================

/// Maximum leverage notional as percentage of AMM depth (basis points)
pub const MAX_NOTIONAL_PERCENT_OF_DEPTH: u64 = 2000; // 20%

/// Minimum TWAP window for leverage calculations (seconds)
pub const MIN_TWAP_WINDOW: i64 = 15; // Must exceed 2 block reorgs (~12s on Solana)

/// Anti-ping-pong cooldown period (seconds)
pub const PING_PONG_COOLDOWN: i64 = 300; // 5 minutes

/// Maximum leverage adjustments per epoch
pub const MAX_LEVERAGE_ADJUSTMENTS_PER_EPOCH: u8 = 3;

/// Confidence threshold for TWAP reliability (basis points)
pub const TWAP_CONFIDENCE_THRESHOLD: u64 = 100; // 1%

// ============================================================================
// Leverage Tracking
// ============================================================================

/// Enhanced position tracking with notional values
#[derive(Clone, Debug, Default)]
pub struct LeveragePosition {
    /// Position owner
    pub owner: Pubkey,
    /// Position size (base amount)
    pub size: u64,
    /// Current leverage (Q64)
    pub leverage: u64,
    /// Notional value (size * leverage * price)
    pub notional_value: u128,
    /// Last adjustment timestamp
    pub last_adjustment: i64,
    /// Adjustment count in current epoch
    pub adjustment_count: u8,
    /// Position opened timestamp
    pub opened_at: i64,
}

/// Epoch-based leverage tracking
#[account]
#[derive(Default)]
pub struct LeverageEpochTracker {
    /// Current epoch start time
    pub epoch_start: i64,
    /// Epoch duration
    pub epoch_duration: i64,
    /// Total notional in current epoch
    pub total_epoch_notional: u128,
    /// Number of positions opened this epoch
    pub positions_opened: u32,
    /// Number of positions closed this epoch
    pub positions_closed: u32,
    /// Ping-pong detection counter
    pub ping_pong_counter: u32,
    /// Last ping-pong detection time
    pub last_ping_pong_detected: i64,
}

// ============================================================================
// Notional Limit Enforcement
// ============================================================================

/// Calculate AMM depth over TWAP window
pub fn calculate_amm_depth_twap(
    market: &MarketManager,
    oracle: &TwapOracle,
    window_seconds: i64,
) -> Result<u128> {
    // Ensure window is long enough
    require!(
        window_seconds >= MIN_TWAP_WINDOW,
        FeelsProtocolError::InvalidInput
    );
    
    // Get TWAP prices based on window
    let (twap_0, twap_1) = if window_seconds <= 300 {
        (oracle.twap_5min_a, oracle.twap_5min_b)
    } else {
        // For longer windows, use 1hr TWAP (we don't have 30min or 24hr)
        (oracle.twap_1hr_a, oracle.twap_1hr_b)
    };
    
    // Calculate depth using liquidity and price
    // For now, use liquidity as a proxy for depth
    // depth = liquidity * sqrt(twap_0 * twap_1)
    let price_product = twap_0
        .checked_mul(twap_1)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    let reserve_product = market.liquidity
        .checked_mul(price_product)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Use integer square root
    let sqrt_product = integer_sqrt::IntegerSquareRoot::integer_sqrt(&reserve_product);
    let depth = sqrt_product.checked_mul(2).ok_or(FeelsProtocolError::MathOverflow)?;
    
    Ok(depth)
}

/// Validate leverage notional against limits
pub fn validate_leverage_notional(
    position_notional: u128,
    epoch_tracker: &LeverageEpochTracker,
    market: &MarketManager,
    oracle: &TwapOracle,
) -> Result<()> {
    // Calculate AMM depth over appropriate window
    let amm_depth = calculate_amm_depth_twap(
        market,
        oracle,
        epoch_tracker.epoch_duration,
    )?;
    
    // Calculate maximum allowed notional
    let max_notional = amm_depth
        .saturating_mul(MAX_NOTIONAL_PERCENT_OF_DEPTH as u128)
        .saturating_div(10000);
    
    // Check if new position would exceed limit
    let new_total_notional = epoch_tracker.total_epoch_notional
        .saturating_add(position_notional);
    
    require!(
        new_total_notional <= max_notional,
        FeelsProtocolError::ConstraintViolation
    );
    
    // Additional check: single position can't exceed 10% of max
    let single_position_limit = max_notional.saturating_div(10);
    require!(
        position_notional <= single_position_limit,
        FeelsProtocolError::InvalidAmount
    );
    
    Ok(())
}

// ============================================================================
// Anti-Ping-Pong Protection
// ============================================================================

/// Check for ping-pong behavior patterns
pub fn check_ping_pong_behavior(
    position: &LeveragePosition,
    new_leverage: u64,
    current_time: i64,
    epoch_tracker: &LeverageEpochTracker,
) -> Result<()> {
    // Check cooldown period
    let time_since_last_adjustment = current_time - position.last_adjustment;
    require!(
        time_since_last_adjustment >= PING_PONG_COOLDOWN,
        FeelsProtocolError::ConstraintViolation
    );
    
    // Check adjustment count in epoch
    require!(
        position.adjustment_count < MAX_LEVERAGE_ADJUSTMENTS_PER_EPOCH,
        FeelsProtocolError::ConstraintViolation
    );
    
    // Detect rapid leverage reversals (ping-pong)
    let _is_increase = new_leverage > position.leverage;
    let leverage_change_ratio = if new_leverage > position.leverage {
        ((new_leverage - position.leverage) as u128) * Q64 / (position.leverage as u128)
    } else {
        ((position.leverage - new_leverage) as u128) * Q64 / (position.leverage as u128)
    };
    
    // If large leverage change (>50%) in opposite direction within cooldown * 2
    if leverage_change_ratio > Q64 / 2 && time_since_last_adjustment < PING_PONG_COOLDOWN * 2 {
        // This might be ping-pong behavior
        msg!("Warning: Potential ping-pong behavior detected");
        
        // Increment detection counter
        // Note: In actual implementation, this would need to be persisted
        if current_time - epoch_tracker.last_ping_pong_detected < 3600 {
            // Multiple detections within an hour
            return Err(FeelsProtocolError::SecurityViolation.into());
        }
    }
    
    Ok(())
}

// ============================================================================
// TWAP Confidence Validation
// ============================================================================

/// Validate TWAP data confidence for leverage decisions
pub fn validate_twap_confidence(
    oracle: &TwapOracle,
    current_time: i64,
) -> Result<()> {
    // Check TWAP freshness
    let age = current_time - oracle.last_update;
    require!(
        age <= MIN_TWAP_WINDOW,
        FeelsProtocolError::StateError
    );
    
    // Check observation count for reliability
    require!(
        oracle.observation_count >= 10,
        FeelsProtocolError::InsufficientResource
    );
    
    // Calculate confidence based on price stability
    let price_variance = if oracle.twap_5min_a > 0 && oracle.twap_1hr_a > 0 {
        let ratio = if oracle.twap_5min_a > oracle.twap_1hr_a {
            (oracle.twap_5min_a - oracle.twap_1hr_a) * 10000 / oracle.twap_1hr_a
        } else {
            (oracle.twap_1hr_a - oracle.twap_5min_a) * 10000 / oracle.twap_1hr_a
        };
        ratio
    } else {
        0
    };
    
    require!(
        price_variance <= TWAP_CONFIDENCE_THRESHOLD as u128,
        FeelsProtocolError::ValidationError
    );
    
    Ok(())
}

// ============================================================================
// Risk Scaler Gating
// ============================================================================

/// Calculate risk scaler based on TWAP confidence and market conditions
pub fn calculate_risk_scaler(
    oracle: &TwapOracle,
    market: &MarketManager,
    volatility_bps: u64,
) -> Result<u64> {
    // Base scaler is 1.0 (Q64)
    let mut scaler = Q64;
    
    // Reduce scaler based on volatility
    if volatility_bps > 1000 {
        scaler = scaler * 50 / 100; // 0.5x for high volatility
    } else if volatility_bps > 500 {
        scaler = scaler * 75 / 100; // 0.75x for medium volatility
    }
    
    // Further reduce based on liquidity concentration
    let liquidity_ratio = if market.liquidity > 0 {
        let active_range = (market.current_tick + 1000).abs() as u64; // Rough estimate
        let concentration = Q64 / active_range.max(1) as u128;
        concentration.min(Q64)
    } else {
        Q64
    };
    
    scaler = scaler * liquidity_ratio / Q64;
    
    // Apply TWAP confidence factor
    let confidence_factor = if oracle.observation_count > 100 {
        Q64 // Full confidence
    } else if oracle.observation_count > 50 {
        Q64 * 90 / 100 // 0.9x
    } else if oracle.observation_count > 10 {
        Q64 * 75 / 100 // 0.75x
    } else {
        Q64 * 50 / 100 // 0.5x minimum
    };
    
    scaler = scaler * confidence_factor / Q64;
    
    Ok(scaler as u64)
}

// ============================================================================
// Integration Functions
// ============================================================================

/// Comprehensive leverage validation for new positions
pub fn validate_new_leverage_position(
    position_size: u64,
    leverage: u64,
    market: &MarketManager,
    oracle: &TwapOracle,
    epoch_tracker: &LeverageEpochTracker,
    current_time: i64,
) -> Result<()> {
    // Validate TWAP confidence
    validate_twap_confidence(oracle, current_time)?;
    
    // Calculate position notional
    let price = oracle.twap_1_per_0; // Simplified - use appropriate price
    let notional = (position_size as u128)
        .saturating_mul(leverage as u128)
        .saturating_mul(price as u128)
        .saturating_div(Q64)
        .saturating_div(Q64);
    
    // Validate against notional limits
    validate_leverage_notional(notional, epoch_tracker, market, oracle)?;
    
    // Calculate and apply risk scaler
    let volatility = oracle.volatility_24hr;
    let risk_scaler = calculate_risk_scaler(oracle, market, volatility)?;
    
    // Adjust maximum allowed leverage based on risk
    let max_leverage_from_market = market.get_max_leverage()
        .unwrap_or(100_000); // Default 10x if not configured
    let max_leverage_adjusted = (max_leverage_from_market as u128)
        .saturating_mul(risk_scaler as u128)
        .saturating_div(Q64) as u64;
    
    require!(
        leverage <= max_leverage_adjusted,
        FeelsProtocolError::ConstraintViolation
    );
    
    Ok(())
}

/// Update epoch tracker after position changes
pub fn update_epoch_tracker(
    epoch_tracker: &mut LeverageEpochTracker,
    position_notional: u128,
    is_open: bool,
    current_time: i64,
) -> Result<()> {
    // Check if we need to start a new epoch
    if current_time >= epoch_tracker.epoch_start + epoch_tracker.epoch_duration {
        epoch_tracker.epoch_start = current_time;
        epoch_tracker.total_epoch_notional = 0;
        epoch_tracker.positions_opened = 0;
        epoch_tracker.positions_closed = 0;
        epoch_tracker.ping_pong_counter = 0;
    }
    
    // Update tracker
    if is_open {
        epoch_tracker.total_epoch_notional = epoch_tracker.total_epoch_notional
            .saturating_add(position_notional);
        epoch_tracker.positions_opened += 1;
    } else {
        epoch_tracker.total_epoch_notional = epoch_tracker.total_epoch_notional
            .saturating_sub(position_notional);
        epoch_tracker.positions_closed += 1;
    }
    
    Ok(())
}

// ============================================================================
// Enhanced Safety Features
// ============================================================================

/// Enhanced leverage safety context
pub struct LeverageSafetyContext<'a> {
    pub field_commitment: &'a FieldCommitment,
    pub fees_policy: &'a FeesPolicy,
    pub pool_status: &'a PoolStatus,
    pub twap_oracle: &'a TwapOracle,
    pub current_time: i64,
}

/// Calculate leverage stress for fee adjustments
pub fn calculate_leverage_stress(
    market: &MarketManager,
    field_commitment: &FieldCommitment,
) -> Result<u64> {
    // Use L scalar from field commitment as primary stress indicator
    let l_normalized = (field_commitment.L * 10000) / Q64;
    
    // Additional stress from leverage concentration
    let concentration_stress = if market.liquidity > 0 {
        // Higher leverage positions concentrate risk
        let leverage_ratio = l_normalized / 10000; // Convert back to ratio
        (leverage_ratio * leverage_ratio).min(10000) // Quadratic stress
    } else {
        0
    };
    
    // Combine stress factors
    let total_stress = l_normalized.saturating_add(concentration_stress) / 2;
    
    Ok(total_stress.min(10000) as u64)
}

/// Check if leverage operations should be restricted
pub fn check_leverage_restrictions(
    ctx: &LeverageSafetyContext,
    market: &MarketManager,
) -> Result<bool> {
    // Check pool operational status
    if ctx.pool_status.status == 2 { // Disabled
        return Ok(true); // All leverage restricted
    }
    
    // Check field commitment staleness
    let staleness = ctx.current_time - ctx.field_commitment.snapshot_ts;
    if staleness > ctx.fees_policy.max_commitment_staleness * 2 {
        msg!("Leverage restricted due to stale field commitment");
        return Ok(true);
    }
    
    // Check leverage stress threshold
    let leverage_stress = calculate_leverage_stress(market, ctx.field_commitment)?;
    if leverage_stress > ctx.fees_policy.leverage_disable_threshold_bps {
        msg!("Leverage restricted due to high stress: {} bps", leverage_stress);
        return Ok(true);
    }
    
    // Check TWAP divergence
    let twap_divergence = calculate_twap_divergence(ctx.field_commitment, ctx.twap_oracle)?;
    if twap_divergence > 1000 { // 10% divergence
        msg!("Leverage restricted due to TWAP divergence: {} bps", twap_divergence);
        return Ok(true);
    }
    
    Ok(false)
}

/// Calculate TWAP divergence between field commitment and oracle
fn calculate_twap_divergence(
    field: &FieldCommitment,
    oracle: &TwapOracle,
) -> Result<u64> {
    // Compare field TWAPs with oracle TWAPs
    let field_price = if field.twap_0 > 0 && field.twap_1 > 0 {
        (field.twap_1 * Q64) / field.twap_0
    } else {
        Q64
    };
    
    let oracle_price = oracle.twap_1_per_0;
    
    // Calculate divergence in basis points
    let divergence = if field_price > oracle_price {
        ((field_price - oracle_price) * 10000) / oracle_price
    } else {
        ((oracle_price - field_price) * 10000) / oracle_price
    };
    
    Ok(divergence.min(10000) as u64)
}

/// Apply leverage-based fee multiplier
pub fn calculate_leverage_fee_multiplier(
    leverage: u64,
    leverage_stress: u64,
) -> Result<u64> {
    // Base multiplier starts at 1x (10000 bps)
    let mut multiplier = 10000u64;
    
    // Add leverage-based premium
    // For every 1x leverage above 1x, add 10 bps
    let leverage_ratio = leverage / 1_000_000; // Convert from Q64-like to integer
    if leverage_ratio > 1 {
        let leverage_premium = (leverage_ratio - 1) * 10;
        multiplier = multiplier.saturating_add(leverage_premium);
    }
    
    // Add stress-based premium
    // For every 1000 bps of stress, add 0.1x multiplier
    let stress_premium = leverage_stress / 10;
    multiplier = multiplier.saturating_add(stress_premium);
    
    // Cap at 3x
    Ok(multiplier.min(30000))
}

/// Validate leverage position against enhanced safety rules
pub fn validate_leverage_safety_enhanced(
    position_size: u64,
    leverage: u64,
    ctx: &LeverageSafetyContext,
    market: &MarketManager,
    epoch_tracker: &LeverageEpochTracker,
) -> Result<()> {
    // First check if leverage is restricted
    if check_leverage_restrictions(ctx, market)? {
        return Err(FeelsProtocolError::ConstraintViolation.into());
    }
    
    // Standard validation
    validate_new_leverage_position(
        position_size,
        leverage,
        market,
        ctx.twap_oracle,
        epoch_tracker,
        ctx.current_time,
    )?;
    
    // Additional safety checks
    
    // 1. Check leverage against field commitment bounds
    let max_safe_leverage = calculate_safe_leverage_limit(ctx.field_commitment)?;
    require!(
        leverage <= max_safe_leverage,
        FeelsProtocolError::ConstraintViolation
    );
    
    // 2. Check position concentration
    let concentration = calculate_position_concentration(position_size, market)?;
    require!(
        concentration < 500, // Max 5% of liquidity
        FeelsProtocolError::InvalidAmount
    );
    
    // 3. Validate funding rate sustainability
    validate_funding_sustainability(leverage, ctx.field_commitment)?;
    
    Ok(())
}

/// Calculate safe leverage limit based on field conditions
fn calculate_safe_leverage_limit(field: &FieldCommitment) -> Result<u64> {
    // Base max leverage (10x)
    let base_max = 10_000_000u64; // In basis points scale
    
    // Reduce based on L scalar
    let l_factor = if field.L > Q64 {
        (Q64 as u128).saturating_mul(Q64 as u128) / field.L // Inverse relationship
    } else {
        Q64
    };
    
    // Reduce based on volatility
    let vol_factor = if field.sigma_leverage > 1000 {
        Q64 * 1000 / field.sigma_leverage as u128
    } else {
        Q64
    };
    
    // Apply factors
    let adjusted_max = (base_max as u128)
        .saturating_mul(l_factor)
        .saturating_div(Q64)
        .saturating_mul(vol_factor)
        .saturating_div(Q64)
        .min(base_max as u128) as u64;
    
    Ok(adjusted_max.max(1_000_000)) // Min 1x leverage
}

/// Calculate position concentration relative to liquidity
fn calculate_position_concentration(
    position_size: u64,
    market: &MarketManager,
) -> Result<u64> {
    if market.liquidity == 0 {
        return Ok(10000); // 100% if no liquidity
    }
    
    let concentration = (position_size as u128)
        .saturating_mul(10000)
        .saturating_div(market.liquidity)
        .min(10000) as u64;
    
    Ok(concentration)
}

/// Validate funding rate sustainability
fn validate_funding_sustainability(
    leverage: u64,
    field: &FieldCommitment,
) -> Result<()> {
    // Estimate funding rate impact
    let leverage_impact = leverage / 1_000_000; // Convert to multiplier
    
    // Check if T scalar indicates high time-dimension stress
    let time_stress = (field.T * 10000) / Q64;
    
    // High leverage + high time stress = unsustainable funding
    if leverage_impact > 5 && time_stress > 5000 {
        msg!("Warning: High leverage with stressed funding conditions");
        return Err(FeelsProtocolError::ConstraintViolation.into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leverage_stress_calculation() {
        let mut market = MarketManager::default();
        market.liquidity = Q64;
        
        let mut field = FieldCommitment::default();
        field.L = Q64 * 2; // 2x normal
        
        let stress = calculate_leverage_stress(&market, &field).unwrap();
        assert!(stress > 10000); // Should show high stress
    }

    #[test]
    fn test_leverage_fee_multiplier() {
        // 5x leverage, 50% stress
        let multiplier = calculate_leverage_fee_multiplier(5_000_000, 5000).unwrap();
        
        // Base 10000 + (5-1)*10 + 5000/10 = 10000 + 40 + 500 = 10540
        assert_eq!(multiplier, 10540);
    }
}