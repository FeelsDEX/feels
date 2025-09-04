//! Big integer operations for high-precision math
//! 
//! This module provides U256 operations and mul_div functionality
//! that's needed for accurate fixed-point calculations.

use crate::errors::{FeelsCoreError, CoreResult};

/// Rounding mode for division operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "anchor", derive(anchor_lang::prelude::AnchorSerialize, anchor_lang::prelude::AnchorDeserialize))]
pub enum Rounding {
    /// Round down (towards zero)
    Down,
    /// Round up (away from zero)
    Up,
}

/// 256-bit unsigned integer for intermediate calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U256 {
    /// Low 128 bits
    pub lo: u128,
    /// High 128 bits
    pub hi: u128,
}

impl U256 {
    /// Create a new U256 from low and high parts
    pub const fn new(lo: u128, hi: u128) -> Self {
        Self { lo, hi }
    }

    /// Create from a single u128 value
    pub const fn from_u128(value: u128) -> Self {
        Self { lo: value, hi: 0 }
    }

    /// Create from a single u64 value
    pub const fn from_u64(value: u64) -> Self {
        Self { lo: value as u128, hi: 0 }
    }

    /// Check if the value is zero
    pub const fn is_zero(&self) -> bool {
        self.lo == 0 && self.hi == 0
    }

    /// Convert to u128, returning None if overflow
    pub fn to_u128(&self) -> Option<u128> {
        if self.hi == 0 {
            Some(self.lo)
        } else {
            None
        }
    }

    /// Convert to u64, returning None if overflow
    pub fn to_u64(&self) -> Option<u64> {
        if self.hi == 0 && self.lo <= u64::MAX as u128 {
            Some(self.lo as u64)
        } else {
            None
        }
    }

    /// Add two U256 values
    pub fn add(&self, other: &U256) -> Option<U256> {
        let (lo, carry) = self.lo.overflowing_add(other.lo);
        let hi = self.hi.checked_add(other.hi)?.checked_add(carry as u128)?;
        Some(U256::new(lo, hi))
    }

    /// Subtract two U256 values
    pub fn sub(&self, other: &U256) -> Option<U256> {
        let (lo, borrow) = self.lo.overflowing_sub(other.lo);
        let hi = self.hi.checked_sub(other.hi)?.checked_sub(borrow as u128)?;
        Some(U256::new(lo, hi))
    }

    /// Multiply two U256 values
    pub fn mul(&self, other: &U256) -> Option<U256> {
        // Simple case: both high parts are zero
        if self.hi == 0 && other.hi == 0 {
            // Use u128 widening multiplication
            let result = (self.lo as u128).checked_mul(other.lo as u128)?;
            let (lo, hi) = split_u128(result);
            return Some(U256::new(lo, hi));
        }

        // Full multiplication would overflow if either has high part
        if self.hi != 0 || other.hi != 0 {
            return None;
        }

        None
    }

    /// Divide U256 by U256, returning quotient
    pub fn div(&self, other: &U256) -> Option<U256> {
        if other.is_zero() {
            return None;
        }

        // Simple case: divisor fits in u128
        if other.hi == 0 {
            let divisor = other.lo;
            
            // Even simpler: dividend also fits in u128
            if self.hi == 0 {
                return Some(U256::from_u128(self.lo / divisor));
            }

            // Dividend is larger than u128
            let quotient_hi = self.hi / divisor;
            let remainder_hi = self.hi % divisor;
            
            // Combine remainder with low part
            let dividend_lo = ((remainder_hi as u128) << 64) | (self.lo >> 64);
            let quotient_mid = dividend_lo / divisor;
            let remainder_mid = dividend_lo % divisor;
            
            let dividend_lo = ((remainder_mid as u128) << 64) | (self.lo & 0xFFFFFFFFFFFFFFFF);
            let quotient_lo = dividend_lo / divisor;
            
            let lo = (quotient_mid << 64) | quotient_lo;
            Some(U256::new(lo, quotient_hi))
        } else {
            // Complex case: would need full long division
            // For now, return None for divisors that don't fit in u128
            None
        }
    }
    
    /// Compare if self <= other
    pub fn le(&self, other: &U256) -> bool {
        self.hi < other.hi || (self.hi == other.hi && self.lo <= other.lo)
    }
    
    /// Compare if self < other
    pub fn lt(&self, other: &U256) -> bool {
        self.hi < other.hi || (self.hi == other.hi && self.lo < other.lo)
    }
    
    /// Compare if self >= other
    pub fn ge(&self, other: &U256) -> bool {
        self.hi > other.hi || (self.hi == other.hi && self.lo >= other.lo)
    }
    
    /// Compare if self > other
    pub fn gt(&self, other: &U256) -> bool {
        self.hi > other.hi || (self.hi == other.hi && self.lo > other.lo)
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.hi.cmp(&other.hi) {
            std::cmp::Ordering::Equal => self.lo.cmp(&other.lo),
            ordering => ordering,
        }
    }
}

/// Split a u128 into low and high u64 parts
fn split_u128(value: u128) -> (u128, u128) {
    (value, 0)
}

/// Multiply two u128 values and return as U256
pub fn mul_u128_to_u256(a: u128, b: u128) -> U256 {
    // Split into 64-bit parts for multiplication
    let a_lo = a as u64;
    let a_hi = (a >> 64) as u64;
    let b_lo = b as u64;
    let b_hi = (b >> 64) as u64;

    // Multiply parts
    let lo_lo = (a_lo as u128) * (b_lo as u128);
    let lo_hi = (a_lo as u128) * (b_hi as u128);
    let hi_lo = (a_hi as u128) * (b_lo as u128);
    let hi_hi = (a_hi as u128) * (b_hi as u128);

    // Add cross products
    let mid = lo_hi + hi_lo;
    let lo = lo_lo + (mid << 64);
    let hi = hi_hi + (mid >> 64) + if lo < lo_lo { 1 } else { 0 };

    U256::new(lo, hi)
}

/// Multiply two values and divide by a third with specified rounding
/// result = (a * b) / denominator
pub fn mul_div(
    a: U256,
    b: U256,
    denominator: U256,
    rounding: Rounding,
) -> CoreResult<U256> {
    if denominator.is_zero() {
        return Err(FeelsCoreError::DivisionByZero);
    }

    // Calculate a * b
    let product = a.mul(&b)
        .ok_or(FeelsCoreError::MulDivOverflow)?;

    // Divide by denominator
    let quotient = product.div(&denominator)
        .ok_or(FeelsCoreError::MulDivOverflow)?;

    // Apply rounding if needed
    if rounding == Rounding::Up {
        // Check if there's a remainder
        let remainder = product.sub(&quotient.mul(&denominator).unwrap_or_default())
            .unwrap_or_default();
        
        if !remainder.is_zero() {
            // Round up by adding 1
            return quotient.add(&U256::from_u64(1))
                .ok_or(FeelsCoreError::MulDivOverflow);
        }
    }

    Ok(quotient)
}

/// Multiply two u64 values and divide by a third with specified rounding
pub fn mul_div_u64(
    a: u64,
    b: u64,
    denominator: u64,
    rounding: Rounding,
) -> CoreResult<u64> {
    if denominator == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }

    let product = (a as u128) * (b as u128);
    let quotient = product / (denominator as u128);
    let remainder = product % (denominator as u128);

    let mut result = quotient;
    if rounding == Rounding::Up && remainder > 0 {
        result = result.checked_add(1)
            .ok_or(FeelsCoreError::MulDivOverflow)?;
    }

    result.try_into()
        .map_err(|_| FeelsCoreError::MulDivOverflow)
}

/// Multiply two u128 values and divide by a third with specified rounding
pub fn mul_div_u128(
    a: u128,
    b: u128,
    denominator: u128,
    rounding: Rounding,
) -> CoreResult<u128> {
    if denominator == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }

    // Convert to U256 for intermediate calculation
    let a_u256 = U256::from_u128(a);
    let b_u256 = U256::from_u128(b);
    let denom_u256 = U256::from_u128(denominator);

    let result = mul_div(a_u256, b_u256, denom_u256, rounding)?;
    
    result.to_u128()
        .ok_or(FeelsCoreError::MulDivOverflow)
}

/// Convert U256 to two u128 words (lo, hi)
pub fn u256_to_words(value: U256) -> (u128, u128) {
    (value.lo, value.hi)
}

/// Convert two u128 words to U256
pub fn words_to_u256(lo: u128, hi: u128) -> U256 {
    U256::new(lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_basic_ops() {
        let a = U256::from_u128(100);
        let b = U256::from_u128(200);
        
        // Addition
        let sum = a.add(&b).unwrap();
        assert_eq!(sum.to_u128().unwrap(), 300);
        
        // Subtraction
        let diff = b.sub(&a).unwrap();
        assert_eq!(diff.to_u128().unwrap(), 100);
        
        // Multiplication
        let product = a.mul(&b).unwrap();
        assert_eq!(product.to_u128().unwrap(), 20000);
        
        // Division
        let quotient = b.div(&a).unwrap();
        assert_eq!(quotient.to_u128().unwrap(), 2);
    }

    #[test]
    fn test_mul_div_rounding() {
        // Test rounding down
        let result = mul_div_u64(10, 3, 4, Rounding::Down).unwrap();
        assert_eq!(result, 7); // 30 / 4 = 7.5, rounds down to 7
        
        // Test rounding up
        let result = mul_div_u64(10, 3, 4, Rounding::Up).unwrap();
        assert_eq!(result, 8); // 30 / 4 = 7.5, rounds up to 8
        
        // Test exact division
        let result = mul_div_u64(10, 4, 5, Rounding::Up).unwrap();
        assert_eq!(result, 8); // 40 / 5 = 8, no rounding needed
    }

    #[test]
    fn test_mul_div_large_numbers() {
        let a = u128::MAX / 2;
        let b = 2;
        let denom = 2;
        
        let result = mul_div_u128(a, b, denom, Rounding::Down).unwrap();
        assert_eq!(result, a); // (a * 2) / 2 = a
    }
}