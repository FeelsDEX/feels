//! # Tick Math
//! 
//! Conversions between ticks and sqrt prices using Q64 fixed-point precision.
//! This provides exact compatibility between on-chain and off-chain calculations.

use crate::constants::{MIN_TICK, MAX_TICK, MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64, Q64};
use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::big_int::{mul_div, U256, Rounding};

/// Magic sqrt(1.0001) powers for Q64 tick math
/// These are pre-computed values of sqrt(1.0001)^(2^i) in Q64 format
const MAGIC_SQRT_1_0001_POW_2: [u128; 20] = [
    18446808569878950912,  // 2^0
    18447134875309251715,  // 2^1  
    18447788169134960386,  // 2^2
    18449095795169968956,  // 2^3
    18452014230994128635,  // 2^4
    18458166817563198432,  // 2^5
    18471618969925737856,  // 2^6
    18499931457322659840,  // 2^7
    18558637110719970304,  // 2^8
    18679370726829963264,  // 2^9
    18928236417948540928,  // 2^10
    19451367230682021888,  // 2^11
    20605423956018225152,  // 2^12
    23325457043927080960,  // 2^13
    30423823330301186048,  // 2^14
    56566953143375020032,  // 2^15
    227809249476094689280, // 2^16
    18709476082618564266843504640, // 2^17
    2891605450058869263366914764612820992, // 2^18
    68784512281246656890519855,   // 2^19
];

/// Get sqrt price from tick using Q64 precision
pub fn get_sqrt_price_at_tick(tick: i32) -> CoreResult<u128> {
    // Validate tick range
    if tick < MIN_TICK || tick > MAX_TICK {
        return Err(FeelsCoreError::InvalidTick);
    }
    
    let abs_tick = tick.abs() as u32;
    let mut sqrt_ratio = Q64;
    
    // Binary decomposition of tick value using magic constants
    for i in 0..20 {
        if abs_tick & (1 << i) != 0 {
            // Multiply by the appropriate power of sqrt(1.0001)
            sqrt_ratio = mul_shift(sqrt_ratio, MAGIC_SQRT_1_0001_POW_2[i])?;
        }
    }
    
    // If tick is negative, invert the result
    if tick < 0 {
        sqrt_ratio = reciprocal(sqrt_ratio)?;
    }
    
    Ok(sqrt_ratio)
}

/// Get tick from sqrt price using Q64 precision
pub fn get_tick_at_sqrt_price(sqrt_price: u128) -> CoreResult<i32> {
    // Validate price range
    if sqrt_price < MIN_SQRT_PRICE_X64 || sqrt_price > MAX_SQRT_PRICE_X64 {
        return Err(FeelsCoreError::InvalidPrice);
    }
    
    // Use binary search to find the tick
    let mut low = MIN_TICK;
    let mut high = MAX_TICK;
    
    while low <= high {
        let mid = low + (high - low) / 2;
        let mid_sqrt_price = get_sqrt_price_at_tick(mid)?;
        
        if mid_sqrt_price == sqrt_price {
            return Ok(mid);
        } else if mid_sqrt_price < sqrt_price {
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    
    // Return the lower tick
    Ok(high)
}

/// Multiply two Q64 values and shift right by 64 bits
fn mul_shift(a: u128, b: u128) -> CoreResult<u128> {
    // Instead of shifting, divide by 2^64
    mul_div(
        U256::from_u128(a),
        U256::from_u128(b),
        U256::from_u128(1u128 << 64),
        Rounding::Down,
    )
    .and_then(|result| result.to_u128().ok_or(FeelsCoreError::ConversionError))
}

/// Calculate reciprocal of Q64 value (Q64^2 / value)
fn reciprocal(value: u128) -> CoreResult<u128> {
    if value == 0 {
        return Err(FeelsCoreError::DivisionByZero);
    }
    
    let q64_squared = U256::from_u128(Q64).mul(&U256::from_u128(Q64))
        .ok_or(FeelsCoreError::MathOverflow)?;
    let result = q64_squared.div(&U256::from_u128(value))
        .ok_or(FeelsCoreError::DivisionByZero)?;
    
    result.to_u128().ok_or(FeelsCoreError::ConversionError)
}

/// Check if a tick is within the supported range
pub fn is_tick_valid(tick: i32) -> bool {
    tick >= MIN_TICK && tick <= MAX_TICK
}

/// Check if a Q64 sqrt price is within the supported range
pub fn is_sqrt_price_x64_valid(sqrt_price: u128) -> bool {
    sqrt_price >= MIN_SQRT_PRICE_X64 && sqrt_price <= MAX_SQRT_PRICE_X64
}

/// Get next initialized tick
pub fn get_next_initialized_tick(
    tick: i32,
    tick_spacing: i32,
    lte: bool,
) -> i32 {
    let compressed = if lte {
        // Round down to tick spacing boundary
        let compressed = tick / tick_spacing;
        if tick < 0 && tick % tick_spacing != 0 {
            compressed - 1
        } else {
            compressed
        }
    } else {
        // Round up to tick spacing boundary
        let compressed = tick / tick_spacing;
        if tick >= 0 && tick % tick_spacing != 0 {
            compressed + 1
        } else {
            compressed
        }
    };
    
    compressed * tick_spacing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_to_sqrt_price_conversion() {
        // Test some known tick values
        let tick_0 = get_sqrt_price_at_tick(0).unwrap();
        assert_eq!(tick_0, Q64); // At tick 0, sqrt price should be Q64
        
        // Test bounds
        let min_price = get_sqrt_price_at_tick(MIN_TICK).unwrap();
        assert_eq!(min_price, MIN_SQRT_PRICE_X64);
        
        let max_price = get_sqrt_price_at_tick(MAX_TICK).unwrap();
        assert_eq!(max_price, MAX_SQRT_PRICE_X64);
    }

    #[test]
    fn test_sqrt_price_to_tick_conversion() {
        // Test round trip conversion
        for tick in &[MIN_TICK, -1000, -100, 0, 100, 1000, MAX_TICK] {
            let sqrt_price = get_sqrt_price_at_tick(*tick).unwrap();
            let recovered_tick = get_tick_at_sqrt_price(sqrt_price).unwrap();
            assert_eq!(*tick, recovered_tick);
        }
    }

    #[test]
    fn test_next_initialized_tick() {
        let spacing = 10;
        
        // Test rounding down
        assert_eq!(get_next_initialized_tick(5, spacing, true), 0);
        assert_eq!(get_next_initialized_tick(10, spacing, true), 10);
        assert_eq!(get_next_initialized_tick(-5, spacing, true), -10);
        
        // Test rounding up
        assert_eq!(get_next_initialized_tick(5, spacing, false), 10);
        assert_eq!(get_next_initialized_tick(10, spacing, false), 10);
        assert_eq!(get_next_initialized_tick(-5, spacing, false), 0);
    }
}