/// Provides 256-bit unsigned integer arithmetic for high-precision calculations.
/// Essential for fee growth tracking where cumulative values exceed u128 range.
/// Implements efficient operations using the ruint library while maintaining
/// compatibility with Solana's compute constraints and ensuring no precision loss.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use super::math_big_int::{U256, Rounding};

/// This module provides production-grade arithmetic operations following
/// patterns from established Solana DEX protocols (Meteora, Orca, Raydium).
// ============================================================================
// Core Conversion Functions
// ============================================================================
/// Convert u128 to U256 representation
pub fn u128_to_u256(value: u128) -> [u64; 4] {
    let u256_val = U256::from_u128(value);
    u256_val.words
}

// ============================================================================
// Fee Growth Calculations
// ============================================================================

/// High-precision fee growth calculation: (fee_amount * 2^128) / liquidity
/// 
/// Key operation for concentrated liquidity fee distribution.
/// Uses overflow-safe arithmetic with proper rounding for accuracy.
pub fn calculate_fee_growth_delta(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
    if liquidity == 0 {
        return Err(PoolError::DivisionByZero.into());
    }
    
    // Convert inputs to U256 for high-precision arithmetic
    let fee_u256 = U256::from_u64(fee_amount);
    let liquidity_u256 = U256::from_u128(liquidity);
    
    // Multiply fee by 2^128 (left shift by 128 bits)
    let fee_shifted = fee_u256.checked_shl(128)
        .ok_or(PoolError::MathOverflow)?;
    
    // Divide by liquidity with floor rounding (standard for fee growth)
    let result = fee_shifted.mul_div(&U256::ONE, &liquidity_u256, Rounding::Down)
        .ok_or(PoolError::MathOverflow)?;
    
    Ok(result.words)
}

// ============================================================================
// Basic Arithmetic Operations
// ============================================================================

/// Add two U256 values with overflow checking
pub fn add_u256(a: [u64; 4], b: [u64; 4]) -> Result<[u64; 4]> {
    let a_u256 = U256 { words: a };
    let b_u256 = U256 { words: b };
    
    // Use checked addition - return error on overflow
    match a_u256.checked_add(&b_u256) {
        Some(result) => Ok(result.words),
        None => Err(PoolError::MathOverflow.into()), // Return error on overflow
    }
}

/// Subtract two U256 values with underflow checking
pub fn sub_u256(a: [u64; 4], b: [u64; 4]) -> Result<[u64; 4]> {
    let a_u256 = U256 { words: a };
    let b_u256 = U256 { words: b };
    
    // Use checked subtraction - return error on underflow
    match a_u256.checked_sub(&b_u256) {
        Some(result) => Ok(result.words),
        None => Err(PoolError::ArithmeticUnderflow.into()), // Return error on underflow
    }
}

/// Compare two U256 values
pub fn cmp_u256(a: [u64; 4], b: [u64; 4]) -> std::cmp::Ordering {
    let a_u256 = U256 { words: a };
    let b_u256 = U256 { words: b };
    a_u256.cmp(&b_u256)
}

// ============================================================================
// High-Precision Operations
// ============================================================================

/// High-precision multiply-divide operation for price calculations
/// 
/// This is required for accurate price computations in concentrated liquidity.
/// Pattern: (amount * price_numerator) / price_denominator
pub fn mul_div_u256(
    amount: u128,
    numerator: u128, 
    denominator: u128,
    round_up: bool
) -> Result<u128> {
    let rounding = if round_up { Rounding::Up } else { Rounding::Down };
    
    let amount_u256 = U256::from_u128(amount);
    let numerator_u256 = U256::from_u128(numerator);
    let denominator_u256 = U256::from_u128(denominator);
    
    let result = amount_u256.mul_div(&numerator_u256, &denominator_u256, rounding)
        .ok_or(PoolError::MathOverflow)?;
    
    result.to_u128().ok_or(PoolError::MathOverflow.into())
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert U256 array back to u128 if possible
pub fn u256_to_u128(value: [u64; 4]) -> Option<u128> {
    let u256_val = U256 { words: value };
    u256_val.to_u128()
}

/// Check if U256 value is zero
pub fn is_u256_zero(value: [u64; 4]) -> bool {
    let u256_val = U256 { words: value };
    u256_val.is_zero()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_growth_calculation() {
        // Test fee growth: (1000 * 2^128) / 1000000 
        let result = calculate_fee_growth_delta(1000, 1000000).unwrap();
        
        // Convert back to check
        let result_u256 = U256 { words: result };
        assert!(result_u256.to_u128().is_some());
    }
    
    #[test]
    fn test_u256_operations() {
        let a = u128_to_u256(100);
        let b = u128_to_u256(50);
        
        // Test addition
        let sum = add_u256(a, b).unwrap();
        assert_eq!(u256_to_u128(sum).unwrap(), 150);
        
        // Test subtraction
        let diff = sub_u256(a, b).unwrap();
        assert_eq!(u256_to_u128(diff).unwrap(), 50);
        
        // Test comparison
        assert_eq!(cmp_u256(a, b), std::cmp::Ordering::Greater);
    }
    
    #[test]
    fn test_mul_div_precision() {
        // Test high-precision multiply-divide
        let result = mul_div_u256(1000, 3, 2, false).unwrap();
        assert_eq!(result, 1500); // (1000 * 3) / 2 = 1500
        
        // Test rounding
        let result_floor = mul_div_u256(10, 3, 2, false).unwrap();
        let result_ceil = mul_div_u256(10, 3, 2, true).unwrap();
        assert_eq!(result_floor, 15); // Floor of 15
        assert_eq!(result_ceil, 15);  // Ceiling of 15 (exact division)
    }
    
    #[test]
    fn test_overflow_safety() {
        // Test that operations handle overflow by returning errors
        let max_val = U256::MAX.words;
        let one = u128_to_u256(1);
        
        // Addition overflow should return error
        let result = add_u256(max_val, one);
        assert!(result.is_err());
        
        // Subtraction underflow should return error
        let zero = U256::ZERO.words;
        let result = sub_u256(zero, one);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_division_by_zero_safety() {
        // Should return error, not panic
        let result = calculate_fee_growth_delta(1000, 0);
        assert!(result.is_err());
    }
}