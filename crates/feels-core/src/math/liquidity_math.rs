//! # Liquidity Math
//! 
//! Calculations for concentrated liquidity positions, including amount0/amount1 deltas
//! and liquidity calculations from token amounts.

use crate::constants::Q64;
use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::big_int::{mul_div, U256, Rounding};

/// Calculate amount0 delta for Q64 precision
pub fn get_amount_0_delta(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    liquidity: u128,
    round_up: bool,
) -> CoreResult<u128> {
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        return get_amount_0_delta(sqrt_ratio_b_x64, sqrt_ratio_a_x64, liquidity, round_up);
    }

    let numerator1 = U256::from_u128(liquidity).mul(&U256::from_u128(1u128 << 64))
        .ok_or(FeelsCoreError::MathOverflow)?; // Q64 precision
    let numerator2 = U256::from_u128(sqrt_ratio_b_x64 - sqrt_ratio_a_x64);
    let denominator = U256::from_u128(sqrt_ratio_b_x64).mul(&U256::from_u128(sqrt_ratio_a_x64))
        .ok_or(FeelsCoreError::MathOverflow)?;

    let rounding = if round_up {
        Rounding::Up
    } else {
        Rounding::Down
    };

    let result = mul_div(numerator1, numerator2, denominator, rounding)?;

    result.to_u128().ok_or(FeelsCoreError::ConversionError)
}

/// Calculate amount1 delta for Q64 precision
pub fn get_amount_1_delta(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    liquidity: u128,
    round_up: bool,
) -> CoreResult<u128> {
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        return get_amount_1_delta(sqrt_ratio_b_x64, sqrt_ratio_a_x64, liquidity, round_up);
    }

    let rounding = if round_up {
        Rounding::Up
    } else {
        Rounding::Down
    };

    mul_div(
        U256::from_u128(liquidity),
        U256::from_u128(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
        U256::from_u128(Q64), // Q64 precision
        rounding,
    )
    .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
}

/// Get the next sqrt price from a given amount of token0
pub fn get_next_sqrt_price_from_amount_0_rounding_up(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> CoreResult<u128> {
    if amount == 0 {
        return Ok(sqrt_price_x64);
    }

    let numerator1 = U256::from_u128(sqrt_price_x64 as u128).mul(&U256::from_u128(liquidity as u128))
        .ok_or(FeelsCoreError::MathOverflow)?;
    
    if add {
        let product = U256::from_u128(amount as u128).mul(&U256::from_u128(sqrt_price_x64 as u128))
            .ok_or(FeelsCoreError::MathOverflow)?;
        let denominator = U256::from_u128(liquidity as u128).mul(&U256::from_u128(Q64 as u128))
            .ok_or(FeelsCoreError::MathOverflow)?
            .add(&product)
            .ok_or(FeelsCoreError::MathOverflow)?;
        
        mul_div(
            numerator1,
            U256::from_u128(Q64 as u128),
            denominator,
            Rounding::Up,
        )
        .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
    } else {
        let product = U256::from_u128(amount as u128).mul(&U256::from_u128(sqrt_price_x64 as u128))
            .ok_or(FeelsCoreError::MathOverflow)?;
        let q64_liquidity = U256::from_u128(liquidity as u128).mul(&U256::from_u128(Q64 as u128))
            .ok_or(FeelsCoreError::MathOverflow)?;
        
        if q64_liquidity <= product {
            return Err(FeelsCoreError::MathUnderflow);
        }
        
        let denominator = q64_liquidity.sub(&product)
            .ok_or(FeelsCoreError::MathUnderflow)?;
        
        mul_div(
            numerator1,
            U256::from_u128(Q64 as u128),
            denominator,
            Rounding::Up,
        )
        .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
    }
}

/// Get the next sqrt price from a given amount of token1
pub fn get_next_sqrt_price_from_amount_1_rounding_down(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount: u64,
    add: bool,
) -> CoreResult<u128> {
    if add {
        let quotient = mul_div(
            U256::from_u128(amount as u128),
            U256::from_u128(Q64 as u128),
            U256::from_u128(liquidity as u128),
            Rounding::Down,
        )?;

        let result = U256::from_u128(sqrt_price_x64 as u128).add(&quotient)
            .ok_or(FeelsCoreError::MathOverflow)?;
        result.to_u128().ok_or(FeelsCoreError::ConversionError)
    } else {
        let quotient = mul_div(
            U256::from_u128(amount as u128),
            U256::from_u128(Q64 as u128),
            U256::from_u128(liquidity as u128),
            Rounding::Up,
        )?;

        if U256::from_u128(sqrt_price_x64 as u128) < quotient {
            return Err(FeelsCoreError::MathUnderflow);
        }

        let result = U256::from_u128(sqrt_price_x64 as u128).sub(&quotient)
            .ok_or(FeelsCoreError::MathUnderflow)?;
        result.to_u128().ok_or(FeelsCoreError::ConversionError)
    }
}

/// Calculate liquidity for a given amount of token0
pub fn get_liquidity_for_amount_0(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    amount0: u64,
) -> CoreResult<u128> {
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        return get_liquidity_for_amount_0(sqrt_ratio_b_x64, sqrt_ratio_a_x64, amount0);
    }

    let intermediate = mul_div(
        U256::from_u128(sqrt_ratio_a_x64),
        U256::from_u128(sqrt_ratio_b_x64),
        U256::from_u128(Q64),
        Rounding::Down,
    )?;

    mul_div(
        U256::from_u128(amount0 as u128),
        intermediate,
        U256::from_u128(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
        Rounding::Down,
    )
    .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
}

/// Calculate liquidity for a given amount of token1
pub fn get_liquidity_for_amount_1(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    amount1: u64,
) -> CoreResult<u128> {
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        return get_liquidity_for_amount_1(sqrt_ratio_b_x64, sqrt_ratio_a_x64, amount1);
    }

    mul_div(
        U256::from_u128(amount1 as u128),
        U256::from_u128(Q64),
        U256::from_u128(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
        Rounding::Down,
    )
    .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
}

/// Calculate liquidity from amounts for a position
pub fn get_liquidity_for_amounts(
    sqrt_price_x64: u128,
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    amount0: u64,
    amount1: u64,
) -> CoreResult<u128> {
    let (sqrt_ratio_a_x64, sqrt_ratio_b_x64) = if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        (sqrt_ratio_b_x64, sqrt_ratio_a_x64)
    } else {
        (sqrt_ratio_a_x64, sqrt_ratio_b_x64)
    };

    if sqrt_price_x64 <= sqrt_ratio_a_x64 {
        get_liquidity_for_amount_0(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount0)
    } else if sqrt_price_x64 < sqrt_ratio_b_x64 {
        let liquidity0 = get_liquidity_for_amount_0(sqrt_price_x64, sqrt_ratio_b_x64, amount0)?;
        let liquidity1 = get_liquidity_for_amount_1(sqrt_ratio_a_x64, sqrt_price_x64, amount1)?;
        
        Ok(liquidity0.min(liquidity1))
    } else {
        get_liquidity_for_amount_1(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount1)
    }
}

/// Get amounts from liquidity for a position
pub fn get_amounts_for_liquidity(
    sqrt_price_x64: u128,
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    liquidity: u128,
) -> CoreResult<(u64, u64)> {
    let (sqrt_ratio_a_x64, sqrt_ratio_b_x64) = if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        (sqrt_ratio_b_x64, sqrt_ratio_a_x64)
    } else {
        (sqrt_ratio_a_x64, sqrt_ratio_b_x64)
    };

    let (amount0, amount1) = if sqrt_price_x64 <= sqrt_ratio_a_x64 {
        (
            get_amount_0_delta(sqrt_ratio_a_x64, sqrt_ratio_b_x64, liquidity, false)? as u64,
            0,
        )
    } else if sqrt_price_x64 < sqrt_ratio_b_x64 {
        (
            get_amount_0_delta(sqrt_price_x64, sqrt_ratio_b_x64, liquidity, false)? as u64,
            get_amount_1_delta(sqrt_ratio_a_x64, sqrt_price_x64, liquidity, false)? as u64,
        )
    } else {
        (
            0,
            get_amount_1_delta(sqrt_ratio_a_x64, sqrt_ratio_b_x64, liquidity, false)? as u64,
        )
    };

    Ok((amount0, amount1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_deltas() {
        let sqrt_price_lower = Q64; // 1.0 in Q64
        let sqrt_price_upper = Q64 + (Q64 / 100); // 1.01 in Q64
        let liquidity = 1000u128;

        // Test amount0 delta
        let amount0 = get_amount_0_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false).unwrap();
        assert!(amount0 > 0);

        // Test amount1 delta
        let amount1 = get_amount_1_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false).unwrap();
        assert!(amount1 > 0);
    }

    #[test]
    fn test_liquidity_calculations() {
        let sqrt_price_lower = Q64;
        let sqrt_price_upper = Q64 + (Q64 / 100);
        let amount0 = 1000u64;
        let amount1 = 1000u64;

        // Test liquidity from amount0
        let liquidity0 = get_liquidity_for_amount_0(sqrt_price_lower, sqrt_price_upper, amount0).unwrap();
        assert!(liquidity0 > 0);

        // Test liquidity from amount1
        let liquidity1 = get_liquidity_for_amount_1(sqrt_price_lower, sqrt_price_upper, amount1).unwrap();
        assert!(liquidity1 > 0);
    }

    #[test]
    fn test_next_sqrt_price() {
        let sqrt_price = Q64;
        let liquidity = 1000u128;
        let amount = 100u64;

        // Test adding amount0
        let next_price = get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount, true).unwrap();
        assert!(next_price < sqrt_price); // Adding token0 decreases sqrt price

        // Test adding amount1
        let next_price = get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount, true).unwrap();
        assert!(next_price > sqrt_price); // Adding token1 increases sqrt price
    }
}