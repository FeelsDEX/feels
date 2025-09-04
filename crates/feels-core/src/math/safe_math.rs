//! # Safe Math Operations
//! 
//! Overflow-checked arithmetic operations for both on-chain and off-chain use.

use crate::errors::{CoreResult, FeelsCoreError};

/// Macro to generate safe arithmetic functions
macro_rules! safe_arith {
    // Binary operations with checked methods
    ($fn_name:ident, $type:ty, $checked_method:ident, $error:expr) => {
        /// Safe $fn_name with overflow/underflow check
        pub fn $fn_name(a: $type, b: $type) -> CoreResult<$type> {
            a.$checked_method(b).ok_or($error)
        }
    };
    
    // Division operations with zero check
    (div, $fn_name:ident, $type:ty) => {
        /// Safe division with zero check
        pub fn $fn_name(a: $type, b: $type) -> CoreResult<$type> {
            if b == 0 {
                return Err(FeelsCoreError::DivisionByZero);
            }
            Ok(a / b)
        }
    };
    
    // Shift operations
    (shift, $fn_name:ident, $type:ty, $checked_method:ident, $error:expr) => {
        /// Safe $fn_name
        pub fn $fn_name(value: $type, shift: u32) -> CoreResult<$type> {
            value.$checked_method(shift).ok_or($error)
        }
    };
    
    // Type conversion operations
    (cast, $fn_name:ident, $from_type:ty, $to_type:ty, $max_val:expr, $min_val:expr) => {
        /// Safe cast from $from_type to $to_type
        pub fn $fn_name(value: $from_type) -> CoreResult<$to_type> {
            if value > $max_val || value < $min_val {
                return Err(FeelsCoreError::ConversionError);
            }
            Ok(value as $to_type)
        }
    };
    
    // Simple cast with only max check
    (cast_max, $fn_name:ident, $from_type:ty, $to_type:ty, $max_val:expr) => {
        /// Safe cast from $from_type to $to_type
        pub fn $fn_name(value: $from_type) -> CoreResult<$to_type> {
            if value > $max_val {
                return Err(FeelsCoreError::ConversionError);
            }
            Ok(value as $to_type)
        }
    };
}

// Generate basic arithmetic functions
safe_arith!(safe_add_u64, u64, checked_add, FeelsCoreError::MathOverflow);
safe_arith!(safe_sub_u64, u64, checked_sub, FeelsCoreError::MathUnderflow);
safe_arith!(safe_mul_u64, u64, checked_mul, FeelsCoreError::MathOverflow);
safe_arith!(div, safe_div_u64, u64);

safe_arith!(safe_add_u128, u128, checked_add, FeelsCoreError::MathOverflow);
safe_arith!(safe_sub_u128, u128, checked_sub, FeelsCoreError::MathUnderflow);
safe_arith!(safe_mul_u128, u128, checked_mul, FeelsCoreError::MathOverflow);
safe_arith!(div, safe_div_u128, u128);

safe_arith!(safe_add_i128, i128, checked_add, FeelsCoreError::MathOverflow);
safe_arith!(safe_sub_i128, i128, checked_sub, FeelsCoreError::MathUnderflow);
safe_arith!(safe_mul_i128, i128, checked_mul, FeelsCoreError::MathOverflow);
safe_arith!(div, safe_div_i128, i128);

// Generate shift operations
safe_arith!(shift, safe_shl_u128, u128, checked_shl, FeelsCoreError::MathOverflow);
safe_arith!(shift, safe_shr_u128, u128, checked_shr, FeelsCoreError::MathUnderflow);

// Generate type conversion functions
safe_arith!(cast_max, safe_cast_u128_to_u64, u128, u64, u64::MAX as u128);
safe_arith!(cast, safe_cast_i128_to_i64, i128, i64, i64::MAX as i128, i64::MIN as i128);

/// Calculate percentage (basis points)
pub fn safe_calculate_bps(value: u128, bps: u32) -> CoreResult<u128> {
    let result = safe_mul_u128(value, bps as u128)?;
    safe_div_u128(result, 10_000)
}

/// Mul-div operation with u64 to prevent overflow
pub fn safe_mul_div_u64(a: u64, b: u64, denominator: u64) -> CoreResult<u64> {
    crate::math::big_int::mul_div_u64(a, b, denominator, crate::math::big_int::Rounding::Down)
}

/// Mul-div operation with u128 using U256 intermediate
pub fn safe_mul_div_u128(a: u128, b: u128, c: u128) -> CoreResult<u128> {
    crate::math::big_int::mul_div(
        crate::math::big_int::U256::from_u128(a),
        crate::math::big_int::U256::from_u128(b),
        crate::math::big_int::U256::from_u128(c),
        crate::math::big_int::Rounding::Down,
    )
    .and_then(|result| result.to_u128().ok_or(FeelsCoreError::MathOverflow))
}

/// Add liquidity delta operations
pub fn safe_add_liquidity_delta(liquidity: u128, delta: i128) -> CoreResult<u128> {
    if delta >= 0 {
        safe_add_u128(liquidity, delta as u128)
    } else {
        safe_sub_u128(liquidity, (-delta) as u128)
    }
}

/// Subtract liquidity delta operations
pub fn safe_sub_liquidity_delta(liquidity: u128, delta: i128) -> CoreResult<u128> {
    if delta >= 0 {
        safe_sub_u128(liquidity, delta as u128)
    } else {
        safe_add_u128(liquidity, (-delta) as u128)
    }
}

/// Calculate percentage with basis points
pub fn safe_calculate_percentage(amount: u64, basis_points: u16) -> CoreResult<u64> {
    const BPS_DENOMINATOR: u64 = 10_000;
    if basis_points as u64 > BPS_DENOMINATOR {
        return Err(FeelsCoreError::MathOverflow);
    }
    safe_mul_div_u64(amount, basis_points as u64, BPS_DENOMINATOR)
}

/// Integer square root for u64
pub fn sqrt_u64(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    
    // Newton's method for integer square root
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Integer square root for u128
pub fn sqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    
    // Newton's method for integer square root
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

// safe_mul_i128 and safe_div_i128 are already generated by the macro above

/// Convert sqrt price to price using Q64 format
pub fn safe_sqrt_price_to_price(sqrt_price: u128) -> CoreResult<u128> {
    use crate::math::big_int::U256;
    
    // Use U256 for intermediate calculation to prevent overflow
    let sqrt_u256 = U256::from_u128(sqrt_price);
    let squared = sqrt_u256.mul(&sqrt_u256)
        .ok_or(FeelsCoreError::MathOverflow)?;
    
    // Divide by Q64 (2^64) to get the price
    let q64_divisor = U256::from_u128(1u128 << 64);
    let result = squared.div(&q64_divisor)
        .ok_or(FeelsCoreError::MathOverflow)?;
    
    // Check if result fits in u128
    result.to_u128().ok_or(FeelsCoreError::MathOverflow)
}

// Additional safe operations needed by fixed-point math

/// Multiplication for Q64 fixed-point
pub fn safe_mul_q64(a: u128, b: u128) -> CoreResult<u128> {
    use crate::math::big_int::U256;
    use crate::constants::Q64;
    
    let a_u256 = U256::from_u128(a);
    let b_u256 = U256::from_u128(b);
    let q64_u256 = U256::from_u128(Q64);
    
    let product = a_u256.mul(&b_u256).ok_or(FeelsCoreError::MathOverflow)?;
    let result = product.div(&q64_u256).ok_or(FeelsCoreError::DivisionByZero)?;
    
    result.to_u128().ok_or(FeelsCoreError::MathOverflow)
}

/// Division for Q64 fixed-point
pub fn safe_div_q64(a: u128, b: u128) -> CoreResult<u128> {
    use crate::math::big_int::U256;
    use crate::constants::Q64;
    
    if b == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }
    
    let a_u256 = U256::from_u128(a);
    let b_u256 = U256::from_u128(b);
    let q64_u256 = U256::from_u128(Q64);
    
    let scaled = a_u256.mul(&q64_u256).ok_or(FeelsCoreError::MathOverflow)?;
    let result = scaled.div(&b_u256).ok_or(FeelsCoreError::DivisionByZero)?;
    
    result.to_u128().ok_or(FeelsCoreError::MathOverflow)
}

/// Square root for Q64 fixed-point
pub fn safe_sqrt_q64(value: u128) -> CoreResult<u128> {
    use crate::constants::Q32;
    
    // sqrt(x * 2^64) = sqrt(x) * 2^32
    // So we need to compute sqrt and then multiply by Q32
    let sqrt_val = sqrt_u128(value);
    Ok(sqrt_val * Q32 / sqrt_u128(Q32))
}