/// Field update logic for the market field commitment strategy.
/// Handles updates to market scalars and risk parameters.
use anchor_lang::prelude::*;
use crate::state::{MarketField, TwapOracle, MarketManager, TokenPriceOracle, VolumeTracker};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum change in market scalars per update (basis points)
pub const MAX_SCALAR_CHANGE_BPS: u32 = 200; // 2%

/// Minimum time between field updates (seconds)
pub const MIN_UPDATE_INTERVAL: i64 = 60; // 1 minute

/// Maximum risk scalar value (basis points)
pub const MAX_RISK_SCALAR_BPS: u64 = 10000; // 100%

// ============================================================================
// Field Update Logic
// ============================================================================

/// Update market field data from pool state
pub fn update_market_field(
    field: &mut MarketField,
    pool: &MarketManager,
    twap_oracle: &TwapOracle,
    token_price_oracle: Option<&TokenPriceOracle>,
    vault_0_balance: Option<u64>,
    vault_1_balance: Option<u64>,
    current_time: i64,
) -> Result<()> {
    // Check update frequency
    require!(
        current_time - field.snapshot_ts >= MIN_UPDATE_INTERVAL,
        FeelsProtocolError::UpdateTooFrequent
    );
    
    // Get token TWAPs from oracle or calculate from pool
    let (twap_0, twap_1) = if let Some(price_oracle) = token_price_oracle {
        // Use token-specific price oracles if available
        if price_oracle.is_fresh(current_time, 300) { // 5 minute staleness
            price_oracle.get_token_twaps()?
        } else {
            // Oracle is stale, fall back to pool calculation
            msg!("Token price oracle is stale, using pool TWAP");
            let twap_window = crate::state::DEFAULT_TWAP_WINDOW.min(field.max_staleness);
            let current_price = twap_oracle.get_twap(twap_window, current_time)?;
            calculate_token_twaps(current_price, pool)?
        }
    } else {
        // No oracle available, use pool TWAP
        let twap_window = crate::state::DEFAULT_TWAP_WINDOW.min(field.max_staleness);
        let current_price = twap_oracle.get_twap(twap_window, current_time)?;
        calculate_token_twaps(current_price, pool)?
    };
    
    // Calculate new market scalars
    let new_scalars = calculate_market_scalars(pool, twap_0, twap_1, vault_0_balance, vault_1_balance)?;
    
    // Validate scalar changes are within bounds
    validate_scalar_changes(field, &new_scalars)?;
    
    // Update field data
    field.S = new_scalars.S;
    field.T = new_scalars.T;
    field.L = new_scalars.L;
    field.twap_0 = twap_0;
    field.twap_1 = twap_1;
    field.snapshot_ts = current_time;
    
    // Validate updated field
    field.validate()?;
    
    Ok(())
}

/// Calculate token-specific TWAPs from pool price
fn calculate_token_twaps(pool_price: u128, _pool: &MarketManager) -> Result<(u128, u128)> {
    // For simplicity, assume pool price is token_0/token_1
    // and token_1 is the numeraire (value = 1)
    let twap_1 = 1u128 << 64; // Q64 fixed point
    let twap_0 = pool_price;
    
    Ok((twap_0, twap_1))
}

/// Market scalars calculated from pool state
#[derive(Debug)]
#[allow(non_snake_case)]
struct MarketScalars {
    pub S: u128,
    pub T: u128,
    pub L: u128,
}

/// Calculate market scalars from pool state
#[allow(non_snake_case)]
fn calculate_market_scalars(
    pool: &MarketManager,
    twap_0: u128,
    twap_1: u128,
    vault_0_balance: Option<u64>,
    vault_1_balance: Option<u64>,
) -> Result<MarketScalars> {
    // Calculate spot scalar S
    // S = (x_a * p_a)^ω_a * (x_b * p_b)^ω_b / sqrt(1 + σ_price²)
    let S = calculate_spot_scalar(pool, twap_0, twap_1, vault_0_balance, vault_1_balance)?;
    
    // Calculate time scalar T
    // Pass volume tracker if available
    let T = calculate_time_scalar(pool, None)?; // None for now, would pass actual tracker
    
    // Calculate leverage scalar L
    // For now, use placeholder
    let L = calculate_leverage_scalar(pool)?;
    
    Ok(MarketScalars { S, T, L })
}

/// Calculate spot dimension scalar
fn calculate_spot_scalar(
    pool: &MarketManager,
    twap_0: u128,
    twap_1: u128,
    vault_0_balance: Option<u64>,
    vault_1_balance: Option<u64>,
) -> Result<u128> {
    // Get token balances - use actual vault balances if provided
    let balance_0 = if let Some(vault_balance) = vault_0_balance {
        vault_balance as u128
    } else {
        estimate_token_balance_0(pool)?
    };
    
    let balance_1 = if let Some(vault_balance) = vault_1_balance {
        vault_balance as u128
    } else {
        estimate_token_balance_1(pool)?
    };
    
    // Calculate numeraire values
    let value_0 = balance_0
        .checked_mul(twap_0)
        .ok_or(FeelsProtocolError::MathOverflow)?
        >> 64; // Adjust for Q64
        
    let value_1 = balance_1
        .checked_mul(twap_1)
        .ok_or(FeelsProtocolError::MathOverflow)?
        >> 64;
    
    // For equal weights, use geometric mean
    let spot_value = crate::utils::safe::sqrt_u128(
        value_0
            .checked_mul(value_1)
            .ok_or(FeelsProtocolError::MathOverflow)?
    );
    
    // Apply risk scaling (simplified - no risk for now)
    Ok(spot_value)
}

/// Calculate time dimension scalar
fn calculate_time_scalar(
    pool: &MarketManager,
    volume_tracker: Option<&VolumeTracker>,
) -> Result<u128> {
    if let Some(tracker) = volume_tracker {
        // Use actual lending/borrowing volumes
        let (util_0, util_1) = tracker.get_utilization_rate();
        
        // Average utilization weighted by time deposits
        let avg_utilization = (util_0 + util_1) / 2;
        
        // Scale to time dimension: higher utilization = higher time value
        // T = liquidity * (1 + utilization_rate)
        let time_factor = 10000 + avg_utilization; // 10000 = 100% base
        
        let scaled = (pool.liquidity as u128)
            .saturating_mul(time_factor as u128)
            .saturating_div(10000);
            
        Ok(scaled.max(1u128 << 64))
    } else {
        // Fallback: use pool liquidity as proxy
        Ok(pool.liquidity.max(1u128 << 64))
    }
}

/// Calculate leverage dimension scalar
fn calculate_leverage_scalar(pool: &MarketManager) -> Result<u128> {
    // MarketManager already tracks leverage positions
    // Use average leverage as the scalar
    let leverage_scalar = (pool.avg_leverage_bps as u128 * crate::constant::Q64) / 10000;
    Ok(leverage_scalar.max(1u128 << 64))
}

/// Estimate token 0 balance from pool state
fn estimate_token_balance_0(pool: &MarketManager) -> Result<u128> {
    // Simplified: use liquidity and price
    // L = sqrt(x * y) at current price
    // x = L² / y, y = L² / x
    
    let sqrt_price = pool.current_sqrt_rate;
    if sqrt_price == 0 {
        return Ok(0);
    }
    
    // x = L / sqrt_price (in token units) - use safe math for critical financial calculation
    let shifted_liquidity = crate::utils::safe::safe_shl_u128(pool.liquidity, 96)?;
    let balance = crate::utils::safe::div_u128(shifted_liquidity, sqrt_price)?;
    Ok(balance)
}

/// Estimate token 1 balance from pool state
fn estimate_token_balance_1(pool: &MarketManager) -> Result<u128> {
    // y = L * sqrt_price (in token units) - use safe math for critical financial calculation
    let product = crate::utils::safe::mul_u128(pool.liquidity, pool.current_sqrt_rate)?;
    let balance = crate::utils::safe::safe_shr_u128(product, 96)?;
    Ok(balance)
}

/// Validate scalar changes are within allowed bounds
fn validate_scalar_changes(
    current: &MarketField,
    new: &MarketScalars,
) -> Result<()> {
    // Check S change
    let s_change_bps = calculate_change_bps(current.S, new.S);
    require!(
        s_change_bps <= MAX_SCALAR_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange
    );
    
    // Check T change
    let t_change_bps = calculate_change_bps(current.T, new.T);
    require!(
        t_change_bps <= MAX_SCALAR_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange
    );
    
    // Check L change
    let l_change_bps = calculate_change_bps(current.L, new.L);
    require!(
        l_change_bps <= MAX_SCALAR_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange
    );
    
    Ok(())
}

/// Calculate change in basis points
fn calculate_change_bps(old: u128, new: u128) -> u32 {
    if old == 0 {
        return if new == 0 { 0 } else { 10000 };
    }
    
    let change = if new > old {
        new - old
    } else {
        old - new
    };
    
    ((change * 10000) / old).min(10000) as u32
}

// ============================================================================
// Risk Parameter Updates
// ============================================================================

/// Update risk parameters based on market conditions
pub fn update_risk_parameters(
    field: &mut MarketField,
    volatility_bps: u64,
) -> Result<()> {
    // Update risk scalers based on observed volatility
    
    // Price risk
    field.sigma_price = volatility_bps.min(MAX_RISK_SCALAR_BPS);
    
    // Rate risk (typically lower than price risk)
    field.sigma_rate = (volatility_bps / 2).min(MAX_RISK_SCALAR_BPS);
    
    // Leverage risk (higher than price risk)
    field.sigma_leverage = (volatility_bps * 3 / 2).min(MAX_RISK_SCALAR_BPS);
    
    Ok(())
}

// ============================================================================
// TWAP Window Constants
// ============================================================================

/// Default TWAP window for field updates
pub const DEFAULT_TWAP_WINDOW: i64 = 900; // 15 minutes

/// Minimum TWAP window
pub const MIN_TWAP_WINDOW: i64 = 300; // 5 minutes

/// Maximum TWAP window  
pub const MAX_TWAP_WINDOW: i64 = 3600; // 1 hour