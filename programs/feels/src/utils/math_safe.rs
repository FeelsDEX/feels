/// Provides overflow-safe arithmetic operations for all numerical calculations.
/// Prevents common vulnerabilities like integer overflow attacks by using checked
/// arithmetic that returns errors instead of wrapping. Essential for maintaining
/// protocol integrity when handling large liquidity values and precise calculations.

use anchor_lang::prelude::*;
use crate::state::PoolError;

// ============================================================================
// Type Definitions
// ============================================================================

pub trait SafeMath<T> {
    fn safe_add(self, v: T) -> Result<T>;
    fn safe_sub(self, v: T) -> Result<T>;
    fn safe_mul(self, v: T) -> Result<T>;
    fn safe_div(self, v: T) -> Result<T>;
}

// ============================================================================
// Core Implementation
// ============================================================================

macro_rules! impl_safe_math {
    ($type:ty) => {
        impl SafeMath<$type> for $type {
            fn safe_add(self, v: $type) -> Result<$type> {
                self.checked_add(v).ok_or_else(|| {
                    msg!("Math overflow in safe_add: {} + {}", self, v);
                    PoolError::ArithmeticOverflow.into()
                })
            }

            fn safe_sub(self, v: $type) -> Result<$type> {
                self.checked_sub(v).ok_or_else(|| {
                    msg!("Math underflow in safe_sub: {} - {}", self, v);
                    PoolError::ArithmeticUnderflow.into()
                })
            }

            fn safe_mul(self, v: $type) -> Result<$type> {
                self.checked_mul(v).ok_or_else(|| {
                    msg!("Math overflow in safe_mul: {} * {}", self, v);
                    PoolError::ArithmeticOverflow.into()
                })
            }

            fn safe_div(self, v: $type) -> Result<$type> {
                if v == 0 {
                    msg!("Division by zero in safe_div: {} / {}", self, v);
                    return Err(PoolError::DivisionByZero.into());
                }
                self.checked_div(v).ok_or_else(|| {
                    msg!("Math error in safe_div: {} / {}", self, v);
                    PoolError::ArithmeticOverflow.into()
                })
            }
        }
    };
}

// ------------------------------------------------------------------------
// Standard Type Implementations
// ------------------------------------------------------------------------

// Implement SafeMath for common integer types
impl_safe_math!(u8);
impl_safe_math!(u16);
impl_safe_math!(u32);
impl_safe_math!(u64);
impl_safe_math!(u128);
impl_safe_math!(i8);
impl_safe_math!(i16);
impl_safe_math!(i32);
impl_safe_math!(i64);
impl_safe_math!(i128);

// ------------------------------------------------------------------------
// Specialized Implementations
// ------------------------------------------------------------------------

/// Safe math operations specifically for liquidity calculations
pub trait LiquiditySafeMath {
    /// Add liquidity with overflow protection
    fn safe_add_liquidity(self, delta: i128) -> Result<u128>;
    /// Subtract liquidity with underflow protection  
    fn safe_sub_liquidity(self, delta: i128) -> Result<u128>;
}

impl LiquiditySafeMath for u128 {
    fn safe_add_liquidity(self, delta: i128) -> Result<u128> {
        if delta >= 0 {
            self.safe_add(delta as u128)
        } else {
            self.safe_sub((-delta) as u128)
        }
    }

    fn safe_sub_liquidity(self, delta: i128) -> Result<u128> {
        if delta >= 0 {
            self.safe_sub(delta as u128)
        } else {
            self.safe_add((-delta) as u128)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_add_overflow() {
        let result = u64::MAX.safe_add(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_sub_underflow() {
        let result = 0u64.safe_sub(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_div_by_zero() {
        let result = 100u64.safe_div(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_liquidity_math() {
        let liquidity = 1000u128;
        let result = liquidity.safe_add_liquidity(500).unwrap();
        assert_eq!(result, 1500);
        
        let result = liquidity.safe_sub_liquidity(-200).unwrap();
        assert_eq!(result, 1200);
    }
}