/// Field update logic for the market field commitment strategy.
/// Handles updates to market scalars and risk parameters.
use anchor_lang::prelude::*;
use crate::state::{MarketField, TwapOracle, Pool};
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
    pool: &Pool,
    twap_oracle: &TwapOracle,
    current_time: i64,
) -> Result<()> {
    // Check update frequency
    require!(
        current_time - field.snapshot_ts >= MIN_UPDATE_INTERVAL,
        FeelsProtocolError::UpdateTooFrequent {
            min_interval: MIN_UPDATE_INTERVAL,
            elapsed: current_time - field.snapshot_ts,
        }
    );
    
    // Get current TWAPs
    let twap_window = DEFAULT_TWAP_WINDOW.min(field.max_staleness);
    let current_price = twap_oracle.get_twap(twap_window, current_time)?;
    
    // For two-token pools, calculate individual TWAPs
    // In production, would use token-specific oracles
    let (twap_a, twap_b) = calculate_token_twaps(current_price, pool)?;
    
    // Calculate new market scalars
    let new_scalars = calculate_market_scalars(pool, twap_a, twap_b)?;
    
    // Validate scalar changes are within bounds
    validate_scalar_changes(field, &new_scalars)?;
    
    // Update field data
    field.S = new_scalars.S;
    field.T = new_scalars.T;
    field.L = new_scalars.L;
    field.twap_a = twap_a;
    field.twap_b = twap_b;
    field.snapshot_ts = current_time;
    
    // Validate updated field
    field.validate()?;
    
    Ok(())
}

/// Calculate token-specific TWAPs from pool price
fn calculate_token_twaps(pool_price: u128, pool: &Pool) -> Result<(u128, u128)> {
    // For simplicity, assume pool price is token_a/token_b
    // and token_b is the numeraire (value = 1)
    let twap_b = 1u128 << 64; // Q64 fixed point
    let twap_a = pool_price;
    
    Ok((twap_a, twap_b))
}

/// Market scalars calculated from pool state
#[derive(Debug)]
struct MarketScalars {
    pub S: u128,
    pub T: u128,
    pub L: u128,
}

/// Calculate market scalars from pool state
fn calculate_market_scalars(
    pool: &Pool,
    twap_a: u128,
    twap_b: u128,
) -> Result<MarketScalars> {
    // Calculate spot scalar S
    // S = (x_a * p_a)^ω_a * (x_b * p_b)^ω_b / sqrt(1 + σ_price²)
    let S = calculate_spot_scalar(pool, twap_a, twap_b)?;
    
    // Calculate time scalar T
    // For now, use placeholder based on pool liquidity
    let T = calculate_time_scalar(pool)?;
    
    // Calculate leverage scalar L
    // For now, use placeholder
    let L = calculate_leverage_scalar(pool)?;
    
    Ok(MarketScalars { S, T, L })
}

/// Calculate spot dimension scalar
fn calculate_spot_scalar(
    pool: &Pool,
    twap_a: u128,
    twap_b: u128,
) -> Result<u128> {
    // Get token balances from pool
    // In production, would query actual vault balances
    let balance_a = estimate_token_balance_a(pool)?;
    let balance_b = estimate_token_balance_b(pool)?;
    
    // Calculate numeraire values
    let value_a = balance_a
        .checked_mul(twap_a)
        .ok_or(FeelsProtocolError::MathOverflow)?
        >> 64; // Adjust for Q64
        
    let value_b = balance_b
        .checked_mul(twap_b)
        .ok_or(FeelsProtocolError::MathOverflow)?
        >> 64;
    
    // For equal weights, use geometric mean
    let spot_value = crate::utils::math::sqrt_u128(
        value_a
            .checked_mul(value_b)
            .ok_or(FeelsProtocolError::MathOverflow)?
    )?;
    
    // Apply risk scaling (simplified - no risk for now)
    Ok(spot_value)
}

/// Calculate time dimension scalar
fn calculate_time_scalar(pool: &Pool) -> Result<u128> {
    // Placeholder: use pool liquidity as proxy
    // In production, would track actual lending/borrowing volumes
    Ok(pool.liquidity.max(1u128 << 64))
}

/// Calculate leverage dimension scalar
fn calculate_leverage_scalar(pool: &Pool) -> Result<u128> {
    // Placeholder: use constant
    // In production, would track actual leverage positions
    Ok(1u128 << 64)
}

/// Estimate token A balance from pool state
fn estimate_token_balance_a(pool: &Pool) -> Result<u128> {
    // Simplified: use liquidity and price
    // L = sqrt(x * y) at current price
    // x = L² / y, y = L² / x
    
    let sqrt_price = pool.current_sqrt_rate;
    if sqrt_price == 0 {
        return Ok(0);
    }
    
    // x = L / sqrt_price (in token units)
    let balance = (pool.liquidity << 96) / sqrt_price;
    Ok(balance)
}

/// Estimate token B balance from pool state
fn estimate_token_balance_b(pool: &Pool) -> Result<u128> {
    // y = L * sqrt_price (in token units)
    let balance = (pool.liquidity * pool.current_sqrt_rate) >> 96;
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
        FeelsProtocolError::ExcessiveChange {
            field: "S".to_string(),
            change_bps: s_change_bps,
            max_bps: MAX_SCALAR_CHANGE_BPS,
        }
    );
    
    // Check T change
    let t_change_bps = calculate_change_bps(current.T, new.T);
    require!(
        t_change_bps <= MAX_SCALAR_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange {
            field: "T".to_string(),
            change_bps: t_change_bps,
            max_bps: MAX_SCALAR_CHANGE_BPS,
        }
    );
    
    // Check L change
    let l_change_bps = calculate_change_bps(current.L, new.L);
    require!(
        l_change_bps <= MAX_SCALAR_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange {
            field: "L".to_string(),
            change_bps: l_change_bps,
            max_bps: MAX_SCALAR_CHANGE_BPS,
        }
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