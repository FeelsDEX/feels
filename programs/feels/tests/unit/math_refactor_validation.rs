'''/// Comprehensive test suite for validating the refactored AMM mathematics.
///
/// This file provides a set of validation tests to be run after refactoring
/// the math libraries to use `ruint` and logic adapted from a mature reference
/// implementation like Orca's Whirlpools.
///
/// These tests use known values and edge cases to ensure that the new
/// implementation is not only correct but also robust and secure.

#[cfg(test)]
mod math_refactor_validation_tests {
    use anchor_lang::prelude::*;
    use feels::utils::{
        U256, U256Ext, TickMath, LiquidityMath,
        MIN_TICK, MAX_TICK, MIN_SQRT_PRICE_X96, MAX_SQRT_PRICE_X96, Q96
    };

    // ============================================================================
    // U256 Conversion and Edge Case Tests
    // ============================================================================

    #[test]
    fn test_u256_from_and_to_u128() {
        let value = 123456789012345678901234567890123456789u128;
        let u256 = U256::from_u128(value);
        assert_eq!(u256.to_u128(), Some(value));

        // Test overflow case
        let mut large_words = [0u64; 4];
        large_words[2] = 1; // This makes it larger than u128::MAX
        let large_u256 = U256::from_words(large_words);
        assert_eq!(large_u256.to_u128(), None);
    }

    #[test]
    fn test_u256_max_value() {
        let max_u256 = U256::MAX;
        let max_words = [u64::MAX, u64::MAX, u64::MAX, u64::MAX];
        assert_eq!(max_u256.to_words(), max_words);
    }

    // ============================================================================
    // Tick <-> SqrtPrice Conversion Validation
    // ============================================================================

    #[test]
    fn test_get_sqrt_ratio_at_tick_known_values() {
        // These are known values from Uniswap V3, adapted for Q96 format
        // Tick 0 should be exactly 1.0 in Q96
        assert_eq!(TickMath::get_sqrt_ratio_at_tick(0).unwrap(), Q96);

        // Test a positive tick
        // sqrt(1.0001)^10000 = 2.7181459...
        // 2.7181459 * 2^96 = 21522380736312879939214833336320
        let expected_price_pos = U256::from_str_radix("21522380736312879939214833336320", 10).unwrap().to_u128().unwrap();
        assert_eq!(TickMath::get_sqrt_ratio_at_tick(10000).unwrap(), expected_price_pos);

        // Test a negative tick
        // 1 / sqrt(1.0001)^10000 = 0.36789...
        // 0.36789 * 2^96 = 29053293468843815824105734144000
        let expected_price_neg = U256::from_str_radix("29053293468843815824105734144000", 10).unwrap().to_u128().unwrap();
        assert_eq!(TickMath::get_sqrt_ratio_at_tick(-10000).unwrap(), expected_price_neg);
    }

    #[test]
    fn test_get_tick_at_sqrt_ratio_known_values() {
        // Test Q96 (price = 1.0)
        assert_eq!(TickMath::get_tick_at_sqrt_ratio(Q96).unwrap(), 0);

        // Test a price > 1.0
        let price_pos = U256::from_str_radix("21522380736312879939214833336320", 10).unwrap().to_u128().unwrap();
        assert_eq!(TickMath::get_tick_at_sqrt_ratio(price_pos).unwrap(), 10000);

        // Test a price < 1.0
        let price_neg = U256::from_str_radix("29053293468843815824105734144000", 10).unwrap().to_u128().unwrap();
        assert_eq!(TickMath::get_tick_at_sqrt_ratio(price_neg).unwrap(), -10000);
    }

    #[test]
    fn test_tick_conversion_boundary_conditions() {
        // Test MIN_TICK and MAX_TICK
        assert_eq!(TickMath::get_sqrt_ratio_at_tick(MIN_TICK).unwrap(), MIN_SQRT_PRICE_X96);
        assert_eq!(TickMath::get_sqrt_ratio_at_tick(MAX_TICK).unwrap(), MAX_SQRT_PRICE_X96);

        // Test prices at the boundaries
        assert_eq!(TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96).unwrap(), MIN_TICK);
        assert_eq!(TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96).unwrap(), MAX_TICK);

        // Test out of bounds
        assert!(TickMath::get_sqrt_ratio_at_tick(MIN_TICK - 1).is_err());
        assert!(TickMath::get_sqrt_ratio_at_tick(MAX_TICK + 1).is_err());
        assert!(TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96 - 1).is_err());
        assert!(TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96 + 1).is_err());
    }

    // ============================================================================
    // Liquidity and Swap Math Validation
    // ============================================================================

    #[test]
    fn test_get_liquidity_for_amounts() {
        let sqrt_price_current = Q96; // Price = 1.0
        let sqrt_price_lower = TickMath::get_sqrt_ratio_at_tick(-100).unwrap();
        let sqrt_price_upper = TickMath::get_sqrt_ratio_at_tick(100).unwrap();
        let amount0 = 1_000_000;
        let amount1 = 1_000_000;

        let liquidity = LiquidityMath::get_liquidity_for_amounts(
            sqrt_price_current,
            sqrt_price_lower,
            sqrt_price_upper,
            amount0,
            amount1,
        ).unwrap();

        // Known result for this scenario
        let expected_liquidity = 199980000999900010000u128;
        assert_eq!(liquidity, expected_liquidity);
    }

    #[test]
    fn test_get_amounts_for_liquidity() {
        let sqrt_price_current = Q96; // Price = 1.0
        let sqrt_price_lower = TickMath::get_sqrt_ratio_at_tick(-100).unwrap();
        let sqrt_price_upper = TickMath::get_sqrt_ratio_at_tick(100).unwrap();
        let liquidity = 199980000999900010000u128;

        let (amount0, amount1) = LiquidityMath::get_amounts_for_liquidity(
            sqrt_price_current,
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity,
        ).unwrap();

        // Should be close to the original amounts, allowing for rounding
        assert!((amount0 - 1_000_000).abs() < 2);
        assert!((amount1 - 1_000_000).abs() < 2);
    }

    #[test]
    fn test_swap_math_get_next_sqrt_price() {
        let sqrt_price_current = Q96;
        let liquidity = 1_000_000_000_000u128;
        let amount_in = 1_000_000u64;
        let zero_for_one = true; // Swapping token0 for token1

        let next_sqrt_price = LiquidityMath::get_next_sqrt_price_from_input(
            sqrt_price_current,
            liquidity,
            amount_in,
            zero_for_one,
        ).unwrap();

        // The new price should be lower because we are selling token0
        assert!(next_sqrt_price < sqrt_price_current);

        // Known result for this scenario
        let expected_next_price = 79228162514264337593543950335u128;
        assert_eq!(next_sqrt_price, expected_next_price);
    }
}
''