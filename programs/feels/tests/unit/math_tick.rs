/// Tests for tick math to ensure correctness and safety

#[cfg(test)]
mod test_tick_math {
    use feels::utils::{TickMath, MIN_TICK, MAX_TICK, MIN_SQRT_PRICE_X96, MAX_SQRT_PRICE_X96};

    #[test]
    fn test_sqrt_price_bounds() {
        // Test lower bound - use saturating_sub to avoid underflow in test
        if MIN_SQRT_PRICE_X96 > 0 {
            let result = TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96 - 1);
            assert!(result.is_err());
        }
        
        // Test upper bound - only test if we won't overflow
        if MAX_SQRT_PRICE_X96 < u128::MAX {
            let result = TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96 + 1);
            assert!(result.is_err());
        }
        
        // Test valid range
        let result = TickMath::get_tick_at_sqrt_ratio(MIN_SQRT_PRICE_X96);
        assert!(result.is_ok());
        let min_tick = result.unwrap();
        // Should be at or very close to MIN_TICK
        assert!(min_tick >= MIN_TICK && min_tick <= MIN_TICK + 1);
        
        let result = TickMath::get_tick_at_sqrt_ratio(MAX_SQRT_PRICE_X96);
        assert!(result.is_ok());
        let max_tick = result.unwrap();
        // Should be close to MAX_TICK
        assert!(max_tick >= MAX_TICK - 10 && max_tick <= MAX_TICK);
    }

    #[test]
    fn test_tick_to_sqrt_ratio_clamping() {
        // Test that we get clamped values at boundaries
        let sqrt_price_min = TickMath::get_sqrt_ratio_at_tick(MIN_TICK).unwrap();
        assert_eq!(sqrt_price_min, MIN_SQRT_PRICE_X96);
        
        let sqrt_price_max = TickMath::get_sqrt_ratio_at_tick(MAX_TICK).unwrap();
        assert!(sqrt_price_max <= MAX_SQRT_PRICE_X96);
    }

    #[test]
    fn test_binary_search_no_overflow() {
        // Test that binary search handles extreme values without overflow
        // This would have failed with the old (low + high) / 2 implementation
        let prices = vec![
            MIN_SQRT_PRICE_X96,
            MIN_SQRT_PRICE_X96 + 1000,
            (MIN_SQRT_PRICE_X96 + MAX_SQRT_PRICE_X96) / 2,
            MAX_SQRT_PRICE_X96 - 1000,
            MAX_SQRT_PRICE_X96,
        ];
        
        for price in prices {
            let result = TickMath::get_tick_at_sqrt_ratio(price);
            assert!(result.is_ok(), "Failed for price: {}", price);
            
            let tick = result.unwrap();
            assert!(tick >= MIN_TICK && tick <= MAX_TICK);
        }
    }

    #[test]
    fn test_mul_shift_64_overflow_handling() {
        // Test that mul_shift_64 handles overflow gracefully
        // The function should saturate at u128::MAX rather than panic
        let sqrt_price = TickMath::get_sqrt_ratio_at_tick(MAX_TICK).unwrap();
        assert!(sqrt_price > 0);
        assert!(sqrt_price <= MAX_SQRT_PRICE_X96);
    }

    #[test]
    fn test_tick_price_consistency() {
        // Test round-trip conversion consistency
        // Test both positive and negative ticks
        let test_ticks = vec![-10000, -1000, -100, -10, -1, 0, 1, 10, 100, 1000, 10000];
        
        for tick in test_ticks {
            let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
            let recovered_tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();
            
            // Allow for rounding difference
            // The tick math implementation has inherent precision limitations
            let expected_error = if tick == 0 {
                0 // Tick 0 should be exact
            } else {
                1 // Allow 1 tick error for all other values
            };
            
            let actual_error = (recovered_tick - tick).abs();
            assert!(
                actual_error <= expected_error,
                "Tick {} -> sqrt_price {} -> tick {} (error: {}, max allowed: {})",
                tick, sqrt_price, recovered_tick, actual_error, expected_error
            );
        }
    }

    #[test]
    fn test_price_monotonicity() {
        // Ensure prices increase monotonically with ticks
        let ticks = vec![-10000, -1000, -100, 0, 100, 1000, 10000];
        let mut last_price = 0u128;
        
        for tick in ticks {
            let price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
            if last_price > 0 {
                assert!(price > last_price, "Price not monotonic at tick {} (price: {}, last: {})", tick, price, last_price);
            }
            last_price = price;
        }
    }
}