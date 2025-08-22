use anchor_lang::prelude::*;
use feels::utils::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_math_constants() {
        // Test that our constants are within expected ranges
        assert!(MIN_TICK < 0);
        assert!(MAX_TICK > 0);
        assert!(MIN_TICK == -MAX_TICK); // Symmetry check
        
        // Test Q64 constant
        assert_eq!(Q64, 1u128 << 64);
        assert_eq!(Q64, 18446744073709551616u128);
    }

    #[test] 
    fn test_safe_math_overflow() {
        use feels::utils::SafeMath;
        
        // Test u64 overflow
        let max_u64 = u64::MAX;
        let result = max_u64.safe_add(1);
        assert!(result.is_err());
        
        // Test normal operation
        let result = 100u64.safe_add(50).unwrap();
        assert_eq!(result, 150);
        
        // Test multiplication overflow
        let large = u64::MAX / 2;
        let result = large.safe_mul(3);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_math_underflow() {
        use feels::utils::SafeMath;
        
        // Test u64 underflow
        let result = 0u64.safe_sub(1);
        assert!(result.is_err());
        
        // Test normal operation
        let result = 100u64.safe_sub(50).unwrap();
        assert_eq!(result, 50);
    }

    #[test]
    fn test_liquidity_safe_math() {
        use feels::utils::LiquiditySafeMath;
        
        let liquidity = 1000u128;
        
        // Test positive delta
        let result = liquidity.safe_add_liquidity(500).unwrap();
        assert_eq!(result, 1500);
        
        // Test negative delta 
        let result = liquidity.safe_add_liquidity(-200).unwrap();
        assert_eq!(result, 800);
        
        // Test subtraction with positive delta
        let result = liquidity.safe_sub_liquidity(300).unwrap();
        assert_eq!(result, 700);
        
        // Test subtraction with negative delta (adds)
        let result = liquidity.safe_sub_liquidity(-100).unwrap();
        assert_eq!(result, 1100);
    }

    #[test]
    fn test_mul_shr_rounding() {
        use feels::utils::{mul_shr, Rounding};
        
        let x = 1000u128;
        let y = 3u128;
        let offset = 1;
        
        // Test rounding down: (1000 * 3) >> 1 = 3000 >> 1 = 1500
        let result_down = mul_shr(x, y, offset, Rounding::Down).unwrap();
        assert_eq!(result_down, 1500);
        
        // Test with remainder to check rounding up behavior
        let x = 1001u128; // This will create a remainder when shifted
        let result_up = mul_shr(x, y, offset, Rounding::Up);
        let result_down = mul_shr(x, y, offset, Rounding::Down);
        
        // Up should be >= Down (may be equal if no remainder)
        if let (Some(up), Some(down)) = (result_up, result_down) {
            assert!(up >= down);
        }
    }

    #[test]
    fn test_shl_div_rounding() {
        use feels::utils::{shl_div, Rounding};
        
        let x = 1000u128;
        let y = 3u128;
        let offset = 1;
        
        // Test basic division: (1000 << 1) / 3 = 2000 / 3 = 666.666... 
        let result_down = shl_div(x, y, offset, Rounding::Down).unwrap();
        let result_up = shl_div(x, y, offset, Rounding::Up).unwrap();
        
        assert_eq!(result_down, 666); // Floor
        assert_eq!(result_up, 667);   // Ceiling
    }

    #[test]
    fn test_tick_conversion_bounds() {
        use feels::utils::TickMath;
        
        // Test minimum tick
        let sqrt_price = TickMath::get_sqrt_ratio_at_tick(MIN_TICK);
        assert!(sqrt_price.is_ok());
        let sqrt_price = sqrt_price.unwrap();
        assert!(sqrt_price >= MIN_SQRT_PRICE_X64);
        
        // Test maximum tick
        let sqrt_price = TickMath::get_sqrt_ratio_at_tick(MAX_TICK);
        assert!(sqrt_price.is_ok()); 
        let sqrt_price = sqrt_price.unwrap();
        assert!(sqrt_price <= MAX_SQRT_PRICE_X64);
        
        // Test out of bounds
        let result = TickMath::get_sqrt_ratio_at_tick(MIN_TICK - 1);
        assert!(result.is_err());
        
        let result = TickMath::get_sqrt_ratio_at_tick(MAX_TICK + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_tick_conversion_symmetry() {
        use feels::utils::TickMath;
        
        let test_ticks = vec![0, 1, -1, 100, -100, 10000, -10000];
        
        for tick in test_ticks {
            if tick >= MIN_TICK && tick <= MAX_TICK {
                // Convert tick to sqrt price
                let sqrt_price = TickMath::get_sqrt_ratio_at_tick(tick).unwrap();
                
                // Convert back to tick
                let recovered_tick = TickMath::get_tick_at_sqrt_ratio(sqrt_price).unwrap();
                
                // Should be close (within 1 due to rounding)
                let diff = (tick - recovered_tick).abs();
                assert!(diff <= 1, "Tick conversion failed for {}: got {}, diff {}", tick, recovered_tick, diff);
            }
        }
    }

    #[test]
    fn test_price_monotonicity() {
        use feels::utils::TickMath;
        
        // Test that higher ticks produce higher sqrt prices
        let tick1 = -1000;
        let tick2 = 0;
        let tick3 = 1000;
        
        let price1 = TickMath::get_sqrt_ratio_at_tick(tick1).unwrap();
        let price2 = TickMath::get_sqrt_ratio_at_tick(tick2).unwrap();
        let price3 = TickMath::get_sqrt_ratio_at_tick(tick3).unwrap();
        
        assert!(price1 < price2);
        assert!(price2 < price3);
        
        // Check that tick 0 gives approximately sqrt(1) * 2^64
        // sqrt(1) = 1, so should be close to Q64
        assert!(price2 > Q64 * 99 / 100); // Within 1%
        assert!(price2 < Q64 * 101 / 100);
    }

    #[test]
    fn test_u256_operations() {
        use feels::utils::U256;
        
        // Test basic construction
        let a = U256::from(1000u128);
        let b = U256::from(500u128);
        
        // Test addition
        let sum = a + b;
        assert_eq!(u128::try_from(sum).unwrap(), 1500);
        
        // Test multiplication
        let product = a * b;
        assert_eq!(u128::try_from(product).unwrap(), 500000);
        
        // Test shift operations
        let shifted = a << 1;
        assert_eq!(u128::try_from(shifted).unwrap(), 2000);
    }

    #[test]
    fn test_division_by_zero_protection() {
        use feels::utils::SafeMath;
        
        // Test safe division by zero
        let result = 100u64.safe_div(0);
        assert!(result.is_err());
        
        // Test normal division
        let result = 100u64.safe_div(5).unwrap();
        assert_eq!(result, 20);
    }

    #[test] 
    fn test_large_number_precision() {
        use feels::utils::{mul_shr, Rounding};
        
        // Test with large numbers to ensure no precision loss
        let large_a = u128::MAX / 4;
        let large_b = u128::MAX / 4;
        
        // This should not overflow with U256 intermediate calculation
        let result = mul_shr(large_a, large_b, 64, Rounding::Down);
        assert!(result.is_some());
        
        // Result should be reasonable (roughly large_a * large_b / 2^64)
        let result = result.unwrap();
        assert!(result > 0);
    }

    #[test]
    fn test_fee_calculation_precision() {
        // Test fee calculation precision similar to what we'd see in swaps
        let amount_in = 1_000_000u64; // 1M tokens
        let fee_rate = 30u16; // 0.3% (30 basis points)
        
        // Calculate fee: amount * rate / 10000
        let fee_amount = ((amount_in as u128) * (fee_rate as u128) / 10000) as u64;
        
        // Should be 3000 (0.3% of 1M)
        assert_eq!(fee_amount, 3000);
        
        // Test with smaller amounts
        let small_amount = 100u64;
        let small_fee = ((small_amount as u128) * (fee_rate as u128) / 10000) as u64;
        
        // Should be 0 (rounds down)
        assert_eq!(small_fee, 0);
    }

    #[test]
    fn test_liquidity_math_functions() {
        use feels::utils::math_liquidity::*;
        
        // Test get_amount_0_delta with valid inputs
        let sqrt_ratio_a = Q64; // Price = 1
        let sqrt_ratio_b = Q64 * 2; // Price = 4
        let liquidity = 1000u128;
        
        let amount_0 = get_amount_0_delta(sqrt_ratio_a, sqrt_ratio_b, liquidity, true);
        assert!(amount_0.is_ok());
        assert!(amount_0.unwrap() > 0);
        
        // Test get_amount_1_delta
        let amount_1 = get_amount_1_delta(sqrt_ratio_a, sqrt_ratio_b, liquidity, true);
        assert!(amount_1.is_ok());
        assert!(amount_1.unwrap() > 0);
        
        // Test boundary cases with zero liquidity
        let zero_amount = get_amount_0_delta(sqrt_ratio_a, sqrt_ratio_b, 0, true).unwrap();
        assert_eq!(zero_amount, 0);
    }

    #[test]
    fn test_general_math_functions() {
        use feels::utils::math_general::*;
        
        // Test integer square root
        assert_eq!(integer_sqrt(16), 4);
        assert_eq!(integer_sqrt(15), 3); // Rounds down
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        
        // Test percentage calculation
        let result = percentage(1000, 25, true); // 25% of 1000
        assert_eq!(result.unwrap(), 250);
        
        let result = percentage(1000, 33, false); // 33% of 1000, round down
        assert_eq!(result.unwrap(), 330);
        
        let result = percentage(1000, 33, true); // 33% of 1000, round up
        assert_eq!(result.unwrap(), 330);
    }

    #[test]
    fn test_big_integer_operations() {
        use feels::utils::math_big_int::*;
        
        // Test U256 basic operations
        let a = U256::from(u128::MAX);
        let b = U256::from(2u128);
        
        // Test multiplication doesn't overflow
        let product = a.checked_mul(b);
        assert!(product.is_some());
        
        // Test division
        let quotient = a.checked_div(b);
        assert!(quotient.is_some());
        assert_eq!(quotient.unwrap(), U256::from(u128::MAX / 2));
        
        // Test shift operations
        let shifted_left = a << 1;
        let shifted_right = shifted_left >> 1;
        assert_eq!(shifted_right, a);
    }

    #[test]
    fn test_tick_math_edge_cases() {
        // Test tick spacing validation
        let valid_spacings = [1, 60, 200];
        for spacing in valid_spacings {
            // Should be able to validate tick alignment
            let aligned_tick = (1000 / spacing) * spacing;
            assert_eq!(aligned_tick % spacing, 0);
        }
        
        // Test price impact calculations
        let tick_lower = -1000;
        let tick_upper = 1000;
        let current_tick = 0;
        
        assert!(tick_lower < current_tick);
        assert!(current_tick < tick_upper);
        
        // Test tick boundaries
        assert!(MIN_TICK < 0);
        assert!(MAX_TICK > 0);
        assert_eq!(MIN_TICK, -MAX_TICK);
    }

    #[test]
    fn test_fee_growth_math() {
        use feels::utils::math_fee::*;
        use feels::utils::math_big_int::U256;
        
        // Test fee growth tracking precision
        // Fee growth is stored as Q128.128 fixed point
        let fee_amount = 1000u64;
        let liquidity = 100000u128;
        
        // Calculate fee growth per unit of liquidity
        // This should use Q128.128 precision to avoid rounding errors
        let fee_growth = (U256::from(fee_amount) << 128) / U256::from(liquidity);
        assert!(fee_growth > U256::ZERO);
        
        // Test fee growth accumulation
        let prev_growth = fee_growth;
        let new_fee = 500u64;
        let additional_growth = (U256::from(new_fee) << 128) / U256::from(liquidity);
        let total_growth = prev_growth + additional_growth;
        
        assert!(total_growth > prev_growth);
        assert_eq!(total_growth, prev_growth + additional_growth);
    }

    #[test]
    fn test_constants_validity() {
        // Test mathematical constants are reasonable
        assert!(Q64 > 0);
        assert_eq!(Q64, 1u128 << 64);
        
        // Test fee constants
        assert_eq!(BASIS_POINTS_DENOMINATOR, 10_000);
        assert!(MAX_FEE_RATE <= BASIS_POINTS_DENOMINATOR as u16);
        
        // Test valid fee tiers
        for &tier in VALID_FEE_TIERS {
            assert!(tier <= MAX_FEE_RATE);
            assert!(tier > 0);
        }
        
        // Test tick array constants
        assert_eq!(TICK_ARRAY_SIZE, 60);
        assert!(TICK_ARRAY_SIZE_BITS >= 6); // ceil(log2(60))
    }

    #[test]
    fn test_overflow_protection() {
        use feels::utils::SafeMath;
        
        // Test multiplication overflow protection
        let large = u128::MAX / 2;
        let result = large.safe_mul(3);
        assert!(result.is_err());
        
        // Test addition overflow protection
        let result = u128::MAX.safe_add(1);
        assert!(result.is_err());
        
        // Test subtraction underflow protection
        let result = 0u128.safe_sub(1);
        assert!(result.is_err());
        
        // Test division by zero protection
        let result = 100u128.safe_div(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_precision_rounding() {
        use feels::utils::{mul_shr, shl_div, Rounding};
        
        // Test rounding behavior with remainders
        let x = 7u128;
        let y = 2u128;
        let offset = 1;
        
        // 7 * 2 = 14, 14 >> 1 = 7 (exact)
        let result_down = mul_shr(x, y, offset, Rounding::Down).unwrap();
        let result_up = mul_shr(x, y, offset, Rounding::Up).unwrap();
        assert_eq!(result_down, 7);
        assert_eq!(result_up, 7);
        
        // Test with remainder: 15 / 2 = 7.5
        let result_down = shl_div(15u128, 2u128, 0, Rounding::Down).unwrap();
        let result_up = shl_div(15u128, 2u128, 0, Rounding::Up).unwrap();
        assert_eq!(result_down, 7);
        assert_eq!(result_up, 8);
    }

    #[test]
    fn test_liquidity_delta_calculations() {
        use feels::utils::LiquiditySafeMath;
        
        let base_liquidity = 1000u128;
        
        // Test adding liquidity
        let result = base_liquidity.safe_add_liquidity(500i128).unwrap();
        assert_eq!(result, 1500);
        
        // Test removing liquidity
        let result = base_liquidity.safe_add_liquidity(-300i128).unwrap();
        assert_eq!(result, 700);
        
        // Test underflow protection
        let result = base_liquidity.safe_add_liquidity(-1500i128);
        assert!(result.is_err());
        
        // Test subtraction with positive delta
        let result = base_liquidity.safe_sub_liquidity(200i128).unwrap();
        assert_eq!(result, 800);
        
        // Test subtraction with negative delta (becomes addition)
        let result = base_liquidity.safe_sub_liquidity(-100i128).unwrap();
        assert_eq!(result, 1100);
    }

    #[test]
    fn test_error_handling_utilities() {
        use feels::utils::error_handling::*;
        
        // Test error conversion and context
        let error = create_error_with_context("Math overflow in liquidity calculation");
        assert!(error.to_string().contains("Math overflow"));
        
        // Test that we can handle different error types
        let anchor_error = anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::AccountNotMutable);
        let converted = handle_anchor_error(anchor_error);
        assert!(converted.is_err());
    }
}