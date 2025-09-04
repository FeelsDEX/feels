//! # Fixed-Point Arithmetic
//! 
//! Q64, Q96, and Q128 fixed-point math operations with advanced functions.
//! 
//! This module provides both basic fixed-point arithmetic and advanced mathematical
//! functions (logarithms, exponentials, powers) needed by both on-chain and off-chain
//! components of the Feels protocol.

use crate::constants::{Q64, Q96};
use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::safe_math::{safe_add_u128, safe_sub_u128, safe_mul_u128, safe_div_u128, safe_shl_u128, safe_shr_u128, safe_mul_q64, safe_div_q64, safe_sqrt_q64};

// ============================================================================
// Fixed-Point Conversion Functions
// ============================================================================

/// Convert f64 to Q64.64 fixed-point
pub fn f64_to_q64(value: f64) -> CoreResult<u128> {
    if value < 0.0 {
        return Err(FeelsCoreError::InvalidParameter);
    }
    
    if value >= (u64::MAX as f64) {
        return Err(FeelsCoreError::MathOverflow);
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
// Basic Fixed-Point Arithmetic
// ============================================================================

/// Add two Q64 fixed-point numbers
pub fn add_q64(a: u128, b: u128) -> CoreResult<u128> {
    safe_add_u128(a, b)
}

/// Subtract two Q64 fixed-point numbers  
pub fn sub_q64(a: u128, b: u128) -> CoreResult<u128> {
    safe_sub_u128(a, b)
}

/// Multiply two Q64 fixed-point numbers
pub fn mul_q64(a: u128, b: u128) -> CoreResult<u128> {
    safe_mul_q64(a, b)
}

/// Divide two Q64 fixed-point numbers
pub fn div_q64(a: u128, b: u128) -> CoreResult<u128> {
    safe_div_q64(a, b)
}

/// Multiply two Q96 fixed-point numbers
pub fn mul_q96(a: u128, b: u128) -> CoreResult<u128> {
    let product = safe_mul_u128(a, b)?;
    safe_div_u128(product, Q96)
}

/// Divide two Q96 fixed-point numbers
pub fn div_q96(a: u128, b: u128) -> CoreResult<u128> {
    let scaled = safe_mul_u128(a, Q96)?;
    safe_div_u128(scaled, b)
}

/// Square root of Q64 number
pub fn sqrt_q64(value: u128) -> CoreResult<u128> {
    safe_sqrt_q64(value)
}

/// Convert Q64 to Q96
pub fn q64_to_q96(value: u128) -> CoreResult<u128> {
    safe_shl_u128(value, 32)
}

/// Convert Q96 to Q64
pub fn q96_to_q64(value: u128) -> CoreResult<u128> {
    safe_shr_u128(value, 32)
}

/// Convert basis points to Q64
pub fn bps_to_q64(bps: u32) -> u128 {
    (bps as u128 * Q64) / 10_000
}

/// Convert Q64 to basis points
pub fn q64_to_bps(value: u128) -> CoreResult<u32> {
    let bps = safe_div_u128(safe_mul_u128(value, 10_000)?, Q64)?;
    if bps > u32::MAX as u128 {
        return Err(FeelsCoreError::ConversionError);
    }
    Ok(bps as u32)
}

/// Square a Q64.64 fixed-point number
pub fn square_q64(value: u128) -> CoreResult<u128> {
    mul_q64(value, value)
}

// ============================================================================
// Advanced Mathematical Functions (for off-chain use)
// ============================================================================

/// Calculate x^y in Q64.64 fixed point (for small integer y)
#[cfg(feature = "advanced")]
pub fn pow_q64(base: u128, exponent: u32) -> CoreResult<u128> {
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
/// Returns result in Q64 format (can be negative, hence i128)
#[cfg(feature = "advanced")]
pub fn ln_q64(x: u128) -> CoreResult<i128> {
    if x == 0 {
        return Err(FeelsCoreError::InvalidParameter);
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
        // For values far from 1, use iterative approximation
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
        let ln_2_q64 = 12786308645202655659i128; // 0.693147180559945 * 2^64
        let k_ln_2 = (k as i128) * ln_2_q64;
        
        Ok(k_ln_2 + ln_mantissa)
    }
}

/// Exponential function e^x in Q64.64 fixed point
/// Input x is in Q64 format (can be negative)
#[cfg(feature = "advanced")]
pub fn exp_q64(x: i128) -> CoreResult<u128> {
    if x == 0 {
        return Ok(Q64); // e^0 = 1
    }
    
    // For numerical stability, use the identity: e^x = e^(n*ln(2)) * e^(x - n*ln(2))
    let ln_2_q64 = 12786308645202655659i128; // 0.693147180559945 * 2^64
    let n = x / ln_2_q64; // Integer part
    let remainder = x - n * ln_2_q64; // Fractional part
    
    // Calculate e^remainder using Taylor series (remainder should be small)
    // e^x = 1 + x + x²/2! + x³/3! + x⁴/4! + ...
    let mut result = Q64 as i128; // 1.0
    let mut term = remainder; // x
    let mut i = 1i128;
    
    // Add first few terms of Taylor series
    for _ in 1..10 { // Limit terms for performance
        result += term;
        term = (term * remainder) >> 64; // x^(i+1) in Q64
        i += 1;
        term = term / i; // divide by i!
    }
    
    // Now multiply by 2^n
    if n >= 0 {
        let shift_amount = (n as u32).min(63); // Prevent overflow
        if shift_amount > 63 {
            return Err(FeelsCoreError::MathOverflow);
        }
        result = result.checked_shl(shift_amount)
            .ok_or(FeelsCoreError::MathOverflow)?;
    } else {
        let shift_amount = ((-n) as u32).min(127);
        result >>= shift_amount;
    }
    
    if result < 0 {
        return Err(FeelsCoreError::MathUnderflow);
    }
    
    Ok(result as u128)
}