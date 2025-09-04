//! # Fee Math
//! 
//! Fee growth calculations using Q64 precision for tick-based accounting.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::big_int::{U256, u256_to_words};

/// Calculate fee growth using native Q64 precision
pub fn calculate_fee_growth_q64(fee_amount: u64, liquidity: u128) -> CoreResult<[u64; 4]> {
    if liquidity == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }

    // Use Q64 precision for fee calculations
    // Multiply by 2^64 instead of shifting
    let fee_u256 = U256::from_u128(fee_amount as u128);
    let q64 = U256::from_u128(1u128 << 64);
    let fee_shifted = fee_u256.mul(&q64)
        .ok_or(FeelsCoreError::MathOverflow)?;
    let result = fee_shifted.div(&U256::from_u128(liquidity as u128))
        .ok_or(FeelsCoreError::DivisionByZero)?;

    let (lo, hi) = u256_to_words(result);
    // Convert to [u64; 4] format
    Ok([
        lo as u64,
        (lo >> 64) as u64,
        hi as u64,
        (hi >> 64) as u64,
    ])
}

/// Convert [u64; 4] fee growth to u128 (using lower 128 bits)
pub fn words_to_u128(words: [u64; 4]) -> u128 {
    // Use lower 128 bits (first two u64s)
    (words[1] as u128) << 64 | (words[0] as u128)
}

/// Convert u128 to [u64; 4] fee growth
pub fn u128_to_words(value: u128) -> [u64; 4] {
    [
        value as u64,
        (value >> 64) as u64,
        0,
        0,
    ]
}

/// Subtract fee growth values with overflow handling (u128 version)
pub fn sub_fee_growth(a: u128, b: u128) -> u128 {
    // Fee growth can wrap around, so we need to handle underflow properly
    a.wrapping_sub(b)
}

/// Subtract fee growth values with overflow handling ([u64; 4] version)
pub fn sub_fee_growth_words(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let a_u128 = words_to_u128(a);
    let b_u128 = words_to_u128(b);
    let result = sub_fee_growth(a_u128, b_u128);
    u128_to_words(result)
}

/// Calculate position fee growth inside a tick range
pub fn calculate_position_fee_growth_inside(
    tick_lower: i32,
    tick_upper: i32,
    tick_current: i32,
    fee_growth_global: [u64; 4],
    fee_growth_outside_lower: [u64; 4],
    fee_growth_outside_upper: [u64; 4],
) -> [u64; 4] {
    // Calculate fee growth below lower tick
    let fee_growth_below = if tick_current >= tick_lower {
        fee_growth_outside_lower
    } else {
        sub_fee_growth_words(fee_growth_global, fee_growth_outside_lower)
    };

    // Calculate fee growth above upper tick
    let fee_growth_above = if tick_current < tick_upper {
        fee_growth_outside_upper
    } else {
        sub_fee_growth_words(fee_growth_global, fee_growth_outside_upper)
    };

    // Fee growth inside = global - below - above
    let fee_growth_inside = sub_fee_growth_words(
        sub_fee_growth_words(fee_growth_global, fee_growth_below),
        fee_growth_above
    );

    fee_growth_inside
}

/// Calculate fees owed to a position
pub fn calculate_fees_owed(
    liquidity: u128,
    fee_growth_inside_last: [u64; 4],
    fee_growth_inside_current: [u64; 4],
) -> CoreResult<u64> {
    // Calculate the delta in fee growth
    let fee_growth_delta = sub_fee_growth_words(fee_growth_inside_current, fee_growth_inside_last);
    let fee_growth_delta_u128 = words_to_u128(fee_growth_delta);

    // Fees = liquidity * fee_growth_delta / Q64
    let fees_u256 = U256::from_u128(liquidity).mul(&U256::from_u128(fee_growth_delta_u128))
        .ok_or(FeelsCoreError::MathOverflow)?;
    let q64_u256 = U256::from_u128(1u128 << 64);
    let result = fees_u256.div(&q64_u256)
        .ok_or(FeelsCoreError::DivisionByZero)?;

    result.to_u64().ok_or(FeelsCoreError::ConversionError)
}

/// Backwards compatibility function for updating position fees
pub fn calculate_position_fees_owed(
    liquidity: u128,
    fee_growth_inside_0: [u64; 4],
    fee_growth_inside_1: [u64; 4],
    fee_growth_inside_last_0: [u64; 4],
    fee_growth_inside_last_1: [u64; 4],
) -> CoreResult<(u64, u64)> {
    let tokens_owed_0 = calculate_fees_owed(
        liquidity,
        fee_growth_inside_last_0,
        fee_growth_inside_0,
    )?;
    
    let tokens_owed_1 = calculate_fees_owed(
        liquidity,
        fee_growth_inside_last_1,
        fee_growth_inside_1,
    )?;
    
    Ok((tokens_owed_0, tokens_owed_1))
}

// Aliases for backwards compatibility
pub use words_to_u128 as fee_growth_words_to_u128;
pub use u128_to_words as fee_growth_u128_to_words;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_growth_calculation() {
        // Test basic fee growth
        let fee_amount = 1000u64;
        let liquidity = 10000u128;
        
        let fee_growth = calculate_fee_growth_q64(fee_amount, liquidity).unwrap();
        let fee_growth_u128 = words_to_u128(fee_growth);
        
        // Fee growth should be (1000 * 2^64) / 10000
        let expected = ((fee_amount as u128) << 64) / liquidity;
        assert_eq!(fee_growth_u128, expected);
    }

    #[test]
    fn test_fee_growth_wrapping() {
        // Test that fee growth subtraction handles wrapping correctly
        let a = 100u128;
        let b = 200u128;
        
        let result = sub_fee_growth(a, b);
        assert_eq!(result, a.wrapping_sub(b));
    }

    #[test]
    fn test_position_fee_growth() {
        let tick_lower = 0;
        let tick_upper = 100;
        let tick_current = 50;
        
        let fee_growth_global = u128_to_words(1000);
        let fee_growth_outside_lower = u128_to_words(100);
        let fee_growth_outside_upper = u128_to_words(200);
        
        let fee_growth_inside = calculate_position_fee_growth_inside(
            tick_lower,
            tick_upper,
            tick_current,
            fee_growth_global,
            fee_growth_outside_lower,
            fee_growth_outside_upper,
        );
        
        // When current tick is inside range:
        // fee_growth_inside = global - outside_lower - outside_upper
        let expected = 1000 - 100 - 200;
        assert_eq!(words_to_u128(fee_growth_inside), expected);
    }

    #[test]
    fn test_fees_owed() {
        let liquidity = 1000u128;
        let fee_growth_last = u128_to_words(100);
        let fee_growth_current = u128_to_words(150);
        
        let fees = calculate_fees_owed(liquidity, fee_growth_last, fee_growth_current).unwrap();
        
        // Delta = 50, fees = 1000 * 50 / 2^64
        assert!(fees > 0);
    }
}