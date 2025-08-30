/// Unified mathematics module for the Feels Protocol.
///
/// This module consolidates all mathematical operations required for concentrated
/// liquidity AMM operations, providing a single source of truth for calculations.
///
/// # Architecture
///
/// The module is organized into logical sub-modules:
/// - `big_int`: 256-bit and 512-bit integer operations using `ruint`
/// - `safe`: Overflow-safe arithmetic operations
/// - `amm`: AMM-specific calculations (tick math, liquidity, fees)
/// - `q96`: Q96 fixed-point arithmetic using `spl-math`
///
/// # Design Principles
///
/// 1. **Safety First**: All operations use checked arithmetic or proven libraries
/// 2. **Q-Format Strategy**: External APIs use Q96, internal calculations use Q64
/// 3. **Battle-Tested**: Leverages Orca's algorithms and established libraries
/// 4. **Performance**: Optimized for Solana's compute constraints
///
/// # Q-Format Strategy
///
/// This module implements a hybrid Q96/Q64 fixed-point arithmetic strategy:
///
/// ## Q96 Format (External API)
/// - Used for all public interfaces to maintain compatibility with Uniswap V3
/// - Provides 96 bits of fractional precision (2^96 ≈ 7.9 × 10^28)
/// - Sqrt prices are represented as: `sqrt_price_x96 = sqrt(price) × 2^96`
///
/// ## Q64 Format (Internal Calculations)
/// - Used internally for calculations, following Orca's Whirlpools implementation
/// - More efficient on Solana's 64-bit architecture
/// - Provides sufficient precision while reducing compute costs
/// - Sqrt prices are represented as: `sqrt_price_x64 = sqrt(price) × 2^64`
///
/// ## Conversion Strategy
/// - All public functions accept Q96 inputs and return Q96 outputs
/// - Internal calculations convert Q96 → Q64 → perform math → Q64 → Q96
/// - Conversion functions: `q96_to_q64()` and `q64_to_q96()`
///
/// # Library Choices
///
/// ## ruint for U256/U512
/// - Production-grade 256/512-bit unsigned integer library
/// - Used for overflow-safe multiplication and division
/// - Essential for fee growth calculations in Q128.128 format
///
/// ## spl-math PreciseNumber
/// - Solana's official high-precision decimal math library
/// - Used for Q96 fixed-point calculations that exceed u128 range
/// - Provides automatic scaling and rounding
///
/// # Mathematical Foundations
///
/// ## Tick Math (Adapted from Orca)
/// The tick-to-price conversion uses a fast exponential algorithm that decomposes
/// ticks into binary components. For a tick `t`, the sqrt price is calculated as:
///
/// ```text
/// sqrt_price = sqrt(1.0001^t) × 2^96
/// ```
///
/// The implementation uses precomputed constants for powers of sqrt(1.0001) and
/// applies them based on the binary representation of the tick. This approach
/// minimizes compute units while maintaining precision.
///
/// ## Fee Growth Tracking
/// Fees are tracked using Q128.128 fixed-point format (256 bits total):
/// - Upper 128 bits: Integer part
/// - Lower 128 bits: Fractional part
///
/// This allows tracking fee growth as small as 2^-128 per unit liquidity,
/// ensuring fair distribution even for minimal fees across large liquidity pools.
// Re-export commonly used types
pub use ruint::aliases::{U256, U512};
// pub use spl_math::precise_number::PreciseNumber; // Commented out due to global allocator conflict

// ============================================================================
// Sub-modules
// ============================================================================

/// Big integer arithmetic using the production-grade `ruint` library
pub mod big_int {
    use super::*;

    /// Rounding modes for financial calculations
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Rounding {
        Down, // Floor - round towards zero
        Up,   // Ceiling - round away from zero
    }

    /// Multiply two U256 values and divide by a third, with rounding
    pub fn mul_div(
        a: U256,
        numerator: U256,
        denominator: U256,
        rounding: Rounding,
    ) -> Option<U256> {
        if denominator.is_zero() {
            return None;
        }

        // Use U512 for intermediate calculation to prevent overflow
        let product = U512::from(a) * U512::from(numerator);

        // Perform division
        let (quotient, remainder) = product.div_rem(U512::from(denominator));

        // Check if quotient fits in U256
        let quotient_bytes: [u8; 64] = quotient.to_le_bytes();
        // Check if upper 32 bytes are zero
        for byte in quotient_bytes.iter().skip(32) {
            if *byte != 0 {
                return None; // Overflow
            }
        }
        // Safe to convert lower 32 bytes to U256
        let mut u256_bytes = [0u8; 32];
        u256_bytes.copy_from_slice(&quotient_bytes[0..32]);
        let quotient_u256 = U256::from_le_bytes(u256_bytes);

        // Apply rounding if necessary
        if rounding == Rounding::Up && !remainder.is_zero() {
            quotient_u256.checked_add(U256::from(1))
        } else {
            Some(quotient_u256)
        }
    }

    /// Multiply two U256 values and divide by a third, rounding up
    pub fn mul_div_rounding_up(a: U256, b: U256, c: U256) -> Option<U256> {
        mul_div(a, b, c, Rounding::Up)
    }

    /// Convert [u64; 4] little-endian words to U256
    #[inline]
    pub fn words_to_u256(words: [u64; 4]) -> U256 {
        let mut bytes = [0u8; 32];
        for (i, &word) in words.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&word.to_le_bytes());
        }
        U256::from_le_bytes(bytes)
    }

    /// Convert U256 to [u64; 4] little-endian words
    #[inline]
    pub fn u256_to_words(value: U256) -> [u64; 4] {
        let bytes: [u8; 32] = value.to_le_bytes();
        let mut words = [0u64; 4];
        for i in 0..4 {
            words[i] = u64::from_le_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        words
    }
}

/// Safe arithmetic operations with overflow protection
pub mod safe {
    use crate::state::FeelsProtocolError;
    use anchor_lang::prelude::Result;

    /// Safe addition for u64 values
    pub fn add_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_add(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }

    /// Safe subtraction for u64 values
    pub fn sub_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_sub(b)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow.into())
    }

    /// Safe multiplication for u64 values
    pub fn mul_u64(a: u64, b: u64) -> Result<u64> {
        a.checked_mul(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }

    /// Safe division for u64 values
    pub fn div_u64(a: u64, b: u64) -> Result<u64> {
        if b == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }
        a.checked_div(b).ok_or(FeelsProtocolError::DivisionByZero.into())
    }

    /// Safe addition for u128 values
    pub fn add_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_add(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }

    /// Safe subtraction for u128 values
    pub fn sub_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_sub(b)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow.into())
    }

    /// Safe multiplication for u128 values
    pub fn mul_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_mul(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }

    /// Safe division for u128 values
    pub fn div_u128(a: u128, b: u128) -> Result<u128> {
        if b == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }
        a.checked_div(b).ok_or(FeelsProtocolError::DivisionByZero.into())
    }

    /// Safe addition for i128 values
    pub fn add_i128(a: i128, b: i128) -> Result<i128> {
        a.checked_add(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }

    /// Safe subtraction for i128 values
    pub fn sub_i128(a: i128, b: i128) -> Result<i128> {
        a.checked_sub(b)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow.into())
    }

    /// Add a signed liquidity delta to an unsigned liquidity value
    pub fn add_liquidity_delta(liquidity: u128, delta: i128) -> Result<u128> {
        if delta >= 0 {
            liquidity
                .checked_add(delta as u128)
                .ok_or(FeelsProtocolError::LiquidityOverflow.into())
        } else {
            liquidity
                .checked_sub((-delta) as u128)
                .ok_or(FeelsProtocolError::LiquidityUnderflow.into())
        }
    }

    /// Subtract a signed liquidity delta from an unsigned liquidity value
    pub fn sub_liquidity_delta(liquidity: u128, delta: i128) -> Result<u128> {
        if delta >= 0 {
            liquidity
                .checked_sub(delta as u128)
                .ok_or(FeelsProtocolError::LiquidityUnderflow.into())
        } else {
            liquidity
                .checked_add((-delta) as u128)
                .ok_or(FeelsProtocolError::LiquidityOverflow.into())
        }
    }

    /// Calculate percentage with basis points (10000 = 100%)
    pub fn calculate_percentage(amount: u64, basis_points: u16) -> Result<u64> {
        const BASIS_POINTS_DIVISOR: u64 = 10_000;

        if basis_points > BASIS_POINTS_DIVISOR as u16 {
            return Err(FeelsProtocolError::InvalidPercentage.into());
        }

        mul_div_u64(amount, basis_points as u64, BASIS_POINTS_DIVISOR)
    }

    /// Integer square root for u64 (Newton's method)
    pub fn sqrt_u64(n: u64) -> u64 {
        if n == 0 {
            return 0;
        }

        let mut x = n;
        let mut y = (x + 1) / 2;

        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }

        x
    }

    /// Integer square root for u128 (Newton's method)
    pub fn sqrt_u128(n: u128) -> Result<u128> {
        if n == 0 {
            return Ok(0);
        }

        let mut x = n;
        let mut y = (x + 1) / 2;

        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }

        Ok(x)
    }

    /// Multiply two u64 values and divide by a third
    pub fn mul_div_u64(a: u64, b: u64, denominator: u64) -> Result<u64> {
        if denominator == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }

        let product = (a as u128) * (b as u128);
        let result = product / (denominator as u128);

        if result > u64::MAX as u128 {
            return Err(FeelsProtocolError::MathOverflow.into());
        }

        Ok(result as u64)
    }
}

/// Q96 fixed-point arithmetic using spl-math
pub mod q96 {
    use super::*;
    use crate::state::FeelsProtocolError;
    use anchor_lang::prelude::*;

    /// Q96 constant: 2^96
    pub const Q96: u128 = 1u128 << 96;

    /*
    /// Convert a Q96 value stored in U256 to PreciseNumber for calculations
    pub fn q96_to_precise(value: U256) -> Result<PreciseNumber> {
        // First, check if the value fits in u128 (required for PreciseNumber)
        let value_u128: u128 = value.try_into()
            .map_err(|_| FeelsProtocolError::MathOverflow)?;

        // Convert to PreciseNumber
        let actual_value = value_u128 / Q96;
        let remainder = value_u128 % Q96;

        // Create PreciseNumber from the integer part
        let mut precise = PreciseNumber::new(actual_value)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        // Add the fractional part if non-zero
        if remainder > 0 {
            let remainder_precise = PreciseNumber::new(remainder)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            let q96_precise = PreciseNumber::new(Q96)
                .ok_or(FeelsProtocolError::DivisionByZero)?;
            let fraction = remainder_precise.checked_div(&q96_precise)
                .ok_or(FeelsProtocolError::DivisionByZero)?;

            precise = precise.checked_add(&fraction)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }

        Ok(precise)
    }
    */

    /// Calculate fee growth in Q128.128 format
    pub fn calculate_fee_growth_q128(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
        if liquidity == 0 {
            return Err(FeelsProtocolError::InvalidLiquidity.into());
        }

        // For Q128.128 scaling, we use U256
        let fee_u256 = U256::from(fee_amount);
        let fee_shifted = fee_u256.checked_shl(128).ok_or(FeelsProtocolError::MathOverflow)?;
        let result = fee_shifted
            .checked_div(U256::from(liquidity))
            .ok_or(FeelsProtocolError::DivisionByZero)?;

        // Convert U256 to [u64; 4]
        let result_bytes: [u8; 32] = result.to_le_bytes();
        let mut words = [0u64; 4];
        for i in 0..4 {
            words[i] = u64::from_le_bytes(result_bytes[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        Ok(words)
    }
}

/// AMM-specific calculations including tick math, fees, and liquidity
pub mod amm {
    use super::big_int::{mul_div, mul_div_rounding_up, Rounding};
    use super::q96::calculate_fee_growth_q128;
    use super::safe::mul_div_u64;
    use super::*;
    use crate::constant::{BASIS_POINTS_DENOMINATOR, MAX_PROTOCOL_FEE_RATE, MAX_TICK, MIN_TICK};
    use crate::state::FeelsProtocolError;
    use anchor_lang::prelude::*;

    // ============================================================================
    // Constants
    // ============================================================================

    /// Sqrt price constants in Q96 format
    /// These are the sqrt price values at the min/max ticks
    /// Derived from Orca's Q64 constants
    pub const MIN_SQRT_PRICE_X96: u128 = 18447090763469684736; // Actual value for tick -443636
    pub const MAX_SQRT_PRICE_X96: u128 = 340_275_971_719_517_849_884_101_479_037_289_023_427; // Actual value for tick 443636

    // ============================================================================
    // Tick-Price Conversion Mathematics
    // ============================================================================

    /// Helper functions for Q96/Q64 conversions
    ///
    /// The codebase uses Q96 format for external APIs (following Uniswap V3)
    /// but converts to Q64 internally for some calculations (adapting Orca's algorithms).
    /// This hybrid approach provides:
    /// - Q96: Higher precision for price representation
    /// - Q64: More efficient calculations on Solana
    #[inline]
    pub fn q96_to_q64(value: u128) -> u128 {
        value >> 32
    }

    #[inline]
    pub fn q64_to_q96(value: u128) -> u128 {
        value << 32
    }

    /// Tick math operations for concentrated liquidity
    pub struct TickMath;

    impl TickMath {
        /// Get the sqrt price as a Q96 number for a given tick
        ///
        /// This function converts a tick value to its corresponding sqrt price using
        /// the formula: sqrt_price = sqrt(1.0001^tick) × 2^96
        ///
        /// The implementation uses a fast exponential algorithm that decomposes the
        /// tick into binary components and applies precomputed powers of sqrt(1.0001).
        ///
        /// # Parameters
        /// - `tick`: The tick value, must be within [MIN_TICK, MAX_TICK]
        ///
        /// # Returns
        /// The sqrt price in Q96 format
        ///
        /// # Example
        /// ```
        /// use feels::utils::math::amm::TickMath;
        ///
        /// let sqrt_price = TickMath::get_sqrt_ratio_at_tick(0).unwrap();
        /// assert_eq!(sqrt_price, 79228162514264337593543950336); // 2^96 (price = 1.0)
        /// ```
        pub fn get_sqrt_ratio_at_tick(tick: i32) -> Result<u128> {
            require!(
                (MIN_TICK..=MAX_TICK).contains(&tick),
                FeelsProtocolError::TickOutOfBounds
            );

            Ok(if tick >= 0 {
                Self::get_sqrt_price_positive_tick(tick)
            } else {
                Self::get_sqrt_price_negative_tick(tick)
            })
        }

        /// Performs the exponential conversion for positive ticks.
        ///
        /// This is a direct adaptation from Orca's Whirlpools implementation,
        /// modified to output Q96 format instead of Q64.
        ///
        /// # Algorithm
        ///
        /// The algorithm decomposes the tick into its binary representation and
        /// applies precomputed powers of sqrt(1.0001) for each set bit. This is
        /// mathematically equivalent to:
        ///
        /// ```text
        /// sqrt_price = sqrt(1.0001^tick) × 2^96
        /// ```
        ///
        /// But computed efficiently using only multiplication and bit shifts.
        ///
        /// # Source
        ///
        /// Adapted from: https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/math/tick_math.rs
        fn get_sqrt_price_positive_tick(tick: i32) -> u128 {
            let mut ratio: u128 = if tick & 1 != 0 {
                79232123823359799118286999567 // sqrt(1.0001) in Q96 * 2^-32
            } else {
                79228162514264337593543950336 // 1.0 in Q96 * 2^-32
            };

            // Apply powers of sqrt(1.0001) for each bit set in tick
            if tick & 2 != 0 {
                ratio = Self::mul_shift_96(ratio, 79236085330515764027303304731);
            }
            if tick & 4 != 0 {
                ratio = Self::mul_shift_96(ratio, 79244008939048815603706035061);
            }
            if tick & 8 != 0 {
                ratio = Self::mul_shift_96(ratio, 79259858533276714757314932305);
            }
            if tick & 16 != 0 {
                ratio = Self::mul_shift_96(ratio, 79291567232598584799939703904);
            }
            if tick & 32 != 0 {
                ratio = Self::mul_shift_96(ratio, 79355022692464371645785046466);
            }
            if tick & 64 != 0 {
                ratio = Self::mul_shift_96(ratio, 79482085999252804386437311141);
            }
            if tick & 128 != 0 {
                ratio = Self::mul_shift_96(ratio, 79736823300114093921829183326);
            }
            if tick & 256 != 0 {
                ratio = Self::mul_shift_96(ratio, 80248749790819932309965073892);
            }
            if tick & 512 != 0 {
                ratio = Self::mul_shift_96(ratio, 81282483887344747381513967011);
            }
            if tick & 1024 != 0 {
                ratio = Self::mul_shift_96(ratio, 83390072131320151908154831281);
            }
            if tick & 2048 != 0 {
                ratio = Self::mul_shift_96(ratio, 87770609709833776024991924138);
            }
            if tick & 4096 != 0 {
                ratio = Self::mul_shift_96(ratio, 97234110755111693312479820773);
            }
            if tick & 8192 != 0 {
                ratio = Self::mul_shift_96(ratio, 119332217159966728226237229890);
            }
            if tick & 16384 != 0 {
                ratio = Self::mul_shift_96(ratio, 179736315981702064433883588727);
            }
            if tick & 32768 != 0 {
                ratio = Self::mul_shift_96(ratio, 407748233172238350107850275304);
            }
            if tick & 65536 != 0 {
                ratio = Self::mul_shift_96(ratio, 2098478828474011932436660412517);
            }
            if tick & 131072 != 0 {
                ratio = Self::mul_shift_96(ratio, 55581415166113811149459800483533);
            }
            if tick & 262144 != 0 {
                ratio = Self::mul_shift_96(ratio, 38992368544603139932233054999993551);
            }

            ratio // Already in Q96 format
        }

        /// Helper for multiplying and right-shifting in negative tick calculation
        #[inline(always)]
        fn mul_shift_64(a: u128, b: u128) -> u128 {
            let result: U256 = (U256::from(a) * U256::from(b)) >> 64;
            result.try_into().unwrap()
        }

        /// Performs the exponential conversion for negative ticks.
        ///
        /// For negative ticks, we calculate the reciprocal: 1/sqrt(1.0001^|tick|).
        /// This uses the same binary decomposition approach but with reciprocal
        /// constants and right shifts instead of multiplication.
        ///
        /// # Q-Format Note
        ///
        /// Orca's implementation returns Q64 format for negative ticks. We add
        /// a final left shift by 32 to convert to Q96 for API consistency.
        ///
        /// # Source
        ///
        /// Adapted from: https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/math/tick_math.rs
        fn get_sqrt_price_negative_tick(tick: i32) -> u128 {
            let abs_tick = tick.abs();

            // Use Orca's Q64 constants for negative ticks
            let mut ratio: u128 = if abs_tick & 1 != 0 {
                18445821805675392311 // From Orca's implementation
            } else {
                18446744073709551616 // 2^64 (1.0 in Q64)
            };

            if abs_tick & 2 != 0 {
                ratio = Self::mul_shift_64(ratio, 18444899583751176498)
            }
            if abs_tick & 4 != 0 {
                ratio = Self::mul_shift_64(ratio, 18443055278223354162)
            }
            if abs_tick & 8 != 0 {
                ratio = Self::mul_shift_64(ratio, 18439367220385604838)
            }
            if abs_tick & 16 != 0 {
                ratio = Self::mul_shift_64(ratio, 18431993317065449817)
            }
            if abs_tick & 32 != 0 {
                ratio = Self::mul_shift_64(ratio, 18417254355718160513)
            }
            if abs_tick & 64 != 0 {
                ratio = Self::mul_shift_64(ratio, 18387811781193591352)
            }
            if abs_tick & 128 != 0 {
                ratio = Self::mul_shift_64(ratio, 18329067761203520168)
            }
            if abs_tick & 256 != 0 {
                ratio = Self::mul_shift_64(ratio, 18212142134806087854)
            }
            if abs_tick & 512 != 0 {
                ratio = Self::mul_shift_64(ratio, 17980523815641551639)
            }
            if abs_tick & 1024 != 0 {
                ratio = Self::mul_shift_64(ratio, 17526086738831147013)
            }
            if abs_tick & 2048 != 0 {
                ratio = Self::mul_shift_64(ratio, 16651378430235024244)
            }
            if abs_tick & 4096 != 0 {
                ratio = Self::mul_shift_64(ratio, 15030750278693429944)
            }
            if abs_tick & 8192 != 0 {
                ratio = Self::mul_shift_64(ratio, 12247334978882834399)
            }
            if abs_tick & 16384 != 0 {
                ratio = Self::mul_shift_64(ratio, 8131365268884726200)
            }
            if abs_tick & 32768 != 0 {
                ratio = Self::mul_shift_64(ratio, 3584323654723342297)
            }
            if abs_tick & 65536 != 0 {
                ratio = Self::mul_shift_64(ratio, 696457651847595233)
            }
            if abs_tick & 131072 != 0 {
                ratio = Self::mul_shift_64(ratio, 26294789957452057)
            }
            if abs_tick & 262144 != 0 {
                ratio = Self::mul_shift_64(ratio, 37481735321082)
            }

            // Orca returns Q64 for negative ticks, we need Q96
            ratio << 32
        }

        /// Get the tick value for a given sqrt price ratio
        ///
        /// This function performs the inverse operation of `get_sqrt_ratio_at_tick`,
        /// converting a sqrt price back to its corresponding tick value.
        ///
        /// The algorithm uses logarithmic approximation:
        /// 1. Calculate log base 2 of the sqrt price
        /// 2. Convert to log base sqrt(1.0001)
        /// 3. Apply error bounds and verify the result
        ///
        /// # Parameters
        /// - `sqrt_ratio_x96`: The sqrt price in Q96 format
        ///
        /// # Returns
        /// The tick value corresponding to the given sqrt price
        ///
        /// # Precision
        /// The function guarantees that the returned tick, when converted back to
        /// a sqrt price, will be the largest tick whose sqrt price is ≤ the input.
        pub fn get_tick_at_sqrt_ratio(sqrt_ratio_x96: u128) -> Result<i32> {
            require!(
                (MIN_SQRT_PRICE_X96..=MAX_SQRT_PRICE_X96).contains(&sqrt_ratio_x96),
                FeelsProtocolError::PriceOutOfBounds
            );

            // Convert from Q96 to Q64 for Orca's algorithm
            // This conversion is safe and maintains precision for tick calculations
            let sqrt_price_x64 = q96_to_q64(sqrt_ratio_x96);

            // Determine log_b(sqrt_ratio). First by calculating integer portion (msb)
            let msb: u32 = 128 - sqrt_price_x64.leading_zeros() - 1;
            let log2p_integer_x32 = (msb as i128 - 64) << 32;

            // Get fractional value (r/2^msb), msb always > 128
            // We begin the iteration from bit 63 (0.5 in Q64.64)
            let mut bit: i128 = 0x8000_0000_0000_0000i128;
            let mut precision = 0;
            let mut log2p_fraction_x64 = 0;

            // Log2 iterative approximation for the fractional part
            // Go through each 2^(j) bit where j < 64 in a Q64.64 number
            // Append current bit value to fraction result if r^2 Q2.126 is more than 2
            let mut r = if msb >= 64 {
                sqrt_price_x64 >> (msb - 63)
            } else {
                sqrt_price_x64 << (63 - msb)
            };

            const BIT_PRECISION: u32 = 14;
            while bit > 0 && precision < BIT_PRECISION {
                r *= r;
                let is_r_more_than_two = r >> 127_u32;
                r >>= 63 + is_r_more_than_two;
                log2p_fraction_x64 += bit * is_r_more_than_two as i128;
                bit >>= 1;
                precision += 1;
            }

            let log2p_fraction_x32 = log2p_fraction_x64 >> 32;
            let log2p_x32 = log2p_integer_x32 + log2p_fraction_x32;

            // Transform from base 2 to base b
            const LOG_B_2_X32: i128 = 59543866431248i128;
            let logbp_x64 = log2p_x32 * LOG_B_2_X32;

            // Derive tick_low & high estimate. Adjust with the possibility of
            // under-estimating by 2^precision_bits/log_2(b) + 0.01 error margin.
            const LOG_B_P_ERR_MARGIN_LOWER_X64: i128 = 184467440737095516i128; // 0.01
            const LOG_B_P_ERR_MARGIN_UPPER_X64: i128 = 15793534762490258745i128; // 2^-precision / log_2_b + 0.01

            let tick_low: i32 = ((logbp_x64 - LOG_B_P_ERR_MARGIN_LOWER_X64) >> 64) as i32;
            let tick_high: i32 = ((logbp_x64 + LOG_B_P_ERR_MARGIN_UPPER_X64) >> 64) as i32;

            if tick_low == tick_high {
                Ok(tick_low.clamp(MIN_TICK, MAX_TICK))
            } else {
                // If our estimation for tick_high returns a lower sqrt_price than the input
                // then the actual tick_high has to be higher than tick_high.
                // Otherwise, the actual value is between tick_low & tick_high, so a floor value
                // (tick_low) is returned
                let actual_tick_high_sqrt_price =
                    Self::get_sqrt_ratio_at_tick(tick_high.min(MAX_TICK))?;
                if actual_tick_high_sqrt_price <= sqrt_ratio_x96 {
                    Ok(tick_high.clamp(MIN_TICK, MAX_TICK))
                } else {
                    Ok(tick_low.clamp(MIN_TICK, MAX_TICK))
                }
            }
        }

        /// Helper function for multiplication followed by division by 2^96
        /// This maintains Q96 format after multiplication
        /// Adapted from Orca's mul_shift_96
        fn mul_shift_96(n0: u128, n1: u128) -> u128 {
            let product = U256::from(n0).checked_mul(U256::from(n1)).unwrap();
            let result: U256 = product >> 96;
            result.try_into().unwrap()
        }
    }

    // ============================================================================
    // Fee Mathematics
    // ============================================================================

    /// Fee breakdown structure for transparent fee calculation
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FeeBreakdown {
        pub total_fee: u64,
        pub lp_fee: u64,
        pub protocol_fee: u64,
        pub liquidity_fee: u64,  // Alias for lp_fee for compatibility
    }

    /// Fee growth tracking using Q128.128 fixed-point arithmetic
    pub struct FeeGrowthMath;

    impl FeeGrowthMath {
        /// Add two fee growth values with overflow protection
        pub fn add_fee_growth(a: [u64; 4], b: [u64; 4]) -> Result<[u64; 4]> {
            use super::big_int::{u256_to_words, words_to_u256};

            let a_u256 = words_to_u256(a);
            let b_u256 = words_to_u256(b);

            let result = a_u256.checked_add(b_u256).ok_or(FeelsProtocolError::MathOverflow)?;

            Ok(u256_to_words(result))
        }

        /// Subtract two fee growth values with underflow protection
        pub fn sub_fee_growth(a: [u64; 4], b: [u64; 4]) -> Result<[u64; 4]> {
            use super::big_int::{u256_to_words, words_to_u256};

            let a_u256 = words_to_u256(a);
            let b_u256 = words_to_u256(b);

            let result = a_u256
                .checked_sub(b_u256)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;

            Ok(u256_to_words(result))
        }

        /// Calculate fee growth delta from amount and liquidity
        ///
        /// Fee growth is tracked in Q128.128 fixed-point format to maintain precision
        /// for very small fee amounts. This format uses 128 bits for the integer part
        /// and 128 bits for the fractional part, stored as a 256-bit number.
        ///
        /// Formula: fee_growth = (fee_amount × 2^128) / liquidity
        ///
        /// # Parameters
        /// - `fee_amount`: The fee amount to distribute
        /// - `liquidity`: The liquidity to distribute fees across
        ///
        /// # Returns
        /// Fee growth in Q128.128 format as [u64; 4] (256 bits total)
        ///
        /// # Precision
        /// Q128.128 format allows tracking fee growth as small as 2^-128 per unit liquidity,
        /// ensuring fair distribution even for minimal fees across large liquidity pools.
        pub fn calculate_fee_growth(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
            calculate_fee_growth_q128(fee_amount, liquidity)
        }

        /// Convert fee amount to fee growth (alias for calculate_fee_growth)
        pub fn fee_to_fee_growth(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
            Self::calculate_fee_growth(fee_amount, liquidity)
        }
    }

    /// General fee calculation utilities
    pub struct FeeMath;

    impl FeeMath {
        /// Validate fee rate is within allowed bounds
        pub fn validate_fee_rate(fee_rate: u16) -> Result<()> {
            use crate::constant::MAX_FEE_RATE;
            require!(fee_rate <= MAX_FEE_RATE, FeelsProtocolError::InvalidFeeRate);
            Ok(())
        }

        /// Calculate complete fee breakdown for a swap
        pub fn calculate_swap_fees(
            amount_in: u64,
            fee_rate: u16,
            protocol_fee_rate: u16,
        ) -> Result<FeeBreakdown> {
            // Calculate total fee first
            let total_fee = Self::calculate_total_fee(amount_in, fee_rate)?;

            // Calculate protocol's share of the fee
            let protocol_fee = mul_div_u64(
                total_fee,
                protocol_fee_rate as u64,
                BASIS_POINTS_DENOMINATOR as u64,
            )?;

            // LP fee is the remainder
            let lp_fee = total_fee
                .checked_sub(protocol_fee)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;

            Ok(FeeBreakdown {
                total_fee,
                lp_fee,
                protocol_fee,
                liquidity_fee: lp_fee,  // Set liquidity_fee same as lp_fee
            })
        }

        /// Calculate just the total fee amount
        pub fn calculate_total_fee(amount_in: u64, fee_rate: u16) -> Result<u64> {
            // Validate fee rate
            require!(
                fee_rate <= BASIS_POINTS_DENOMINATOR as u16,
                FeelsProtocolError::InvalidFeeRate
            );

            // fee = amount_in * fee_rate / 10000
            // Use mul_div_u64 for precise calculation
            mul_div_u64(amount_in, fee_rate as u64, BASIS_POINTS_DENOMINATOR as u64)
        }

        /// Calculate effective fee rate (placeholder for dynamic fees)
        pub fn calculate_effective_fee_rate(
            base_fee_rate: u16,
            _volume: u128,
            _volatility: u32,
        ) -> Result<u16> {
            // Phase 1: Return base fee rate
            // Phase 2+: Implement dynamic fee based on volume and volatility
            Ok(base_fee_rate)
        }
    }

    /// Fee configuration utility
    pub struct FeeConfig;

    impl FeeConfig {
        /// Get tick spacing for a given fee tier
        pub fn get_tick_spacing_for_fee(fee_rate: u16) -> Result<i16> {
            match fee_rate {
                1 => Ok(1),     // 0.01% = 1 tick spacing
                5 => Ok(10),    // 0.05% = 10 tick spacing
                30 => Ok(60),   // 0.30% = 60 tick spacing
                100 => Ok(200), // 1.00% = 200 tick spacing
                _ => Err(FeelsProtocolError::InvalidFeeRate.into()),
            }
        }

        /// Validate pool fee configuration
        pub fn validate_pool_fees(
            fee_rate: u16,
            protocol_fee_rate: u16,
            tick_spacing: i16,
        ) -> Result<()> {
            // Validate fee rate is one of the allowed tiers
            require!(
                matches!(fee_rate, 1 | 5 | 30 | 100),
                FeelsProtocolError::InvalidFeeRate
            );

            // Validate protocol fee doesn't exceed maximum
            require!(
                protocol_fee_rate <= MAX_PROTOCOL_FEE_RATE,
                FeelsProtocolError::InvalidPercentage
            );

            // Validate tick spacing matches fee tier
            let expected_spacing = Self::get_tick_spacing_for_fee(fee_rate)?;
            require!(
                tick_spacing == expected_spacing,
                FeelsProtocolError::InvalidTickSpacing
            );

            Ok(())
        }

        /// Create fee configuration for a new pool
        pub fn create_for_pool(fee_rate: u16) -> Result<(u16, u16, i16)> {
            // Validate fee rate
            require!(
                matches!(fee_rate, 1 | 5 | 30 | 100),
                FeelsProtocolError::InvalidFeeRate
            );

            // Get tick spacing
            let tick_spacing = Self::get_tick_spacing_for_fee(fee_rate)?;

            // Set protocol fee rate (e.g., 10% of swap fees)
            let protocol_fee_rate = 1000u16; // 10% = 1000 basis points

            Ok((fee_rate, protocol_fee_rate, tick_spacing))
        }
    }

    // ============================================================================
    // Liquidity Mathematics
    // ============================================================================
    //
    // All liquidity calculation functions follow a consistent pattern:
    // 1. Accept Q96 format sqrt prices for API compatibility
    // 2. Convert to Q64 format for internal calculations
    // 3. Use Orca's optimized algorithms for compute efficiency
    // 4. Convert results back to Q96 format
    //
    // This ensures consistency across all delta calculations while leveraging
    // battle-tested implementations optimized for Solana's constraints.

    /// Calculate amount of token0 for a given liquidity and price range
    ///
    /// Formula: Δtoken0 = L × (1/√P_lower - 1/√P_upper)
    ///
    /// This function accepts Q96 format sqrt prices and internally converts to Q64
    /// to use Orca's optimized get_amount_delta_a algorithm. The Q64 calculations
    /// are more efficient on Solana while maintaining precision for token amounts.
    ///
    /// # Parameters
    /// - `sqrt_ratio_0_x96`: First sqrt price in Q96 format
    /// - `sqrt_ratio_1_x96`: Second sqrt price in Q96 format
    /// - `liquidity`: The liquidity amount
    /// - `round_up`: Whether to round up the result
    ///
    /// # Returns
    /// The amount of token0, rounded according to the `round_up` parameter
    pub fn get_amount_0_delta(
        sqrt_ratio_0_x96: u128,
        sqrt_ratio_1_x96: u128,
        liquidity: u128,
        round_up: bool,
    ) -> Result<u128> {
        // Convert from Q96 to Q64 for Orca's calculations
        let sqrt_price_0_x64 = q96_to_q64(sqrt_ratio_0_x96);
        let sqrt_price_1_x64 = q96_to_q64(sqrt_ratio_1_x96);

        let (sqrt_price_lower, sqrt_price_upper) = if sqrt_price_0_x64 > sqrt_price_1_x64 {
            (sqrt_price_1_x64, sqrt_price_0_x64)
        } else {
            (sqrt_price_0_x64, sqrt_price_1_x64)
        };

        let sqrt_price_diff = sqrt_price_upper - sqrt_price_lower;

        let numerator = U256::from(liquidity)
            .checked_mul(U256::from(sqrt_price_diff))
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_shl(64)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        let denominator = U256::from(sqrt_price_upper)
            .checked_mul(U256::from(sqrt_price_lower))
            .ok_or(FeelsProtocolError::MathOverflow)?;

        let (quotient, remainder) = (
            numerator
                .checked_div(denominator)
                .ok_or(FeelsProtocolError::DivisionByZero)?,
            numerator
                .checked_rem(denominator)
                .ok_or(FeelsProtocolError::DivisionByZero)?,
        );

        let result = if round_up && !remainder.is_zero() {
            quotient
                .checked_add(U256::ONE)
                .ok_or(FeelsProtocolError::MathOverflow)?
        } else {
            quotient
        };

        result
            .try_into()
            .map_err(|_| FeelsProtocolError::MathOverflow.into())
    }

    /// Calculate amount of token1 for a given liquidity and price range
    ///
    /// Formula: Δtoken1 = L × (√P_upper - √P_lower)
    ///
    /// This function accepts Q96 format sqrt prices and internally converts to Q64
    /// to use Orca's optimized get_amount_delta_b algorithm. The Q64 calculations
    /// are more efficient on Solana while maintaining precision for token amounts.
    ///
    /// # Parameters
    /// - `sqrt_ratio_0_x96`: First sqrt price in Q96 format
    /// - `sqrt_ratio_1_x96`: Second sqrt price in Q96 format
    /// - `liquidity`: The liquidity amount
    /// - `round_up`: Whether to round up the result
    ///
    /// # Returns
    /// The amount of token1, rounded according to the `round_up` parameter
    pub fn get_amount_1_delta(
        sqrt_ratio_0_x96: u128,
        sqrt_ratio_1_x96: u128,
        liquidity: u128,
        round_up: bool,
    ) -> Result<u128> {
        // Convert from Q96 to Q64 for Orca's calculations
        let sqrt_price_0_x64 = q96_to_q64(sqrt_ratio_0_x96);
        let sqrt_price_1_x64 = q96_to_q64(sqrt_ratio_1_x96);

        let (sqrt_price_lower, sqrt_price_upper) = if sqrt_price_0_x64 > sqrt_price_1_x64 {
            (sqrt_price_1_x64, sqrt_price_0_x64)
        } else {
            (sqrt_price_0_x64, sqrt_price_1_x64)
        };

        let n0 = liquidity;
        let n1 = sqrt_price_upper - sqrt_price_lower;

        if n0 == 0 || n1 == 0 {
            return Ok(0);
        }

        if let Some(p) = n0.checked_mul(n1) {
            let result = (p >> 64) as u64;

            let should_round = round_up && (p & ((1u128 << 64) - 1) > 0);
            if should_round && result == u64::MAX {
                return Err(FeelsProtocolError::MathOverflow.into());
            }

            Ok(if should_round {
                (result + 1) as u128
            } else {
                result as u128
            })
        } else {
            Err(FeelsProtocolError::MathOverflow.into())
        }
    }

    /// Calculate liquidity from token0 amount and price range
    ///
    /// Formula: L = amount0 × √P_lower × √P_upper / (√P_upper - √P_lower)
    ///
    /// # Parameters
    /// - `sqrt_ratio_a_x96`: First sqrt price in Q96 format
    /// - `sqrt_ratio_b_x96`: Second sqrt price in Q96 format
    /// - `amount_0`: The amount of token0
    ///
    /// # Returns
    /// The liquidity amount that corresponds to the given token0 amount
    pub fn get_liquidity_for_amount_0(
        sqrt_ratio_a_x96: u128,
        sqrt_ratio_b_x96: u128,
        amount_0: u128,
    ) -> Result<u128> {
        if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
            return get_liquidity_for_amount_0(sqrt_ratio_b_x96, sqrt_ratio_a_x96, amount_0);
        }

        // Validate inputs
        require!(
            sqrt_ratio_a_x96 > 0 && sqrt_ratio_b_x96 > 0,
            FeelsProtocolError::InvalidSqrtPrice
        );
        require!(
            sqrt_ratio_a_x96 != sqrt_ratio_b_x96,
            FeelsProtocolError::InvalidPriceRange
        );

        let intermediate = mul_div(
            U256::from(amount_0),
            U256::from(sqrt_ratio_a_x96),
            U256::from(sqrt_ratio_b_x96.saturating_sub(sqrt_ratio_a_x96)),
            Rounding::Down,
        )
        .ok_or(FeelsProtocolError::MathOverflow)?;

        mul_div(
            intermediate,
            U256::from(sqrt_ratio_b_x96),
            U256::from(1u128 << 96),
            Rounding::Down,
        )
        .ok_or(FeelsProtocolError::MathOverflow)?
        .try_into()
        .map_err(|_| FeelsProtocolError::MathOverflow.into())
    }

    /// Calculate liquidity from token1 amount and price range
    ///
    /// Formula: L = amount1 / (√P_upper - √P_lower)
    ///
    /// # Parameters
    /// - `sqrt_ratio_a_x96`: First sqrt price in Q96 format
    /// - `sqrt_ratio_b_x96`: Second sqrt price in Q96 format
    /// - `amount_1`: The amount of token1
    ///
    /// # Returns
    /// The liquidity amount that corresponds to the given token1 amount
    pub fn get_liquidity_for_amount_1(
        sqrt_ratio_a_x96: u128,
        sqrt_ratio_b_x96: u128,
        amount_1: u128,
    ) -> Result<u128> {
        if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
            return get_liquidity_for_amount_1(sqrt_ratio_b_x96, sqrt_ratio_a_x96, amount_1);
        }

        // Validate inputs
        require!(
            sqrt_ratio_a_x96 > 0 && sqrt_ratio_b_x96 > 0,
            FeelsProtocolError::InvalidSqrtPrice
        );
        require!(
            sqrt_ratio_a_x96 != sqrt_ratio_b_x96,
            FeelsProtocolError::InvalidPriceRange
        );

        let numerator = U256::from(amount_1)
            .checked_shl(96)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        let denominator = U256::from(sqrt_ratio_b_x96.saturating_sub(sqrt_ratio_a_x96));
        mul_div(numerator, U256::ONE, denominator, Rounding::Down)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .try_into()
            .map_err(|_| FeelsProtocolError::MathOverflow.into())
    }

    /// Calculate the next sqrt price after swapping a given amount of token0
    ///
    /// This function accepts a Q96 format sqrt price and returns a Q96 result.
    /// Internally converts to Q64 to use Orca's get_next_sqrt_price_from_a_round_up
    /// algorithm, which is optimized for Solana's compute units.
    ///
    /// # Parameters
    /// - `sqrt_price_x96`: Current sqrt price in Q96 format
    /// - `liquidity`: The liquidity available
    /// - `amount`: Amount of token0 to swap
    /// - `add`: True if adding token0 (selling), false if removing (buying)
    ///
    /// # Returns
    /// The new sqrt rate after the swap, in Q96 format
    pub fn get_next_sqrt_rate_from_amount_0_rounding_up(
        sqrt_price_x96: u128,
        liquidity: u128,
        amount: u128,
        add: bool,
    ) -> Result<u128> {
        if amount == 0 {
            return Ok(sqrt_price_x96);
        }

        // Orca expects u64, so we need to validate
        if amount > u64::MAX as u128 {
            return Err(FeelsProtocolError::MathOverflow.into());
        }
        let amount_u64 = amount as u64;

        // Convert from Q96 to Q64 for Orca's calculations
        let sqrt_price_x64 = q96_to_q64(sqrt_price_x96);

        let product = U256::from(sqrt_price_x64)
            .checked_mul(U256::from(amount_u64 as u128))
            .ok_or(FeelsProtocolError::MathOverflow)?;

        let numerator = U256::from(liquidity)
            .checked_mul(U256::from(sqrt_price_x64))
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_shl(64)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        // In this scenario the denominator will end up being < 0
        let liquidity_shift_left = U256::from(liquidity)
            .checked_shl(64)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        if !add && liquidity_shift_left <= product {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }

        let denominator = if add {
            liquidity_shift_left
                .checked_add(product)
                .ok_or(FeelsProtocolError::MathOverflow)?
        } else {
            liquidity_shift_left
                .checked_sub(product)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?
        };

        let price_x64 = mul_div_rounding_up(numerator, U256::ONE, denominator)
            .ok_or(FeelsProtocolError::MathOverflow)?;

        let price_x64_u128: u128 = price_x64.try_into().map_err(|_| FeelsProtocolError::MathOverflow)?;

        // Convert back from Q64 to Q96
        let price_x96 = q64_to_q96(price_x64_u128);

        require!(
            (MIN_SQRT_PRICE_X96..=MAX_SQRT_PRICE_X96).contains(&price_x96),
            FeelsProtocolError::PriceOutOfBounds
        );
        Ok(price_x96)
    }

    /// Calculate the next sqrt price after swapping a given amount of token1
    ///
    /// This function accepts a Q96 format sqrt price and returns a Q96 result.
    /// Internally converts to Q64 to use Orca's get_next_sqrt_price_from_b_round_down
    /// algorithm, which is optimized for Solana's compute units.
    ///
    /// # Parameters
    /// - `sqrt_price_x96`: Current sqrt price in Q96 format
    /// - `liquidity`: The liquidity available
    /// - `amount`: Amount of token1 to swap
    /// - `add`: True if adding token1 (buying token0), false if removing (selling token0)
    ///
    /// # Returns
    /// The new sqrt rate after the swap, in Q96 format
    pub fn get_next_sqrt_rate_from_amount_1_rounding_down(
        sqrt_price_x96: u128,
        liquidity: u128,
        amount: u128,
        add: bool,
    ) -> Result<u128> {
        // Orca expects u64, so we need to validate
        if amount > u64::MAX as u128 {
            return Err(FeelsProtocolError::MathOverflow.into());
        }
        let amount_u64 = amount as u64;

        // Convert from Q96 to Q64 for Orca's calculations
        let sqrt_price_x64 = q96_to_q64(sqrt_price_x96);

        // Q64.0 << 64 => Q64.64
        let amount_x64 = (amount_u64 as u128) << 64;

        // Q64.64 / Q64.0 => Q64.64
        let delta = if liquidity == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        } else {
            // div_round_up_if
            let quotient = amount_x64 / liquidity;
            let remainder = amount_x64 % liquidity;
            let round_up = !add && remainder > 0;
            if round_up {
                quotient + 1
            } else {
                quotient
            }
        };

        // Q64(32).64 +/- Q64.64
        let result_x64 = if add {
            // We are adding token b to supply, causing price to increase
            sqrt_price_x64
                .checked_add(delta)
                .ok_or(FeelsProtocolError::MathOverflow)?
        } else {
            // We are removing token b from supply, causing price to decrease
            sqrt_price_x64
                .checked_sub(delta)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?
        };

        // Convert back from Q64 to Q96
        let result_x96 = q64_to_q96(result_x64);

        if !(MIN_SQRT_PRICE_X96..=MAX_SQRT_PRICE_X96).contains(&result_x96) {
            Err(FeelsProtocolError::PriceOutOfBounds.into())
        } else {
            Ok(result_x96)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_q96_q64_conversions() {
            // Test basic conversions
            let value_q96 = 1u128 << 96; // 1.0 in Q96
            let value_q64 = q96_to_q64(value_q96);
            assert_eq!(value_q64, 1u128 << 64); // 1.0 in Q64

            // Test round trip
            let back_to_q96 = q64_to_q96(value_q64);
            assert_eq!(back_to_q96, value_q96);

            // Test with different values
            // Note: Values smaller than 2^32 will lose precision when converting Q96->Q64->Q96
            let test_values = [
                0u128,
                1u128 << 32, // Smallest value that survives round trip
                1u128 << 64,
                1u128 << 80, // Values that can round trip perfectly
                1u128 << 95,
                1u128 << 96,
                1u128 << 100,
            ];

            for &val in &test_values {
                // Only test round trip for values that won't lose precision
                if val == 0 || (val >= (1u128 << 32) && val.trailing_zeros() >= 32) {
                    let q64 = q96_to_q64(val);
                    let q96_back = q64_to_q96(q64);
                    assert_eq!(q96_back, val, "Round trip failed for {}", val);
                }
            }

            // Test that conversion works even if not perfect round trip
            let val = u64::MAX as u128;
            let q64 = q96_to_q64(val);
            let q96_back = q64_to_q96(q64);
            // Should be close but not exact due to precision loss
            assert!(q96_back < val);
            assert!(q96_back > val - (1u128 << 32));

            // Test precision loss for small values
            assert_eq!(q96_to_q64(1), 0); // 1 in Q96 becomes 0 in Q64
            assert_eq!(q96_to_q64((1u128 << 32) - 1), 0); // Just under 2^32 becomes 0
        }

        #[test]
        fn test_tick_zero_sqrt_price() {
            // Tick 0 should return exactly 2^96 (1.0 in Q96 format)
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(0).unwrap();
            assert_eq!(sqrt_price, 79228162514264337593543950336u128); // 2^96

            // Should be well within bounds
            assert!(sqrt_price > MIN_SQRT_PRICE_X96);
            assert!(sqrt_price < MAX_SQRT_PRICE_X96);

            // Should convert back to tick 0
            let tick_back = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();
            assert_eq!(tick_back, 0);
        }

        #[test]
        fn test_tick_to_sqrt_price_consistency() {
            // Test that conversions maintain consistency
            // Use ticks that are within the valid range
            let test_ticks = [-100, -10, -1, 1, 10, 100];

            for &tick in &test_ticks {
                // Skip if tick is out of bounds
                if !(MIN_TICK..=MAX_TICK).contains(&tick) {
                    continue;
                }

                let sqrt_price_x96 = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();

                // Verify the sqrt price is in valid range
                assert!(
                    sqrt_price_x96 >= MIN_SQRT_PRICE_X96,
                    "sqrt_price {} < MIN {} for tick {}",
                    sqrt_price_x96,
                    MIN_SQRT_PRICE_X96,
                    tick
                );
                assert!(
                    sqrt_price_x96 <= MAX_SQRT_PRICE_X96,
                    "sqrt_price {} > MAX {} for tick {}",
                    sqrt_price_x96,
                    MAX_SQRT_PRICE_X96,
                    tick
                );

                // Verify we can get back to approximately the same tick
                let tick_back = TickMath::get_tick_at_sqrt_ratio(sqrt_price_x96).unwrap();

                // Due to rounding, we might be off by 1
                assert!(
                    (tick - tick_back).abs() <= 1,
                    "Tick conversion failed: {} -> {} -> {}",
                    tick,
                    sqrt_price_x96,
                    tick_back
                );
            }
        }

        #[test]
        fn test_amount_delta_calculations() {
            // Test that amount calculations work correctly with Q96/Q64 conversions
            let sqrt_price_lower_x96 = TickMath::get_sqrt_ratio_at_tick(1000).unwrap();
            let sqrt_price_upper_x96 = TickMath::get_sqrt_ratio_at_tick(2000).unwrap();
            let liquidity = 1_000_000_000_000u128; // Increased liquidity for better precision

            // Ensure prices are ordered correctly
            assert!(sqrt_price_upper_x96 > sqrt_price_lower_x96);

            // Calculate amount0 delta
            let amount0 =
                get_amount_0_delta(sqrt_price_lower_x96, sqrt_price_upper_x96, liquidity, false)
                    .unwrap();

            // Calculate amount1 delta
            let amount1 =
                get_amount_1_delta(sqrt_price_lower_x96, sqrt_price_upper_x96, liquidity, false)
                    .unwrap();

            // Both amounts should be positive for a non-zero liquidity range
            assert!(amount0 > 0, "amount0 should be positive, got {}", amount0);
            assert!(amount1 > 0, "amount1 should be positive, got {}", amount1);
        }

        #[test]
        fn test_sqrt_price_from_tick_at_bounds() {
            // Test at maximum tick
            let max_tick = MAX_TICK;
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(max_tick).unwrap();
            assert!(sqrt_price <= MAX_SQRT_PRICE_X96);

            // Test at minimum tick
            let min_tick = MIN_TICK;
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(min_tick).unwrap();
            assert!(sqrt_price >= MIN_SQRT_PRICE_X96);
        }

        #[test]
        fn test_tick_from_sqrt_price_at_bounds() {
            // Test at maximum sqrt price
            let tick = TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96).unwrap();
            assert_eq!(tick, MAX_TICK);

            // Test at minimum sqrt price
            let tick = TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96).unwrap();
            assert_eq!(tick, MIN_TICK);
        }

        #[test]
        fn test_tick_sqrt_price_symmetry() {
            // Test that conversions are symmetric for various ticks
            let test_ticks = [-100000, -1000, -1, 0, 1, 1000, 100000];

            for tick in test_ticks {
                if !(MIN_TICK..=MAX_TICK).contains(&tick) {
                    continue;
                }

                let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
                let recovered_tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();

                // Allow for off-by-one due to rounding
                assert!(
                    (tick - recovered_tick).abs() <= 1,
                    "Symmetry failed for tick {}: got sqrt_price {} -> tick {}",
                    tick,
                    sqrt_price,
                    recovered_tick
                );
            }
        }

        #[test]
        fn test_sqrt_price_monotonicity() {
            // Test that sqrt price increases monotonically with tick
            let test_ticks = [-1000, -100, -10, -1, 0, 1, 10, 100, 1000];

            let mut last_sqrt_price = 0u128;
            for tick in test_ticks {
                let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
                assert!(
                    sqrt_price > last_sqrt_price,
                    "Sqrt price not monotonic at tick {}",
                    tick
                );
                last_sqrt_price = sqrt_price;
            }
        }
    }
}
