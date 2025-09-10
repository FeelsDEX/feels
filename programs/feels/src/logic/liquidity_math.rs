//! Liquidity math functions for AMM calculations
//!
//! Core mathematical functions for converting between
//! liquidity, amounts, and prices

use crate::error::FeelsError;
use orca_whirlpools_core::{
    U128,
    try_get_amount_delta_a, try_get_amount_delta_b,
};

/// Calculate amounts from liquidity (for removing liquidity)
pub fn amounts_from_liquidity(
    sqrt_price: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    liquidity: u128,
) -> Result<(u64, u64), FeelsError> {
    // Handle degenerate cases
    if liquidity == 0 {
        return Ok((0, 0));
    }
    if sqrt_price_lower >= sqrt_price_upper {
        return Err(FeelsError::InvalidPrice);
    }

    // If current price is below the range: fully in token0
    if sqrt_price <= sqrt_price_lower {
        let a0 = amount0_delta(liquidity, sqrt_price_lower, sqrt_price_upper)?;
        return Ok((a0, 0));
    }
    // If current price is above the range: fully in token1
    if sqrt_price >= sqrt_price_upper {
        let a1 = amount1_delta(liquidity, sqrt_price_lower, sqrt_price_upper)?;
        return Ok((0, a1));
    }
    // In-range: split
    let a0 = amount0_delta(liquidity, sqrt_price, sqrt_price_upper)?;
    let a1 = amount1_delta(liquidity, sqrt_price_lower, sqrt_price)?;
    Ok((a0, a1))
}

/// Calculate amount0 delta using Orca core
pub fn amount0_delta(liquidity: u128, sqrt_price_a: u128, sqrt_price_b: u128) -> Result<u64, FeelsError> {
    try_get_amount_delta_a(
        U128::from(sqrt_price_a.min(sqrt_price_b)),
        U128::from(sqrt_price_a.max(sqrt_price_b)),
        U128::from(liquidity),
        false
    ).map_err(|_| FeelsError::MathOverflow)
}

/// Calculate amount1 delta using Orca core  
pub fn amount1_delta(liquidity: u128, sqrt_price_a: u128, sqrt_price_b: u128) -> Result<u64, FeelsError> {
    try_get_amount_delta_b(
        U128::from(sqrt_price_a.min(sqrt_price_b)),
        U128::from(sqrt_price_a.max(sqrt_price_b)),
        U128::from(liquidity),
        false
    ).map_err(|_| FeelsError::MathOverflow)
}