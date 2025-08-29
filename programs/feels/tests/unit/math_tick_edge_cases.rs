/// Comprehensive edge case tests for tick math security
#[cfg(test)]
mod test_tick_math_edge_cases {
    use feels::utils::{TickMath, MAX_SQRT_PRICE_X96, MAX_TICK, MIN_SQRT_PRICE_X96, MIN_TICK};

    // Test commented out - mul_shr was removed in favor of Orca's implementation
    // #[test]
    // fn test_overflow_protection_in_mul_shr() {
    //     // Test that mul_shr properly handles overflow instead of silently truncating
    //     let result = TickMath::mul_shr(u128::MAX, u128::MAX, 0);
    //     assert!(result.is_none(), "mul_shr should return None on overflow, not truncate");
    //
    //     // Test edge case where shift makes result fit
    //     let result = TickMath::mul_shr(u128::MAX, 2, 1);
    //     assert!(result.is_some(), "mul_shr should succeed when shift prevents overflow");
    // }

    #[test]
    fn test_boundary_tick_values() {
        // Test exact boundary values
        let sqrt_price_at_min = TickMath::get_sqrt_ratio_at_tick(MIN_TICK).unwrap();
        assert_eq!(
            sqrt_price_at_min, MIN_SQRT_PRICE_X96,
            "MIN_TICK should map to MIN_SQRT_PRICE_X96"
        );

        let sqrt_price_at_max = TickMath::get_sqrt_ratio_at_tick(MAX_TICK).unwrap();
        assert_eq!(
            sqrt_price_at_max, MAX_SQRT_PRICE_X96,
            "MAX_TICK should map to MAX_SQRT_PRICE_X96"
        );

        // Test out of bounds
        assert!(TickMath::get_sqrt_ratio_at_tick(MIN_TICK - 1).is_err());
        assert!(TickMath::get_sqrt_ratio_at_tick(MAX_TICK + 1).is_err());
    }

    #[test]
    fn test_precision_at_extreme_ticks() {
        // Test precision for very negative ticks
        let extreme_negative_ticks = vec![-400_000, -300_000, -200_000, -100_000];
        for tick in extreme_negative_ticks {
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
            let recovered_tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();

            // For extreme negative ticks, we allow slightly more error due to precision limits
            let error = (recovered_tick - tick).abs();
            assert!(
                error <= 1,
                "Extreme negative tick {} has error {}",
                tick,
                error
            );
        }

        // Test precision for very positive ticks
        let extreme_positive_ticks = vec![100_000, 200_000, 300_000, 400_000];
        for tick in extreme_positive_ticks {
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
            let recovered_tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();

            let error = (recovered_tick - tick).abs();
            assert!(
                error <= 1,
                "Extreme positive tick {} has error {}",
                tick,
                error
            );
        }
    }

    #[test]
    fn test_malicious_sqrt_price_inputs() {
        // Test inputs designed to cause overflow in calculations
        let malicious_inputs = vec![
            MIN_SQRT_PRICE_X96,
            MIN_SQRT_PRICE_X96 + 1,
            MAX_SQRT_PRICE_X96 - 1,
            MAX_SQRT_PRICE_X96,
            1u128 << 96,       // Exactly 2^96
            (1u128 << 96) - 1, // Just below 2^96
            u128::MAX / 2,
        ];

        for input in malicious_inputs {
            if (MIN_SQRT_PRICE_X96..=MAX_SQRT_PRICE_X96).contains(&input) {
                let result = TickMath::get_tick_at_sqrt_ratio(input);
                assert!(result.is_ok(), "Valid input {} should not panic", input);

                let tick = result.unwrap();
                assert!(
                    (MIN_TICK..=MAX_TICK).contains(&tick),
                    "Tick {} out of valid range for input {}",
                    tick,
                    input
                );
            }
        }
    }

    #[test]
    fn test_tick_spacing_edge_cases() {
        // Test ticks near spacing boundaries
        let spacings = vec![1, 10, 60, 200];

        for spacing in spacings {
            // Test ticks at spacing boundaries
            for i in -10..=10 {
                let tick = i * spacing;
                if (MIN_TICK..=MAX_TICK).contains(&tick) {
                    let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
                    let recovered = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();

                    // Should recover exact tick or adjacent tick
                    let error = (recovered - tick).abs();
                    assert!(
                        error <= 1,
                        "Tick {} with spacing {} has error {}",
                        tick,
                        spacing,
                        error
                    );
                }
            }
        }
    }

    #[test]
    fn test_negative_tick_reciprocal_constants() {
        // Verify negative tick calculation doesn't overflow

        // Test each power of 2 tick
        for i in 0..19 {
            let tick = -(1i32 << i);
            if tick >= MIN_TICK {
                let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
                assert!(sqrt_price > 0, "Negative tick {} produced zero price", tick);
                // Note: sqrt_price is u128, so it's always <= u128::MAX by definition

                // Verify the price is reasonable (should be < 1.0 for negative ticks)
                let one_in_q96 = 1u128 << 96; // 1.0 in Q96 format
                assert!(
                    sqrt_price < one_in_q96,
                    "Negative tick {} should produce price < 1.0",
                    tick
                );
            }
        }
    }

    #[test]
    fn test_ilog2_precision() {
        // Test that ilog2 provides sufficient precision
        // This is critical for the get_tick_at_sqrt_ratio algorithm
        let test_values = vec![
            MIN_SQRT_PRICE_X96,
            MIN_SQRT_PRICE_X96 * 2,
            1u128 << 96, // 1.0 in Q96
            MAX_SQRT_PRICE_X96 / 2,
            MAX_SQRT_PRICE_X96,
        ];

        for val in test_values {
            let log2_val = val.ilog2();
            // Verify we have enough bits for precision
            assert!(log2_val <= 127, "ilog2 result too large: {}", log2_val);

            // Verify 2^log2_val <= val < 2^(log2_val+1)
            let lower_bound = 1u128 << log2_val;
            let upper_bound = if log2_val < 127 {
                1u128 << (log2_val + 1)
            } else {
                u128::MAX
            };

            assert!(
                val >= lower_bound && val < upper_bound,
                "ilog2 precision error for {}: log2={}",
                val,
                log2_val
            );
        }
    }

    #[test]
    fn test_tick_to_price_monotonicity_comprehensive() {
        // Comprehensive test of monotonicity across entire tick range
        // Sample every 1000 ticks to keep test time reasonable
        let mut last_price = 0u128;

        for tick in (MIN_TICK..=MAX_TICK).step_by(1000) {
            let price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();

            if last_price > 0 {
                assert!(
                    price > last_price,
                    "Price not monotonic at tick {}: {} <= {}",
                    tick,
                    price,
                    last_price
                );
            }

            last_price = price;
        }
    }

    #[test]
    fn test_price_to_tick_inverse_consistency() {
        // Test that price->tick->price maintains consistency
        let test_prices = vec![
            MIN_SQRT_PRICE_X96,
            MIN_SQRT_PRICE_X96 + 1000,
            79228162514264337593543950336_u128, // 1.0 in Q96
            MAX_SQRT_PRICE_X96 - 1000,
            MAX_SQRT_PRICE_X96,
        ];

        for price in test_prices {
            let tick = TickMath::get_tick_at_sqrt_ratio(price).unwrap();
            let recovered_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();

            // The recovered price should be less than or equal to the input
            // This is because get_tick_at_sqrt_ratio rounds down
            assert!(
                recovered_price <= price,
                "Recovered price {} > input price {} for tick {}",
                recovered_price,
                price,
                tick
            );

            // If we go one tick higher, the price should be greater than input
            if tick < MAX_TICK {
                let next_price = TickMath::get_sqrt_ratio_at_tick(tick + 1).unwrap();
                assert!(
                    next_price > price || tick == MAX_TICK - 1,
                    "Next price {} <= input price {} for tick {}",
                    next_price,
                    price,
                    tick
                );
            }
        }
    }

    #[test]
    fn test_attack_vector_price_manipulation() {
        // Simulate attack vector: try to manipulate price by providing edge values

        // Attack 1: Try to get tick that would cause overflow in swap math
        let result = TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96);
        assert!(result.is_ok());
        let tick = result.unwrap();
        assert_eq!(tick, MAX_TICK);

        // Attack 2: Try to get negative tick that would underflow
        let result = TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96);
        assert!(result.is_ok());
        let tick = result.unwrap();
        assert!((MIN_TICK..=MIN_TICK + 1).contains(&tick));

        // Attack 3: Try values that might cause issues in log calculation
        let tricky_values = vec![
            (1u128 << 96) - 1, // Just below 1.0
            (1u128 << 96) + 1, // Just above 1.0
            1u128 << 95,       // 0.5 in Q96
            3u128 << 95,       // 1.5 in Q96
        ];

        for val in tricky_values {
            if (MIN_SQRT_PRICE_X96..=MAX_SQRT_PRICE_X96).contains(&val) {
                let result = TickMath::get_tick_at_sqrt_ratio(val);
                assert!(result.is_ok(), "Failed for tricky value {}", val);
            }
        }
    }
}
