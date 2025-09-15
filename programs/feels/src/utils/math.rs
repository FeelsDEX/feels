//! Math utilities for the protocol
//!
//! Contains liquidity calculations, safe math operations, and tick/price conversions.
//!
//! IMPORTANT: Rounding Direction Convention
//! ----------------------------------------
//! To protect the protocol from being drained by rounding errors:
//! 
//! 1. When calculating amount_out (output to user):
//!    - ALWAYS round DOWN (truncate)
//!    - This ensures users receive at most what they deserve
//!
//! 2. When calculating amount_in (input from user):
//!    - ALWAYS round UP (ceiling)
//!    - This ensures users pay at least what is required
//!
//! 3. When calculating fees:
//!    - ALWAYS round UP
//!    - This ensures the protocol collects at least the minimum fee
//!
//! The Orca Whirlpools core functions handle this correctly with the
//! `round_up` parameter in try_get_amount_delta_* functions.

use anchor_lang::prelude::*;
use crate::error::FeelsError;
use ethnum::U256;
use orca_whirlpools_core::{tick_index_to_sqrt_price, sqrt_price_to_tick_index, U128};
use crate::logic::SwapDirection;

/// Ceiling division for u64: (a * b + c - 1) / c
/// Always rounds UP to ensure minimum fee collection
pub fn mul_div_ceil_u64(a: u64, b: u64, c: u64) -> Result<u64> {
    if c == 0 {
        return Err(FeelsError::DivisionByZero.into());
    }
    
    // Check for overflow in a * b
    let product = (a as u128)
        .checked_mul(b as u128)
        .ok_or(FeelsError::MathOverflow)?;
    
    // Add (c - 1) for ceiling effect, then divide
    let result = product
        .checked_add(c as u128 - 1)
        .ok_or(FeelsError::MathOverflow)?
        .checked_div(c as u128)
        .ok_or(FeelsError::DivisionByZero)?;
    
    // Check if result fits in u64
    if result > u64::MAX as u128 {
        return Err(FeelsError::MathOverflow.into());
    }
    
    Ok(result as u64)
}

/// Calculate fee amount with ceiling rounding
/// Ensures minimum fee of 1 for any non-zero amount and fee rate
pub fn calculate_fee_ceil(amount: u64, fee_bps: u16) -> Result<u64> {
    if amount == 0 || fee_bps == 0 {
        return Ok(0);
    }
    
    // Calculate fee with ceiling: (amount * fee_bps + 9999) / 10000
    mul_div_ceil_u64(amount, fee_bps as u64, 10000)
}

/// Simple square root for u128 (Newton's method)
#[allow(dead_code)]
fn sqrt_u128(n: u128) -> Result<u128> {
    if n == 0 {
        return Ok(0);
    }
    
    let mut x = n;
    let mut y = (x + 1) / 2;
    
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    
    Ok(x)
}

/// Compute liquidity from amounts and price range
/// 
/// This implements the Uniswap V3 formula for computing liquidity
/// from token amounts given a price range.
/// 
/// NOTE: This function uses U256 for intermediate calculations to avoid
/// overflow when multiplying large sqrt prices. This is acceptable since
/// liquidity_from_amounts is NOT in the hot path (only called during
/// position opening, not during swaps). The hot paths (compute_swap_step
/// and delta calculations) use u128 exclusively to minimize BPF overhead.
pub fn liquidity_from_amounts(
    sqrt_p: u128,
    sqrt_pl: u128,
    sqrt_pu: u128,
    amount0: u64,
    amount1: u64,
) -> Result<u128> {
    require!(sqrt_pl < sqrt_pu, FeelsError::InvalidPrice);
    
    // If current price is below range, only amount0 matters
    if sqrt_p <= sqrt_pl {
        // L = amount0 * sqrt_pl * sqrt_pu / (sqrt_pu - sqrt_pl) / Q64
        let numerator = U256::from(amount0) * U256::from(sqrt_pl) * U256::from(sqrt_pu);
        let denominator = U256::from(sqrt_pu - sqrt_pl);
        let liquidity: U256 = (numerator / denominator) >> 64;
        return Ok(u128::try_from(liquidity.min(U256::from(u128::MAX))).unwrap_or(u128::MAX));
    }
    
    // If current price is above range, only amount1 matters
    if sqrt_p >= sqrt_pu {
        // L = amount1 * Q64 / (sqrt_pu - sqrt_pl)
        let numerator = U256::from(amount1) << 64;
        let denominator = U256::from(sqrt_pu - sqrt_pl);
        let liquidity: U256 = numerator / denominator;
        return Ok(u128::try_from(liquidity.min(U256::from(u128::MAX))).unwrap_or(u128::MAX));
    }
    
    // Current price is within range, calculate both L0 and L1 and take minimum
    
    // L0 = amount0 * sqrt_p * sqrt_pu / (sqrt_pu - sqrt_p) / Q64
    let l0 = if amount0 > 0 && sqrt_p < sqrt_pu {
        let numerator = U256::from(amount0) * U256::from(sqrt_p) * U256::from(sqrt_pu);
        let denominator = U256::from(sqrt_pu - sqrt_p);
        (numerator / denominator) >> 64
    } else {
        U256::MAX
    };
    
    // L1 = amount1 * Q64 / (sqrt_p - sqrt_pl)
    let l1 = if amount1 > 0 && sqrt_p > sqrt_pl {
        let numerator = U256::from(amount1) << 64;
        let denominator = U256::from(sqrt_p - sqrt_pl);
        numerator / denominator
    } else {
        U256::MAX
    };
    
    // Take minimum of L0 and L1
    let liquidity = l0.min(l1);
    Ok(u128::try_from(liquidity.min(U256::from(u128::MAX))).unwrap_or(u128::MAX))
}

/// Convert tick index to sqrt price with consistent error handling
pub fn sqrt_price_from_tick(tick: i32) -> Result<u128> {
    Ok(tick_index_to_sqrt_price(tick))
}

/// Convert sqrt price to tick index with consistent error handling
pub fn tick_from_sqrt_price(sqrt_price: u128) -> Result<i32> {
    Ok(sqrt_price_to_tick_index(U128::from(sqrt_price)))
}

/// Apply liquidity net when crossing a tick
/// 
/// Returns the new liquidity after applying the net change based on direction
pub fn apply_liquidity_net(
    direction: SwapDirection,
    current_liquidity: u128,
    liquidity_net: i128,
) -> Result<u128> {
    match direction {
        SwapDirection::ZeroForOne => {
            // Moving down - subtract liquidity when crossing from left to right
            if liquidity_net >= 0 {
                current_liquidity.checked_sub(liquidity_net as u128)
                    .ok_or(FeelsError::MathOverflow.into())
            } else {
                current_liquidity.checked_add((-liquidity_net) as u128)
                    .ok_or(FeelsError::MathOverflow.into())
            }
        }
        SwapDirection::OneForZero => {
            // Moving up - add liquidity when crossing from right to left
            if liquidity_net >= 0 {
                current_liquidity.checked_add(liquidity_net as u128)
                    .ok_or(FeelsError::MathOverflow.into())
            } else {
                current_liquidity.checked_sub((-liquidity_net) as u128)
                    .ok_or(FeelsError::MathOverflow.into())
            }
        }
    }
}

/// Add liquidity to the current liquidity (used when opening positions)
/// 
/// Ensures consistent overflow checking across the codebase
pub fn add_liquidity(
    current_liquidity: u128,
    liquidity_delta: u128,
) -> Result<u128> {
    current_liquidity.checked_add(liquidity_delta)
        .ok_or(FeelsError::MathOverflow.into())
}

/// Subtract liquidity from the current liquidity (used when closing positions)
/// 
/// Ensures consistent underflow checking across the codebase
pub fn subtract_liquidity(
    current_liquidity: u128,
    liquidity_delta: u128,
) -> Result<u128> {
    current_liquidity.checked_sub(liquidity_delta)
        .ok_or(FeelsError::MathOverflow.into())
}

/// Safe math operations with explicit rounding
pub mod safe {
    use super::*;
    
    pub fn add_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_add(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn sub_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_sub(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn mul_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_mul(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn div_u64(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        Ok(a / b)
    }
    
    /// Division with ceiling (round up) for u64
    /// Used when calculating amount_in or fees to favor the protocol
    pub fn div_ceil_u64(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        // ceiling division: (a + b - 1) / b
        let sum = a.checked_add(b - 1).ok_or(FeelsError::MathOverflow)?;
        Ok(sum / b)
    }
    
    /// Division with floor (round down) for u64  
    /// Used when calculating amount_out to favor the protocol
    pub fn div_floor_u64(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        Ok(a / b) // Standard division already floors
    }
    
    // u128 safe math operations for Buffer fee counters
    pub fn add_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_add(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn sub_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_sub(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn mul_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_mul(b).ok_or(FeelsError::MathOverflow.into())
    }
    
    pub fn div_u128(a: u128, b: u128) -> Result<u128> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        Ok(a / b)
    }
    
    /// Division with ceiling (round up) for u128
    /// Used when calculating amount_in or fees to favor the protocol
    pub fn div_ceil_u128(a: u128, b: u128) -> Result<u128> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        // ceiling division: (a + b - 1) / b
        let sum = a.checked_add(b - 1).ok_or(FeelsError::MathOverflow)?;
        Ok(sum / b)
    }
    
    /// Division with floor (round down) for u128
    /// Used when calculating amount_out to favor the protocol
    pub fn div_floor_u128(a: u128, b: u128) -> Result<u128> {
        if b == 0 {
            return Err(FeelsError::DivisionByZero.into());
        }
        Ok(a / b) // Standard division already floors
    }
    
    /// Calculate fee amount with ceiling (round up)
    /// Ensures protocol always collects at least the minimum fee
    pub fn calculate_fee_ceil(amount: u64, fee_bps: u16) -> Result<u64> {
        let fee_amount = (amount as u128)
            .checked_mul(fee_bps as u128)
            .ok_or(FeelsError::MathOverflow)?;
        div_ceil_u128(fee_amount, 10000).map(|v| v as u64)
    }
}

/// Calculate token output amount from sqrt price for initial buy
/// 
/// This calculates how many output tokens you get for a given input amount
/// at the specified sqrt price, accounting for decimal differences.
/// 
/// sqrt_price = sqrt(price) * 2^64 where price = token1/token0
pub fn calculate_token_out_from_sqrt_price(
    amount_in: u64,
    sqrt_price: u128,
    token_0_decimals: u8,
    token_1_decimals: u8,
    is_token_0_input: bool,
) -> Result<u64> {
    // Convert sqrt_price to actual price
    // price = (sqrt_price / 2^64)^2
    let sqrt_price_q64 = U256::from(sqrt_price);
    let q64 = U256::from(1u128 << 64);
    
    // Calculate price = (sqrt_price)^2 / 2^128
    let price_q128 = sqrt_price_q64 * sqrt_price_q64;
    
    // Adjust for decimal differences
    let decimal_adjustment = match token_0_decimals.cmp(&token_1_decimals) {
        std::cmp::Ordering::Greater => {
            U256::from(10u128.pow((token_0_decimals - token_1_decimals) as u32))
        }
        std::cmp::Ordering::Less => {
            U256::from(1u64) // Will divide by this factor below
        }
        std::cmp::Ordering::Equal => {
            U256::from(1u64)
        }
    };
    
    let amount_out = if is_token_0_input {
        // Buying token_1 with token_0
        // token1_amount = token0_amount * price * decimal_adjustment
        let amount_in_u256 = U256::from(amount_in);
        let numerator = amount_in_u256 * price_q128;
        let denominator = q64 * q64; // 2^128
        
        if token_1_decimals > token_0_decimals {
            let adj_factor = U256::from(10u128.pow((token_1_decimals - token_0_decimals) as u32));
            (numerator * adj_factor) / denominator
        } else {
            (numerator / decimal_adjustment) / denominator
        }
    } else {
        // Buying token_0 with token_1
        // token0_amount = token1_amount / price / decimal_adjustment
        let amount_in_u256 = U256::from(amount_in);
        let numerator = amount_in_u256 * q64 * q64; // amount * 2^128
        let denominator = price_q128;
        
        if token_0_decimals > token_1_decimals {
            (numerator / denominator) * decimal_adjustment
        } else {
            let adj_factor = U256::from(10u128.pow((token_1_decimals - token_0_decimals) as u32));
            (numerator / denominator) / adj_factor
        }
    };
    
    // Convert back to u64, ensuring no overflow
    Ok(u64::try_from(amount_out.min(U256::from(u64::MAX))).unwrap_or(u64::MAX))
}
