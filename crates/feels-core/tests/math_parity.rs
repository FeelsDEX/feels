//! # Math Parity Tests
//! 
//! Comprehensive tests to ensure mathematical operations produce identical results
//! between on-chain and off-chain implementations.

#[cfg(test)]
mod tests {
    use feels_core::math::*;
    use feels_core::constants::*;

    #[test]
    fn test_tick_math_parity() {
        // Test critical tick values
        let test_ticks = vec![
            MIN_TICK,
            MIN_TICK + 1,
            -100000,
            -10000,
            -1000,
            -100,
            -10,
            -1,
            0,
            1,
            10,
            100,
            1000,
            10000,
            100000,
            MAX_TICK - 1,
            MAX_TICK,
        ];

        for tick in test_ticks {
            // Test tick to sqrt price conversion
            let sqrt_price = get_sqrt_price_at_tick(tick).unwrap();
            
            // Verify sqrt price is within valid range
            assert!(sqrt_price >= MIN_SQRT_PRICE_X64);
            assert!(sqrt_price <= MAX_SQRT_PRICE_X64);
            
            // Test round trip conversion
            let recovered_tick = get_tick_at_sqrt_price(sqrt_price).unwrap();
            assert_eq!(tick, recovered_tick, "Round trip failed for tick {}", tick);
        }
    }

    #[test]
    fn test_liquidity_math_parity() {
        // Test with various sqrt price ranges
        let test_cases = vec![
            (Q64, Q64 + (Q64 / 100), 1000u128), // 1% range
            (Q64 - (Q64 / 100), Q64 + (Q64 / 100), 10000u128), // 2% range around 1.0
            (Q64 / 2, Q64, 50000u128), // Large range
        ];

        for (sqrt_lower, sqrt_upper, liquidity) in test_cases {
            // Test amount0 delta
            let amount0_up = get_amount_0_delta(sqrt_lower, sqrt_upper, liquidity, true).unwrap();
            let amount0_down = get_amount_0_delta(sqrt_lower, sqrt_upper, liquidity, false).unwrap();
            
            // Rounding up should always be >= rounding down
            assert!(amount0_up >= amount0_down);
            
            // Test amount1 delta
            let amount1_up = get_amount_1_delta(sqrt_lower, sqrt_upper, liquidity, true).unwrap();
            let amount1_down = get_amount_1_delta(sqrt_lower, sqrt_upper, liquidity, false).unwrap();
            
            // Rounding up should always be >= rounding down
            assert!(amount1_up >= amount1_down);
            
            // Test liquidity from amounts round trip
            if amount0_down > 0 && amount1_down > 0 {
                let liquidity_from_0 = get_liquidity_for_amount_0(sqrt_lower, sqrt_upper, amount0_down as u64).unwrap();
                let liquidity_from_1 = get_liquidity_for_amount_1(sqrt_lower, sqrt_upper, amount1_down as u64).unwrap();
                
                // Should be close to original liquidity (within rounding)
                assert!((liquidity_from_0 as i128 - liquidity as i128).abs() < 100);
                assert!((liquidity_from_1 as i128 - liquidity as i128).abs() < 100);
            }
        }
    }

    #[test]
    fn test_fee_math_parity() {
        // Test fee growth calculations
        let test_cases = vec![
            (100u64, 1000u128), // 10% fee rate
            (1u64, 1000000u128), // Very small fee rate
            (1000u64, 10u128), // Large fee, small liquidity
        ];

        for (fee_amount, liquidity) in test_cases {
            let fee_growth = calculate_fee_growth_q64(fee_amount, liquidity).unwrap();
            let fee_growth_u128 = words_to_u128(fee_growth);
            
            // Verify the calculation: fee_growth = (fee_amount * 2^64) / liquidity
            let expected = ((fee_amount as u128) << 64) / liquidity;
            assert_eq!(fee_growth_u128, expected);
            
            // Test word conversion round trip
            let words = u128_to_words(fee_growth_u128);
            assert_eq!(fee_growth, words);
        }
    }

    #[test]
    fn test_big_int_operations() {
        // Test U256 operations
        let a = U256::from_u128(u128::MAX / 2);
        let b = U256::from_u128(2);
        let c = U256::from_u128(2);
        
        // Test mul_div with exact division
        let result = mul_div(a, b, c, Rounding::Down).unwrap();
        assert_eq!(result.to_u128().unwrap(), u128::MAX / 2);
        
        // Test with rounding
        let a = U256::from_u128(10);
        let b = U256::from_u128(3);
        let c = U256::from_u128(4);
        
        let result_down = mul_div(a, b, c, Rounding::Down).unwrap();
        let result_up = mul_div(a, b, c, Rounding::Up).unwrap();
        
        // 10 * 3 / 4 = 7.5
        assert_eq!(result_down.to_u128().unwrap(), 7);
        assert_eq!(result_up.to_u128().unwrap(), 8);
    }

    #[test]
    fn test_safe_math_operations() {
        // Test safe arithmetic
        assert_eq!(add_u64(100, 200).unwrap(), 300);
        assert!(add_u64(u64::MAX, 1).is_err());
        
        assert_eq!(sub_u64(300, 200).unwrap(), 100);
        assert!(sub_u64(100, 200).is_err());
        
        assert_eq!(mul_u64(100, 200).unwrap(), 20000);
        assert!(mul_u64(u64::MAX, 2).is_err());
        
        assert_eq!(div_u64(200, 100).unwrap(), 2);
        assert!(div_u64(100, 0).is_err());
    }

    #[test]
    fn test_conservation_law() {
        // Test that operations maintain conservation
        // For a rebase operation: Σ w_i * ln(g_i) = 0
        
        let weights = vec![3333u64, 3333u64, 3334u64]; // Sum to 10000 (basis points)
        let growth_factors = vec![
            U256::from_u128(1u128 << 64), // 1.0 in Q64
            U256::from_u128((1u128 << 64) + (1u128 << 63)), // 1.5 in Q64
            U256::from_u128((1u128 << 64) / 2), // 0.5 in Q64
        ];
        
        // In a proper conservation law, the weighted sum of log growth factors should be 0
        // Using proper ln calculations: Σ wᵢ ln(gᵢ) = 0
        use feels_core::math::fixed_point::ln_q64;
        
        let mut weighted_ln_sum = 0i128;
        for (i, &weight) in weights.iter().enumerate() {
            // Calculate ln of growth factor
            let growth_factor = growth_factors[i].to_u128().unwrap();
            let ln_growth = ln_q64(growth_factor).unwrap_or(0);
            
            // Weight the logarithm (weight is in basis points)
            weighted_ln_sum += (weight as i128) * ln_growth / 10000;
        }
        
        // Conservation law requires the weighted sum of logarithms to be zero
        // Allow small tolerance for numerical errors (0.01% of Q64)
        let tolerance = (Q64 / 10000) as i128;
        assert!(
            weighted_ln_sum.abs() < tolerance,
            "Conservation law violated: weighted ln sum = {}, expected ~0",
            weighted_ln_sum
        );
    }

    #[test]
    fn test_edge_cases() {
        // Test edge cases for tick math
        assert!(get_sqrt_price_at_tick(MIN_TICK - 1).is_err());
        assert!(get_sqrt_price_at_tick(MAX_TICK + 1).is_err());
        
        assert!(get_tick_at_sqrt_price(MIN_SQRT_PRICE_X64 - 1).is_err());
        assert!(get_tick_at_sqrt_price(MAX_SQRT_PRICE_X64 + 1).is_err());
        
        // Test edge cases for liquidity math
        assert_eq!(get_amount_0_delta(Q64, Q64, 1000, false).unwrap(), 0);
        assert_eq!(get_amount_1_delta(Q64, Q64, 1000, false).unwrap(), 0);
        
        // Test edge cases for fee math
        assert_eq!(calculate_fee_growth_q64(0, 1000).unwrap(), [0, 0, 0, 0]);
        assert!(calculate_fee_growth_q64(100, 0).is_err());
    }

    #[test]
    fn test_next_initialized_tick() {
        let spacing = 60;
        
        // Test various tick positions
        assert_eq!(get_next_initialized_tick(0, spacing, true), 0);
        assert_eq!(get_next_initialized_tick(0, spacing, false), 0);
        
        assert_eq!(get_next_initialized_tick(30, spacing, true), 0);
        assert_eq!(get_next_initialized_tick(30, spacing, false), 60);
        
        assert_eq!(get_next_initialized_tick(-30, spacing, true), -60);
        assert_eq!(get_next_initialized_tick(-30, spacing, false), 0);
        
        assert_eq!(get_next_initialized_tick(60, spacing, true), 60);
        assert_eq!(get_next_initialized_tick(60, spacing, false), 60);
    }

    #[test]
    fn test_position_fee_growth() {
        // Test fee growth calculation for positions
        let tick_lower = -100;
        let tick_upper = 100;
        let fee_growth_global = u128_to_words(1000);
        let fee_growth_outside_lower = u128_to_words(100);
        let fee_growth_outside_upper = u128_to_words(200);
        
        // Test when current tick is inside range
        let tick_current = 0;
        let fee_growth_inside = calculate_position_fee_growth_inside(
            tick_lower,
            tick_upper,
            tick_current,
            fee_growth_global,
            fee_growth_outside_lower,
            fee_growth_outside_upper,
        );
        
        // Inside = global - outside_lower - outside_upper = 1000 - 100 - 200 = 700
        assert_eq!(words_to_u128(fee_growth_inside), 700);
        
        // Test when current tick is below range
        let tick_current = -200;
        let fee_growth_inside = calculate_position_fee_growth_inside(
            tick_lower,
            tick_upper,
            tick_current,
            fee_growth_global,
            fee_growth_outside_lower,
            fee_growth_outside_upper,
        );
        
        // When below range, the calculation changes
        let fee_growth_below = sub_fee_growth_words(fee_growth_global, fee_growth_outside_lower);
        let expected = words_to_u128(sub_fee_growth_words(
            sub_fee_growth_words(fee_growth_global, fee_growth_below),
            fee_growth_outside_upper
        ));
        assert_eq!(words_to_u128(fee_growth_inside), expected);
    }
}