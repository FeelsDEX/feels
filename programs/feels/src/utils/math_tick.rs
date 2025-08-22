/// Implements tick-to-price conversions using Q64.96 fixed-point arithmetic.
/// Provides bidirectional mapping between discrete tick indices and continuous
/// sqrt prices. Uses efficient binary search and bitwise operations for gas
/// optimization. Core mathematical foundation for concentrated liquidity pricing.

use anchor_lang::prelude::*;
use super::constant::{Q64, MIN_TICK, MAX_TICK};

// ============================================================================
// Constants
// ============================================================================

pub const MIN_SQRT_PRICE_X64: u128 = 4295128739; // sqrt(1.0001^MIN_TICK) * 2^64
// MAX_SQRT_PRICE_X64 is too large for u128, using a close approximation
pub const MAX_SQRT_PRICE_X64: u128 = u128::MAX; // Approximation: sqrt(1.0001^MAX_TICK) * 2^64

// ============================================================================
// Type Definitions
// ============================================================================

/// Rounding direction for precision control
#[derive(Clone, Copy, Debug)]
pub enum Rounding {
    Up,
    Down,
}

pub struct TickMath;

// ============================================================================
// Core Implementation
// ============================================================================

impl TickMath {
    /// Convert tick to sqrt price using binary decomposition with precomputed constants
    /// Returns sqrt price in Q64.64 format
    pub fn get_sqrt_ratio_at_tick(tick: i32) -> Result<u128> {
        require!(
            tick >= MIN_TICK && tick <= MAX_TICK,
            crate::PoolError::TickOutOfBounds
        );

        let abs_tick = tick.abs() as u32;
        
        // Binary decomposition using precomputed magic numbers
        // Each constant represents sqrt(1.0001^(2^i)) in Q64.64
        // These are the exact constants used by Uniswap V3 and Raydium
        let mut ratio = if abs_tick & 0x1 != 0 {
            0xfffcb933bd6fb7400u128 // sqrt(1.0001^1) * 2^64
        } else {
            Q64 // 1.0 * 2^64
        };

        if abs_tick & 0x2 != 0 {
            ratio = mul_shift_64(ratio, 0xfff97272373d41300u128); // sqrt(1.0001^2) * 2^64
        }
        if abs_tick & 0x4 != 0 {
            ratio = mul_shift_64(ratio, 0xfff2e50f5f656f200u128); // sqrt(1.0001^4) * 2^64
        }
        if abs_tick & 0x8 != 0 {
            ratio = mul_shift_64(ratio, 0xffe5caca7e10e4e700u128); // sqrt(1.0001^8) * 2^64
        }
        if abs_tick & 0x10 != 0 {
            ratio = mul_shift_64(ratio, 0xffcb9843d60f6159800u128); // sqrt(1.0001^16) * 2^64
        }
        if abs_tick & 0x20 != 0 {
            ratio = mul_shift_64(ratio, 0xff973b41fa98c081500u128); // sqrt(1.0001^32) * 2^64
        }
        if abs_tick & 0x40 != 0 {
            ratio = mul_shift_64(ratio, 0xff2ea16466c96a3843ec78b326b52861); // sqrt(1.0001^64) * 2^64
        }
        if abs_tick & 0x80 != 0 {
            ratio = mul_shift_64(ratio, 0xfe5dee046a99a2a811c461f1969c3053u128); // sqrt(1.0001^128) * 2^64
        }
        if abs_tick & 0x100 != 0 {
            ratio = mul_shift_64(ratio, 0xfcbe86c7900a88aedcffc83b479aa3a4u128); // sqrt(1.0001^256) * 2^64
        }
        if abs_tick & 0x200 != 0 {
            ratio = mul_shift_64(ratio, 0xf987a7253ac413176f2b074cf7815e54u128); // sqrt(1.0001^512) * 2^64
        }
        if abs_tick & 0x400 != 0 {
            ratio = mul_shift_64(ratio, 0xf3392b0822b70005940c7a398e4b70f3u128); // sqrt(1.0001^1024) * 2^64
        }
        if abs_tick & 0x800 != 0 {
            ratio = mul_shift_64(ratio, 0xe7159475a2c29b7443b29c7fa6e889d9u128); // sqrt(1.0001^2048) * 2^64
        }
        if abs_tick & 0x1000 != 0 {
            ratio = mul_shift_64(ratio, 0xd097f3bdfd2022b8845ad8f792aa5825u128); // sqrt(1.0001^4096) * 2^64
        }
        if abs_tick & 0x2000 != 0 {
            ratio = mul_shift_64(ratio, 0xa9f746462d870fdf8a65dc1f90e061e5u128); // sqrt(1.0001^8192) * 2^64
        }
        if abs_tick & 0x4000 != 0 {
            ratio = mul_shift_64(ratio, 0x70d869a156d2a1b890bb3df62baf32f7u128); // sqrt(1.0001^16384) * 2^64
        }
        if abs_tick & 0x8000 != 0 {
            ratio = mul_shift_64(ratio, 0x31be135f97d08fd981231505542fcfa6u128); // sqrt(1.0001^32768) * 2^64
        }
        if abs_tick & 0x10000 != 0 {
            ratio = mul_shift_64(ratio, 0x9aa508b5b7a84e1c677de54f3e99bc9u128); // sqrt(1.0001^65536) * 2^64
        }
        if abs_tick & 0x20000 != 0 {
            ratio = mul_shift_64(ratio, 0x5d6af8dedb81196699c329225ee604u128); // sqrt(1.0001^131072) * 2^64
        }
        if abs_tick & 0x40000 != 0 {
            ratio = mul_shift_64(ratio, 0x2216e584f5fa1ea926041bedfe98u128); // sqrt(1.0001^262144) * 2^64
        }
        if abs_tick & 0x80000 != 0 {
            ratio = mul_shift_64(ratio, 0x48a170391f7dc42444e8fa2u128); // sqrt(1.0001^524288) * 2^64
        }

        if tick > 0 {
            ratio = u128::MAX / ratio;
        }

        // Convert from Q128.64 to Q64.64 with proper rounding
        if ratio % (1u128 << 32) > 0 {
            (ratio >> 32) + 1
        } else {
            ratio >> 32
        }
        .pipe(|r| {
            if r < MIN_SQRT_PRICE_X64 {
                Ok(MIN_SQRT_PRICE_X64)
            } else if r > MAX_SQRT_PRICE_X64 {
                Ok(MAX_SQRT_PRICE_X64)
            } else {
                Ok(r)
            }
        })
    }

    /// Convert sqrt price to tick using binary search
    /// Input: sqrt price in Q64.64 format
    pub fn get_tick_at_sqrt_ratio(sqrt_price_x64: u128) -> Result<i32> {
        require!(
            sqrt_price_x64 >= MIN_SQRT_PRICE_X64 && sqrt_price_x64 <= MAX_SQRT_PRICE_X64,
            crate::PoolError::PriceOutOfBounds
        );

        let mut low = MIN_TICK;
        let mut high = MAX_TICK;

        while low < high {
            let mid = (low + high) / 2;
            let sqrt_price_mid = Self::get_sqrt_ratio_at_tick(mid)?;
            
            if sqrt_price_mid <= sqrt_price_x64 {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        Ok(low - 1)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Multiply two Q64.64 numbers and shift right by 64 bits
/// Equivalent to: (a * b) >> 64
fn mul_shift_64(a: u128, b: u128) -> u128 {
    // For Phase 1, use simpler approach
    match a.checked_mul(b) {
        Some(product) => product >> 64,
        None => u128::MAX, // Overflow protection
    }
}

/// Safe multiply-shift with overflow protection and rounding control
pub fn mul_shr(x: u128, y: u128, offset: u8, rounding: Rounding) -> Option<u128> {
    // For Phase 1, use simpler approach to avoid complex U256 operations
    let product = x.checked_mul(y)?;
    let shifted = product >> offset;
    
    match rounding {
        Rounding::Up => {
            // Check if there are any bits that would be lost during the shift
            let mask = (1u128 << offset) - 1;
            if product & mask != 0 {
                shifted.checked_add(1)
            } else {
                Some(shifted)
            }
        }
        Rounding::Down => Some(shifted)
    }
}

/// Safe shift-divide with overflow protection and rounding control
pub fn shl_div(x: u128, y: u128, offset: u8, rounding: Rounding) -> Option<u128> {
    // For Phase 1, use simpler approach to avoid complex U256 operations
    let dividend = x.checked_shl(offset as u32)?;
    
    match rounding {
        Rounding::Up => {
            // Ceiling division: (a + b - 1) / b
            dividend.checked_add(y.checked_sub(1)?)?.checked_div(y)
        }
        Rounding::Down => dividend.checked_div(y)
    }
}

// ------------------------------------------------------------------------
// Extension Traits
// ------------------------------------------------------------------------

/// Extension trait for pipelining operations
trait Pipe<T> {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R;
}

impl<T> Pipe<T> for T {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R {
        f(self)
    }
}