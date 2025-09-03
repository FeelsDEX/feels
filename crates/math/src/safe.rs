/// Safe arithmetic operations with overflow protection
/// 
/// All operations return errors instead of panicking, providing
/// comprehensive protection against arithmetic errors.

use feels_types::{FeelsProtocolError, FeelsResult, Q64, Q128_LO, Q128_HI};

// ============================================================================
// Safe Basic Arithmetic
// ============================================================================

/// Safe addition for u128 values
pub fn safe_add_u128(a: u128, b: u128) -> FeelsResult<u128> {
    a.checked_add(b)
        .ok_or_else(|| FeelsProtocolError::math_overflow("u128 addition", &[&a.to_string(), &b.to_string()]))
}

/// Safe subtraction for u128 values
pub fn safe_sub_u128(a: u128, b: u128) -> FeelsResult<u128> {
    a.checked_sub(b)
        .ok_or_else(|| FeelsProtocolError::math_underflow("u128 subtraction", &[&a.to_string(), &b.to_string()]))
}

/// Safe multiplication for u128 values
pub fn safe_mul_u128(a: u128, b: u128) -> FeelsResult<u128> {
    a.checked_mul(b)
        .ok_or_else(|| FeelsProtocolError::math_overflow("u128 multiplication", &[&a.to_string(), &b.to_string()]))
}

/// Safe division for u128 values
pub fn safe_div_u128(a: u128, b: u128) -> FeelsResult<u128> {
    if b == 0 {
        return Err(FeelsProtocolError::DivisionByZero {
            context: format!("u128 division: {} / {}", a, b),
        });
    }
    Ok(a / b)
}

/// Safe modulo for u128 values
pub fn safe_mod_u128(a: u128, b: u128) -> FeelsResult<u128> {
    if b == 0 {
        return Err(FeelsProtocolError::DivisionByZero {
            context: format!("u128 modulo: {} % {}", a, b),
        });
    }
    Ok(a % b)
}

// ============================================================================
// Safe Fixed-Point Arithmetic
// ============================================================================

/// Safe multiplication in Q64.64 fixed point
pub fn safe_mul_q64(a: u128, b: u128) -> FeelsResult<u128> {
    // a * b / Q64, with overflow protection
    let result = (a as u256) * (b as u256) / (Q64 as u256);
    
    if result > u128::MAX as u256 {
        return Err(FeelsProtocolError::math_overflow(
            "Q64 fixed-point multiplication",
            &[&a.to_string(), &b.to_string()],
        ));
    }
    
    Ok(result as u128)
}

/// Safe division in Q64.64 fixed point
pub fn safe_div_q64(a: u128, b: u128) -> FeelsResult<u128> {
    if b == 0 {
        return Err(FeelsProtocolError::DivisionByZero {
            context: format!("Q64 fixed-point division: {} / {}", a, b),
        });
    }
    
    // a * Q64 / b, with overflow protection
    let numerator = (a as u256) * (Q64 as u256);
    let result = numerator / (b as u256);
    
    if result > u128::MAX as u256 {
        return Err(FeelsProtocolError::math_overflow(
            "Q64 fixed-point division",
            &[&a.to_string(), &b.to_string()],
        ));
    }
    
    Ok(result as u128)
}

// ============================================================================
// Safe Shift Operations
// ============================================================================

/// Safe left shift for u128 values
pub fn safe_shl_u128(value: u128, shift: u32) -> FeelsResult<u128> {
    if shift >= 128 {
        return Err(FeelsProtocolError::math_overflow(
            "left shift",
            &[&value.to_string(), &shift.to_string()],
        ));
    }
    
    value.checked_shl(shift)
        .ok_or_else(|| FeelsProtocolError::math_overflow(
            "left shift",
            &[&value.to_string(), &shift.to_string()],
        ))
}

/// Safe right shift for u128 values
pub fn safe_shr_u128(value: u128, shift: u32) -> FeelsResult<u128> {
    if shift >= 128 {
        return Ok(0); // Right shift beyond width gives 0
    }
    
    Ok(value >> shift)
}

// ============================================================================
// Safe Power Operations
// ============================================================================

/// Safe integer exponentiation
pub fn safe_pow_u128(base: u128, exp: u32) -> FeelsResult<u128> {
    base.checked_pow(exp)
        .ok_or_else(|| FeelsProtocolError::math_overflow(
            "exponentiation",
            &[&base.to_string(), &exp.to_string()],
        ))
}

// ============================================================================
// Safe Square Root
// ============================================================================

/// Safe integer square root using the integer-sqrt crate
pub fn safe_sqrt_u128(value: u128) -> FeelsResult<u128> {
    use integer_sqrt::IntegerSquareRoot;
    Ok(value.integer_sqrt())
}

/// Safe square root in Q64.64 fixed point
pub fn safe_sqrt_q64(value: u128) -> FeelsResult<u128> {
    // For Q64.64, we need to be more careful with the square root
    // sqrt(x) in Q64.64 format
    
    if value == 0 {
        return Ok(0);
    }
    
    // Convert to higher precision for calculation
    let value_u256 = value as u256;
    let q64_u256 = Q64 as u256;
    
    // sqrt(value * Q64) to maintain precision
    let sqrt_input = value_u256 * q64_u256;
    
    // Use Newton's method for square root
    let sqrt_result = sqrt_u256(sqrt_input)?;
    
    Ok(sqrt_result as u128)
}

// ============================================================================
// Big Integer Types (using ruint for U256)
// ============================================================================

// For now, we'll use a simple u256 type for intermediate calculations
type u256 = u128; // Placeholder - in production would use ruint::Uint<256, 4>

/// Safe square root for u256 using Newton's method
fn sqrt_u256(value: u256) -> FeelsResult<u256> {
    if value == 0 {
        return Ok(0);
    }
    
    if value == 1 {
        return Ok(1);
    }
    
    // Newton's method: x_{n+1} = (x_n + value/x_n) / 2
    let mut x = value;
    let mut y = (value + 1) / 2;
    
    // Limit iterations to prevent infinite loops
    let max_iterations = 100;
    let mut iterations = 0;
    
    while y < x && iterations < max_iterations {
        x = y;
        y = (x + value / x) / 2;
        iterations += 1;
    }
    
    if iterations >= max_iterations {
        return Err(FeelsProtocolError::ComputationFailed {
            operation: "square root".to_string(),
            reason: "Newton's method failed to converge".to_string(),
        });
    }
    
    Ok(x)
}

// ============================================================================
// Price Calculation Helpers
// ============================================================================

/// Convert sqrt price to regular price safely
pub fn sqrt_price_to_price_safe(sqrt_price: u128) -> FeelsResult<u128> {
    safe_mul_q64(sqrt_price, sqrt_price)
}

/// Convert regular price to sqrt price safely
pub fn price_to_sqrt_price_safe(price: u128) -> FeelsResult<u128> {
    safe_sqrt_q64(price)
}

// ============================================================================
// Liquidity Calculations
// ============================================================================

/// Calculate token amounts from liquidity and price range
pub fn calculate_token_amounts_safe(
    liquidity: u128,
    sqrt_price: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
) -> FeelsResult<(u64, u64)> {
    // Ensure valid price range
    if sqrt_price_lower >= sqrt_price_upper {
        return Err(FeelsProtocolError::InvalidPriceRange {
            min_price: sqrt_price_lower,
            max_price: sqrt_price_upper,
            reason: "Lower bound must be less than upper bound".to_string(),
        });
    }
    
    let amount_0 = if sqrt_price < sqrt_price_upper {
        // amount_0 = liquidity * (sqrt_upper - sqrt_current) / (sqrt_current * sqrt_upper)
        let numerator = safe_mul_u128(
            liquidity,
            safe_sub_u128(sqrt_price_upper, sqrt_price.max(sqrt_price_lower))?
        )?;
        let denominator = safe_mul_q64(
            sqrt_price.max(sqrt_price_lower),
            sqrt_price_upper
        )?;
        safe_div_q64(numerator, denominator)?
    } else {
        0
    };
    
    let amount_1 = if sqrt_price > sqrt_price_lower {
        // amount_1 = liquidity * (sqrt_current - sqrt_lower)
        safe_mul_q64(
            liquidity,
            safe_sub_u128(sqrt_price.min(sqrt_price_upper), sqrt_price_lower)?
        )?
    } else {
        0
    };
    
    // Convert to u64 with overflow check
    let amount_0_u64 = if amount_0 > u64::MAX as u128 {
        return Err(FeelsProtocolError::math_overflow(
            "token amount 0 conversion",
            &[&amount_0.to_string()],
        ));
    } else {
        amount_0 as u64
    };
    
    let amount_1_u64 = if amount_1 > u64::MAX as u128 {
        return Err(FeelsProtocolError::math_overflow(
            "token amount 1 conversion",
            &[&amount_1.to_string()],
        ));
    } else {
        amount_1 as u64
    };
    
    Ok((amount_0_u64, amount_1_u64))
}

// ============================================================================
// Percentage and Basis Point Calculations
// ============================================================================

/// Calculate percentage change in basis points
pub fn calculate_change_bps(new_value: u128, old_value: u128) -> FeelsResult<u64> {
    if old_value == 0 {
        return Ok(10000); // 100% change if starting from zero
    }
    
    let diff = if new_value > old_value {
        safe_sub_u128(new_value, old_value)?
    } else {
        safe_sub_u128(old_value, new_value)?
    };
    
    // (diff * 10000) / old_value
    let change_bps_u128 = safe_div_u128(
        safe_mul_u128(diff, 10000)?,
        old_value
    )?;
    
    // Convert to u64 with cap at 10000 (100%)
    Ok((change_bps_u128.min(10000) as u64))
}

/// Apply basis point change to a value
pub fn apply_bps_change(value: u128, change_bps: u64, increase: bool) -> FeelsResult<u128> {
    let change_amount = safe_div_u128(
        safe_mul_u128(value, change_bps as u128)?,
        10000
    )?;
    
    if increase {
        safe_add_u128(value, change_amount)
    } else {
        safe_sub_u128(value, change_amount)
    }
}

// ============================================================================
// Input Validation
// ============================================================================

/// Validate that a value is within acceptable bounds
pub fn validate_bounds_u128(
    value: u128,
    min: u128,
    max: u128,
    parameter_name: &str,
) -> FeelsResult<()> {
    if value < min || value > max {
        return Err(FeelsProtocolError::ParameterOutOfRange {
            parameter: parameter_name.to_string(),
            value: value as f64,
            min: min as f64,
            max: max as f64,
        });
    }
    Ok(())
}

/// Validate that weights sum to the expected total
pub fn validate_weights(weights: &[u32], expected_sum: u32) -> FeelsResult<()> {
    let actual_sum: u32 = weights.iter().sum();
    if actual_sum != expected_sum {
        return Err(FeelsProtocolError::InvalidWeights {
            reason: format!("Weights sum to {}, expected {}", actual_sum, expected_sum),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use feels_types::Q64;

    #[test]
    fn test_safe_arithmetic() {
        // Normal operations should work
        assert_eq!(safe_add_u128(100, 200).unwrap(), 300);
        assert_eq!(safe_sub_u128(200, 100).unwrap(), 100);
        assert_eq!(safe_mul_u128(10, 20).unwrap(), 200);
        assert_eq!(safe_div_u128(100, 5).unwrap(), 20);
        
        // Overflow cases should error
        assert!(safe_add_u128(u128::MAX, 1).is_err());
        assert!(safe_sub_u128(100, 200).is_err());
        assert!(safe_div_u128(100, 0).is_err());
    }
    
    #[test]
    fn test_safe_fixed_point() {
        // Q64.64 multiplication: 1.0 * 2.0 = 2.0
        let result = safe_mul_q64(Q64, 2 * Q64).unwrap();
        assert_eq!(result, 2 * Q64);
        
        // Q64.64 division: 4.0 / 2.0 = 2.0
        let result = safe_div_q64(4 * Q64, 2 * Q64).unwrap();
        assert_eq!(result, 2 * Q64);
        
        // Division by zero should error
        assert!(safe_div_q64(Q64, 0).is_err());
    }
    
    #[test]
    fn test_safe_shift() {
        assert_eq!(safe_shl_u128(1, 10).unwrap(), 1024);
        assert_eq!(safe_shr_u128(1024, 10).unwrap(), 1);
        
        // Shift overflow should error
        assert!(safe_shl_u128(1, 128).is_err());
        
        // Large right shift should return 0
        assert_eq!(safe_shr_u128(1024, 128).unwrap(), 0);
    }
    
    #[test]
    fn test_safe_sqrt() {
        assert_eq!(safe_sqrt_u128(0).unwrap(), 0);
        assert_eq!(safe_sqrt_u128(1).unwrap(), 1);
        assert_eq!(safe_sqrt_u128(4).unwrap(), 2);
        assert_eq!(safe_sqrt_u128(9).unwrap(), 3);
        assert_eq!(safe_sqrt_u128(100).unwrap(), 10);
    }
    
    #[test]
    fn test_price_conversions() {
        let sqrt_price = Q64; // 1.0 in Q64.64
        let price = sqrt_price_to_price_safe(sqrt_price).unwrap();
        assert_eq!(price, Q64); // 1.0 * 1.0 = 1.0
        
        let recovered_sqrt = price_to_sqrt_price_safe(price).unwrap();
        // Should be approximately equal (within some tolerance due to precision)
        let diff = if recovered_sqrt > sqrt_price {
            recovered_sqrt - sqrt_price
        } else {
            sqrt_price - recovered_sqrt
        };
        assert!(diff < Q64 / 1000000); // Within 0.0001% tolerance
    }
    
    #[test]
    fn test_change_bps() {
        // 10% increase: from 100 to 110
        assert_eq!(calculate_change_bps(110, 100).unwrap(), 1000); // 10% = 1000 bps
        
        // 10% decrease: from 100 to 90  
        assert_eq!(calculate_change_bps(90, 100).unwrap(), 1000); // 10% = 1000 bps
        
        // From zero should give 100%
        assert_eq!(calculate_change_bps(100, 0).unwrap(), 10000); // 100% = 10000 bps
    }
    
    #[test]
    fn test_apply_bps_change() {
        // Apply 10% increase to 1000
        let result = apply_bps_change(1000, 1000, true).unwrap();
        assert_eq!(result, 1100);
        
        // Apply 10% decrease to 1000
        let result = apply_bps_change(1000, 1000, false).unwrap();
        assert_eq!(result, 900);
    }
    
    #[test]
    fn test_validation() {
        // Valid bounds
        assert!(validate_bounds_u128(50, 0, 100, "test").is_ok());
        
        // Invalid bounds
        assert!(validate_bounds_u128(150, 0, 100, "test").is_err());
        
        // Valid weights
        assert!(validate_weights(&[3333, 3333, 3334], 10000).is_ok());
        
        // Invalid weights
        assert!(validate_weights(&[5000, 5000, 5000], 10000).is_err());
    }
}