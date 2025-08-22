/// Low-level mathematical functions for concentrated liquidity amount calculations.
/// Converts between liquidity units and token amounts using sqrt price ratios.
/// Implements precise rounding logic to prevent value leakage and ensure LPs
/// always receive fair amounts when adding/removing liquidity or during swaps.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use crate::utils::constant::Q96;

// ============================================================================
// Core Implementation
// ============================================================================

/// These functions perform the core mathematical operations for concentrated liquidity.
/// They are used by the business logic layer in logic/liquidity_math.rs

/// Calculate amount0 delta for a liquidity change
pub fn get_amount_0_delta(
    sqrt_ratio_a: u128,
    sqrt_ratio_b: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u64> {
    if sqrt_ratio_a > sqrt_ratio_b {
        return get_amount_0_delta(sqrt_ratio_b, sqrt_ratio_a, liquidity, round_up);
    }

    let numerator1 = liquidity << 96;
    let numerator2 = sqrt_ratio_b - sqrt_ratio_a;

    require!(sqrt_ratio_a > 0, PoolError::DivisionByZero);

    // amount0 = liquidity * (sqrt_ratio_b - sqrt_ratio_a) / (sqrt_ratio_a * sqrt_ratio_b)
    let result = if round_up {
        // Ceiling division
        ((numerator1 * numerator2) / sqrt_ratio_a + sqrt_ratio_b - 1) / sqrt_ratio_b
    } else {
        // Floor division  
        (numerator1 * numerator2) / sqrt_ratio_a / sqrt_ratio_b
    };

    Ok(result.min(u64::MAX as u128) as u64)
}

/// Calculate amount1 delta for a liquidity change  
pub fn get_amount_1_delta(
    sqrt_ratio_a: u128,
    sqrt_ratio_b: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u64> {
    if sqrt_ratio_a > sqrt_ratio_b {
        return get_amount_1_delta(sqrt_ratio_b, sqrt_ratio_a, liquidity, round_up);
    }

    // amount1 = liquidity * (sqrt_ratio_b - sqrt_ratio_a)
    let result = if round_up {
        // Ceiling
        (liquidity * (sqrt_ratio_b - sqrt_ratio_a) + (Q96 - 1)) / Q96
    } else {
        // Floor
        liquidity * (sqrt_ratio_b - sqrt_ratio_a) / Q96
    };

    Ok(result.min(u64::MAX as u128) as u64)
}

/// Calculate liquidity for a given amount0
pub fn get_liquidity_for_amount_0(
    sqrt_ratio_a: u128,
    sqrt_ratio_b: u128,
    amount_0: u64,
) -> Result<u128> {
    if sqrt_ratio_a > sqrt_ratio_b {
        return get_liquidity_for_amount_0(sqrt_ratio_b, sqrt_ratio_a, amount_0);
    }

    let intermediate = (sqrt_ratio_a * sqrt_ratio_b) / Q96;
    Ok((amount_0 as u128 * intermediate) / (sqrt_ratio_b - sqrt_ratio_a))
}

/// Calculate liquidity for a given amount1
pub fn get_liquidity_for_amount_1(
    sqrt_ratio_a: u128,
    sqrt_ratio_b: u128,
    amount_1: u64,
) -> Result<u128> {
    if sqrt_ratio_a > sqrt_ratio_b {
        return get_liquidity_for_amount_1(sqrt_ratio_b, sqrt_ratio_a, amount_1);
    }

    Ok((amount_1 as u128 * Q96) / (sqrt_ratio_b - sqrt_ratio_a))
}

// ------------------------------------------------------------------------
// Helper Functions
// ------------------------------------------------------------------------

/// Get next sqrt price from amount 0 (rounding up)
pub fn get_next_sqrt_price_from_amount_0_rounding_up(
    sqrt_price_x96: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price_x96);
    }

    let numerator1 = (liquidity as u128) << 96;

    if add {
        let product = amount as u128 * sqrt_price_x96;
        if product / amount as u128 == sqrt_price_x96 {
            let denominator = numerator1 + product;
            if denominator >= numerator1 {
                return Ok(numerator1 / denominator);
            }
        }

        // Fallback to safer calculation
        Ok(numerator1 / (numerator1 / sqrt_price_x96 + amount as u128))
    } else {
        let product = amount as u128 * sqrt_price_x96;
        require!(product / amount as u128 == sqrt_price_x96, PoolError::MathOverflow);
        require!(numerator1 > product, PoolError::InsufficientLiquidity);
        
        let denominator = numerator1 - product;
        Ok(numerator1 / denominator)
    }
}

/// Get next sqrt price from amount 1 (rounding down)
pub fn get_next_sqrt_price_from_amount_1_rounding_down(
    sqrt_price_x96: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> Result<u128> {
    if add {
        let quotient = ((amount as u128) << 96) / liquidity;
        Ok(sqrt_price_x96 + quotient)
    } else {
        let quotient = ((amount as u128) << 96) / liquidity;
        require!(sqrt_price_x96 > quotient, PoolError::InsufficientLiquidity);
        Ok(sqrt_price_x96 - quotient)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_delta_calculations() {
        let sqrt_price_lower = Q96 / 2; // Price = 0.25
        let sqrt_price_upper = Q96 * 2; // Price = 4
        let liquidity = 1000;

        // Test amount0 delta
        let amount_0 = get_amount_0_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false).unwrap();
        assert!(amount_0 > 0);

        // Test amount1 delta
        let amount_1 = get_amount_1_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false).unwrap();
        assert!(amount_1 > 0);
    }

    #[test]
    fn test_liquidity_calculations() {
        let sqrt_price_a = Q96 / 2;
        let sqrt_price_b = Q96 * 2;
        let amount_0 = 1000;
        let amount_1 = 1000;

        // Test liquidity for amount0
        let liquidity_0 = get_liquidity_for_amount_0(sqrt_price_a, sqrt_price_b, amount_0).unwrap();
        assert!(liquidity_0 > 0);

        // Test liquidity for amount1
        let liquidity_1 = get_liquidity_for_amount_1(sqrt_price_a, sqrt_price_b, amount_1).unwrap();
        assert!(liquidity_1 > 0);
    }

    #[test]
    fn test_sqrt_price_calculations() {
        let sqrt_price = Q96;
        let liquidity = 1000;
        let amount = 100;

        // Test next sqrt price from amount 0
        let next_price_0 = get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount, true).unwrap();
        assert!(next_price_0 != sqrt_price);

        // Test next sqrt price from amount 1
        let next_price_1 = get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount, true).unwrap();
        assert!(next_price_1 != sqrt_price);
    }
}