/// Fixed-point arithmetic utilities for high-precision calculations

use feels_types::{FeelsResult, FeelsProtocolError, Q64};
use crate::safe::*;

// ============================================================================
// Fixed-Point Conversion Functions
// ============================================================================

/// Convert f64 to Q64.64 fixed-point
pub fn f64_to_q64(value: f64) -> FeelsResult<u128> {
    if value < 0.0 {
        return Err(FeelsProtocolError::invalid_parameter(
            "f64_to_q64", &value.to_string(), "non-negative value"
        ));
    }
    
    if value >= (u64::MAX as f64) {
        return Err(FeelsProtocolError::math_overflow(
            "f64 to Q64 conversion", &[&value.to_string()]
        ));
    }
    
    let q64_value = (value * (Q64 as f64)) as u128;
    Ok(q64_value)
}

/// Convert Q64.64 fixed-point to f64
pub fn q64_to_f64(value: u128) -> f64 {
    (value as f64) / (Q64 as f64)
}

/// Convert u64 integer to Q64.64 fixed-point
pub fn u64_to_q64(value: u64) -> u128 {
    (value as u128) * Q64
}

/// Convert Q64.64 fixed-point to u64 integer (truncating fractional part)
pub fn q64_to_u64(value: u128) -> u64 {
    (value / Q64) as u64
}

/// Convert Q64.64 fixed-point to u64 integer with rounding
pub fn q64_to_u64_round(value: u128) -> u64 {
    ((value + Q64/2) / Q64) as u64
}

// ============================================================================
// Fixed-Point Arithmetic Operations
// ============================================================================

/// Add two Q64.64 fixed-point numbers
pub fn add_q64(a: u128, b: u128) -> FeelsResult<u128> {
    safe_add_u128(a, b)
}

/// Subtract two Q64.64 fixed-point numbers  
pub fn sub_q64(a: u128, b: u128) -> FeelsResult<u128> {
    safe_sub_u128(a, b)
}

/// Multiply two Q64.64 fixed-point numbers
pub fn mul_q64(a: u128, b: u128) -> FeelsResult<u128> {
    safe_mul_q64(a, b)
}

/// Divide two Q64.64 fixed-point numbers
pub fn div_q64(a: u128, b: u128) -> FeelsResult<u128> {
    safe_div_q64(a, b)
}

/// Square a Q64.64 fixed-point number
pub fn square_q64(value: u128) -> FeelsResult<u128> {
    mul_q64(value, value)
}

/// Take square root of Q64.64 fixed-point number
pub fn sqrt_q64(value: u128) -> FeelsResult<u128> {
    safe_sqrt_q64(value)
}

// ============================================================================
// Advanced Mathematical Functions
// ============================================================================

/// Calculate x^y in Q64.64 fixed point (for small integer y)
pub fn pow_q64(base: u128, exponent: u32) -> FeelsResult<u128> {
    if exponent == 0 {
        return Ok(Q64); // x^0 = 1
    }
    
    if exponent == 1 {
        return Ok(base);
    }
    
    if base == 0 {
        return Ok(0);
    }
    
    if base == Q64 {
        return Ok(Q64); // 1^y = 1
    }
    
    // Use exponentiation by squaring for efficiency
    let mut result = Q64;
    let mut base_power = base;
    let mut exp = exponent;
    
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_q64(result, base_power)?;
        }
        base_power = mul_q64(base_power, base_power)?;
        exp >>= 1;
    }
    
    Ok(result)
}

/// Natural logarithm approximation in Q64.64 fixed point
pub fn ln_q64(x: u128) -> FeelsResult<i128> {
    if x == 0 {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "ln".to_string(),
            reason: "logarithm of zero".to_string(),
        });
    }
    
    if x == Q64 {
        return Ok(0); // ln(1) = 0
    }
    
    // For x close to 1, use Taylor series: ln(1+u) = u - u²/2 + u³/3 - u⁴/4 + ...
    let one_q64 = Q64;
    
    if x > one_q64 / 2 && x < 2 * one_q64 {
        // Use Taylor series for values near 1
        let u = (x as i128) - (one_q64 as i128); // x - 1
        
        // First few terms of Taylor series
        let u2 = (u * u) >> 64;
        let u3 = (u2 * u) >> 64;
        let u4 = (u3 * u) >> 64;
        
        let term1 = u;
        let term2 = u2 >> 1;  // u²/2
        let term3 = u3 / 3;   // u³/3
        let term4 = u4 >> 2;  // u⁴/4
        
        Ok(term1 - term2 + term3 - term4)
    } else {
        // For values far from 1, use iterative approximation or lookup table
        // This is a simplified version - production would use more sophisticated methods
        
        // Use the identity: ln(x) = ln(2^k * m) = k*ln(2) + ln(m) where 1 ≤ m < 2
        let mut k = 0i32;
        let mut mantissa = x;
        
        // Normalize mantissa to [1, 2) range
        while mantissa >= 2 * one_q64 {
            mantissa >>= 1;
            k += 1;
        }
        
        while mantissa < one_q64 {
            mantissa <<= 1;
            k -= 1;
        }
        
        // Now mantissa is in [1, 2), calculate ln(mantissa) using Taylor series
        let u = (mantissa as i128) - (one_q64 as i128);
        let u2 = (u * u) >> 64;
        let u3 = (u2 * u) >> 64;
        
        let ln_mantissa = u - (u2 >> 1) + (u3 / 3);
        
        // ln(2) ≈ 0.693147 in Q64.64
        let ln_2_q64 = (0.693147180559945 * (Q64 as f64)) as i128;
        let k_ln_2 = (k as i128) * ln_2_q64;
        
        Ok(k_ln_2 + ln_mantissa)
    }
}

/// Exponential function e^x in Q64.64 fixed point
pub fn exp_q64(x: i128) -> FeelsResult<u128> {
    if x == 0 {
        return Ok(Q64); // e^0 = 1
    }
    
    // Use Taylor series: e^x = 1 + x + x²/2! + x³/3! + x⁴/4! + ...
    // For numerical stability, use the identity: e^x = e^(n*ln(2)) * e^(x - n*ln(2))
    // where we choose n to make (x - n*ln(2)) small
    
    let ln_2_q64 = (0.693147180559945 * (Q64 as f64)) as i128;
    let n = x / ln_2_q64; // Integer part
    let remainder = x - n * ln_2_q64; // Fractional part
    
    // Calculate e^remainder using Taylor series (remainder should be small)
    let mut result = Q64 as i128; // 1.0
    let mut term = remainder; // x
    let mut factorial = 1i128;
    
    // Add first few terms of Taylor series
    for i in 1..10 { // Limit terms for performance
        result += term / factorial;
        term = (term * remainder) >> 64; // x^(i+1)
        factorial *= (i + 1) as i128;
    }
    
    // Now multiply by 2^n
    if n >= 0 {
        let shift_amount = n.min(63) as u32; // Prevent overflow
        result = result.checked_shl(shift_amount)
            .ok_or_else(|| FeelsProtocolError::math_overflow("exp", &[&x.to_string()]))?;
    } else {
        let shift_amount = (-n).min(127) as u32;
        result >>= shift_amount;
    }
    
    if result < 0 {
        return Err(FeelsProtocolError::math_underflow("exp", &[&x.to_string()]));
    }
    
    Ok(result as u128)
}

// ============================================================================
// Trigonometric Functions (Basic Approximations)
// ============================================================================

/// Sine function using Taylor series (for small angles in Q64.64)
pub fn sin_q64(x: i128) -> FeelsResult<i128> {
    // sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
    
    // For numerical stability, reduce x to [-π, π] range
    let pi_q64 = (std::f64::consts::PI * (Q64 as f64)) as i128;
    let two_pi_q64 = 2 * pi_q64;
    
    let mut x_reduced = x % two_pi_q64;
    if x_reduced > pi_q64 {
        x_reduced -= two_pi_q64;
    } else if x_reduced < -pi_q64 {
        x_reduced += two_pi_q64;
    }
    
    // Taylor series
    let x2 = (x_reduced * x_reduced) >> 64;
    let x3 = (x2 * x_reduced) >> 64;
    let x5 = (x3 * x2) >> 64;
    let x7 = (x5 * x2) >> 64;
    
    let term1 = x_reduced;
    let term3 = x3 / 6;     // x³/3!
    let term5 = x5 / 120;   // x⁵/5!  
    let term7 = x7 / 5040;  // x⁷/7!
    
    Ok(term1 - term3 + term5 - term7)
}

/// Cosine function using Taylor series
pub fn cos_q64(x: i128) -> FeelsResult<i128> {
    // cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
    
    // Reduce to [-π, π] range
    let pi_q64 = (std::f64::consts::PI * (Q64 as f64)) as i128;
    let two_pi_q64 = 2 * pi_q64;
    
    let mut x_reduced = x % two_pi_q64;
    if x_reduced > pi_q64 {
        x_reduced -= two_pi_q64;
    } else if x_reduced < -pi_q64 {
        x_reduced += two_pi_q64;
    }
    
    let x2 = (x_reduced * x_reduced) >> 64;
    let x4 = (x2 * x2) >> 64;
    let x6 = (x4 * x2) >> 64;
    let x8 = (x6 * x2) >> 64;
    
    let term0 = Q64 as i128;  // 1
    let term2 = x2 >> 1;      // x²/2!
    let term4 = x4 / 24;      // x⁴/4!
    let term6 = x6 / 720;     // x⁶/6!
    let term8 = x8 / 40320;   // x⁸/8!
    
    Ok(term0 - term2 + term4 - term6 + term8)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get the fractional part of a Q64.64 fixed-point number
pub fn frac_q64(value: u128) -> u128 {
    value & (Q64 - 1)
}

/// Get the integer part of a Q64.64 fixed-point number
pub fn int_q64(value: u128) -> u128 {
    value & !((Q64 - 1))
}

/// Check if two Q64.64 values are approximately equal
pub fn approx_eq_q64(a: u128, b: u128, tolerance: u128) -> bool {
    let diff = if a > b { a - b } else { b - a };
    diff <= tolerance
}

/// Clamp a Q64.64 value to a range
pub fn clamp_q64(value: u128, min_val: u128, max_val: u128) -> u128 {
    if value < min_val {
        min_val
    } else if value > max_val {
        max_val
    } else {
        value
    }
}

/// Linear interpolation between two Q64.64 values
pub fn lerp_q64(start: u128, end: u128, t: u128) -> FeelsResult<u128> {
    if t > Q64 {
        return Err(FeelsProtocolError::invalid_parameter(
            "lerp_t", &q64_to_f64(t).to_string(), "value in [0, 1]"
        ));
    }
    
    let range = if end > start {
        sub_q64(end, start)?
    } else {
        sub_q64(start, end)?
    };
    
    let offset = mul_q64(range, t)?;
    
    if end > start {
        add_q64(start, offset)
    } else {
        sub_q64(start, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_point_conversions() {
        // Test f64 to Q64 conversion
        let q64_one = f64_to_q64(1.0).unwrap();
        assert_eq!(q64_one, Q64);
        
        let q64_half = f64_to_q64(0.5).unwrap();
        assert_eq!(q64_half, Q64 / 2);
        
        // Test Q64 to f64 conversion
        assert!((q64_to_f64(Q64) - 1.0).abs() < 1e-10);
        assert!((q64_to_f64(Q64 / 2) - 0.5).abs() < 1e-10);
        
        // Test integer conversions
        assert_eq!(u64_to_q64(5), 5 * Q64);
        assert_eq!(q64_to_u64(5 * Q64), 5);
        
        // Test rounding
        assert_eq!(q64_to_u64_round(Q64 + Q64/3), 1); // 1.33 rounds to 1
        assert_eq!(q64_to_u64_round(Q64 + Q64/2 + 1), 2); // 1.5+ rounds to 2
    }
    
    #[test]
    fn test_fixed_point_arithmetic() {
        let a = f64_to_q64(2.5).unwrap();
        let b = f64_to_q64(1.5).unwrap();
        
        // Addition
        let sum = add_q64(a, b).unwrap();
        assert!((q64_to_f64(sum) - 4.0).abs() < 1e-10);
        
        // Subtraction  
        let diff = sub_q64(a, b).unwrap();
        assert!((q64_to_f64(diff) - 1.0).abs() < 1e-10);
        
        // Multiplication
        let prod = mul_q64(a, b).unwrap();
        assert!((q64_to_f64(prod) - 3.75).abs() < 1e-6);
        
        // Division
        let quot = div_q64(a, b).unwrap();
        assert!((q64_to_f64(quot) - (2.5/1.5)).abs() < 1e-6);
    }
    
    #[test]
    fn test_power_function() {
        let base = f64_to_q64(2.0).unwrap();
        
        // 2^0 = 1
        assert_eq!(pow_q64(base, 0).unwrap(), Q64);
        
        // 2^1 = 2
        assert_eq!(pow_q64(base, 1).unwrap(), base);
        
        // 2^3 = 8
        let result = pow_q64(base, 3).unwrap();
        assert!((q64_to_f64(result) - 8.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_logarithm() {
        // ln(1) = 0
        let ln_one = ln_q64(Q64).unwrap();
        assert_eq!(ln_one, 0);
        
        // ln(e) ≈ 1
        let e_q64 = f64_to_q64(std::f64::consts::E).unwrap();
        let ln_e = ln_q64(e_q64).unwrap();
        let ln_e_f64 = (ln_e as f64) / (Q64 as f64);
        assert!((ln_e_f64 - 1.0).abs() < 0.01); // Within 1% error
    }
    
    #[test]
    fn test_exponential() {
        // e^0 = 1
        let exp_zero = exp_q64(0).unwrap();
        assert_eq!(exp_zero, Q64);
        
        // e^1 ≈ e
        let exp_one = exp_q64(Q64 as i128).unwrap();
        let exp_one_f64 = q64_to_f64(exp_one);
        assert!((exp_one_f64 - std::f64::consts::E).abs() < 0.01);
    }
    
    #[test]
    fn test_trigonometric() {
        // sin(0) = 0
        let sin_zero = sin_q64(0).unwrap();
        assert_eq!(sin_zero, 0);
        
        // cos(0) = 1
        let cos_zero = cos_q64(0).unwrap();
        assert!((cos_zero - Q64 as i128).abs() < 100);
        
        // sin(π/2) ≈ 1
        let pi_half = (std::f64::consts::PI / 2.0 * (Q64 as f64)) as i128;
        let sin_pi_half = sin_q64(pi_half).unwrap();
        let sin_pi_half_f64 = (sin_pi_half as f64) / (Q64 as f64);
        assert!((sin_pi_half_f64 - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_utility_functions() {
        let value = f64_to_q64(3.75).unwrap(); // 3 + 0.75
        
        // Test fractional part
        let frac = frac_q64(value);
        assert!((q64_to_f64(frac) - 0.75).abs() < 1e-10);
        
        // Test integer part  
        let int_part = int_q64(value);
        assert_eq!(q64_to_u64(int_part), 3);
        
        // Test clamping
        let min_val = f64_to_q64(1.0).unwrap();
        let max_val = f64_to_q64(5.0).unwrap();
        
        let clamped_low = clamp_q64(f64_to_q64(0.5).unwrap(), min_val, max_val);
        assert_eq!(clamped_low, min_val);
        
        let clamped_high = clamp_q64(f64_to_q64(10.0).unwrap(), min_val, max_val);
        assert_eq!(clamped_high, max_val);
        
        // Test interpolation
        let start = f64_to_q64(10.0).unwrap();
        let end = f64_to_q64(20.0).unwrap();
        let half = f64_to_q64(0.5).unwrap();
        
        let lerp_result = lerp_q64(start, end, half).unwrap();
        assert!((q64_to_f64(lerp_result) - 15.0).abs() < 1e-10);
    }
}