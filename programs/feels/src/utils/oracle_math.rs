//! Oracle math utilities
//! 
//! Helper functions for working with GTWAP oracle data

use anchor_lang::prelude::*;
use crate::error::FeelsError;
use crate::utils::sqrt_price_from_tick;

/// Calculate the time-weighted average price from two observations
pub fn calculate_twap(
    tick_cumulative_0: i128,
    tick_cumulative_1: i128,
    timestamp_0: i64,
    timestamp_1: i64,
) -> Result<u128> {
    require!(
        timestamp_1 > timestamp_0,
        FeelsError::InvalidTimestamp
    );
    
    let time_delta = timestamp_1 - timestamp_0;
    let tick_delta = tick_cumulative_1 - tick_cumulative_0;
    
    // Average tick over the period
    let avg_tick = (tick_delta / time_delta as i128) as i32;
    
    // Convert to sqrt price  
    let sqrt_price = sqrt_price_from_tick(avg_tick)?;
    
    Ok(sqrt_price)
}

/// Calculate time-weighted average liquidity
pub fn calculate_twal(
    liquidity_cumulative_0: u128,
    liquidity_cumulative_1: u128,
    timestamp_0: i64,
    timestamp_1: i64,
) -> Result<u128> {
    require!(
        timestamp_1 > timestamp_0,
        FeelsError::InvalidTimestamp
    );
    
    let time_delta = (timestamp_1 - timestamp_0) as u128;
    let liquidity_delta = liquidity_cumulative_1.saturating_sub(liquidity_cumulative_0);
    
    // Average liquidity over the period
    let avg_liquidity = liquidity_delta / time_delta;
    
    Ok(avg_liquidity)
}

/// Calculate price volatility from oracle observations
/// Returns an approximation of price volatility as basis points
pub fn calculate_volatility(
    observations: &[(i64, i32)], // (timestamp, tick) pairs
) -> Result<u16> {
    if observations.len() < 2 {
        return Ok(0);
    }
    
    // Calculate tick changes between observations
    let mut sum_squared_changes: u128 = 0u128;
    let mut count = 0;
    
    for i in 1..observations.len() {
        let (prev_time, prev_tick) = observations[i - 1];
        let (curr_time, curr_tick) = observations[i];
        
        if curr_time > prev_time {
            let tick_change = (curr_tick - prev_tick).unsigned_abs();
            sum_squared_changes = sum_squared_changes.saturating_add((tick_change as u128) * (tick_change as u128));
            count += 1;
        }
    }
    
    if count == 0 {
        return Ok(0);
    }
    
    // Simple volatility approximation in basis points
    // This is a simplified calculation - in production you'd want
    // a more sophisticated volatility model
    let avg_squared_change = sum_squared_changes / count as u128;
    let volatility_bps = (avg_squared_change.min(10000)) as u16;
    
    Ok(volatility_bps)
}

/// Check if oracle data is stale
pub fn is_oracle_stale(
    last_update: i64,
    current_time: i64,
    max_age_seconds: i64,
) -> bool {
    current_time - last_update > max_age_seconds
}

/// Interpolate observation at exact timestamp
pub fn interpolate_observation(
    obs_before: (i64, i128, u128), // (timestamp, tick_cumulative, liquidity_cumulative)
    obs_after: (i64, i128, u128),
    target_timestamp: i64,
) -> Result<(i128, u128)> {
    let (time_0, tick_cum_0, liq_cum_0) = obs_before;
    let (time_1, tick_cum_1, liq_cum_1) = obs_after;
    
    require!(
        time_0 <= target_timestamp && target_timestamp <= time_1,
        FeelsError::InvalidTimestamp
    );
    
    if time_0 == time_1 {
        return Ok((tick_cum_0, liq_cum_0));
    }
    
    // Linear interpolation
    let total_time = time_1 - time_0;
    let elapsed_time = target_timestamp - time_0;
    let ratio = elapsed_time as u128 * 1_000_000 / total_time as u128; // Fixed point ratio
    
    let tick_cum_interpolated = tick_cum_0 + 
        ((tick_cum_1 - tick_cum_0) * ratio as i128 / 1_000_000);
    
    let liq_cum_interpolated = liq_cum_0 + 
        ((liq_cum_1 - liq_cum_0) * ratio / 1_000_000);
    
    Ok((tick_cum_interpolated, liq_cum_interpolated))
}