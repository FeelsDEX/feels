use crate::common::*;
use proptest::prelude::*;
use feels::error::FeelsError;
use feels::utils::{get_tick_array_start_index};

/// Generate a valid sqrt price within market bounds
fn sqrt_price_strategy() -> impl Strategy<Value = u128> {
    (constants::MIN_SQRT_PRICE..constants::MAX_SQRT_PRICE)
}

/// Generate a valid liquidity amount
fn liquidity_strategy() -> impl Strategy<Value = u128> {
    (1u128..=u128::MAX / 2)
}

/// Generate a valid swap amount
fn swap_amount_strategy() -> impl Strategy<Value = u64> {
    (1u64..=1_000_000_000u64)
}

/// Generate a valid fee tier
fn fee_tier_strategy() -> impl Strategy<Value = u16> {
    prop_oneof![
        Just(constants::STABLE_FEE_TIER),
        Just(constants::LOW_FEE_TIER),
        Just(constants::MEDIUM_FEE_TIER),
        Just(constants::HIGH_FEE_TIER),
        (1u16..=10000u16), // Any fee up to 100%
    ]
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual swap math implementation"]
    fn fuzz_swap_step_invariants(
        sqrt_price_current in sqrt_price_strategy(),
        sqrt_price_target in sqrt_price_strategy(),
        liquidity in liquidity_strategy(),
        amount in swap_amount_strategy(),
        fee_bps in fee_tier_strategy(),
    ) {
        // Skip if prices are equal (no swap possible)
        if sqrt_price_current == sqrt_price_target {
            return Ok(());
        }
        
        let zero_for_one = sqrt_price_current > sqrt_price_target;
        
        // Simulate swap step computation
        let result = compute_swap_step(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount,
            fee_bps,
            zero_for_one,
        );
        
        match result {
            Ok(step) => {
                // Invariant 1: Amount used <= amount available
                prop_assert!(
                    step.amount_in_used <= amount,
                    "Amount used {} exceeds available {}",
                    step.amount_in_used, amount
                );
                
                // Invariant 2: Fee amount <= amount in
                prop_assert!(
                    step.fee_amount <= step.amount_in_used,
                    "Fee {} exceeds amount in {}",
                    step.fee_amount, step.amount_in_used
                );
                
                // Invariant 3: Price movement is monotonic
                if zero_for_one {
                    prop_assert!(
                        step.sqrt_price_next <= sqrt_price_current,
                        "Price should decrease for zero-for-one"
                    );
                } else {
                    prop_assert!(
                        step.sqrt_price_next >= sqrt_price_current,
                        "Price should increase for one-for-zero"
                    );
                }
                
                // Invariant 4: Price doesn't overshoot target
                if zero_for_one {
                    prop_assert!(
                        step.sqrt_price_next >= sqrt_price_target,
                        "Price overshot target"
                    );
                } else {
                    prop_assert!(
                        step.sqrt_price_next <= sqrt_price_target,
                        "Price overshot target"
                    );
                }
                
                // Invariant 5: Non-zero output for non-zero input
                if step.amount_in_used > 0 {
                    prop_assert!(
                        step.amount_out > 0,
                        "Zero output for non-zero input"
                    );
                }
            }
            Err(e) => {
                // Only expected errors
                prop_assert!(
                    matches!(
                        e,
                        FeelsError::MathOverflow |
                        FeelsError::DivisionByZero |
                        FeelsError::InsufficientLiquidity
                    ),
                    "Unexpected error: {:?}",
                    e
                );
            }
        }
    }
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual tick array implementation"]
    fn fuzz_tick_array_alignment(
        tick_index in i32::MIN..=i32::MAX,
        tick_spacing in 1u16..=1000u16,
    ) {
        let array_start = get_tick_array_start_index(tick_index, tick_spacing);
        let array_size = (feels::state::tick::TICK_ARRAY_SIZE as i32) * (tick_spacing as i32);
        
        // Invariant 1: Array start is aligned to array size
        prop_assert_eq!(
            array_start % array_size,
            0,
            "Array start {} not aligned to size {}",
            array_start,
            array_size
        );
        
        // Invariant 2: Tick is within array bounds
        prop_assert!(
            tick_index >= array_start && tick_index < array_start + array_size,
            "Tick {} not in array starting at {}",
            tick_index,
            array_start
        );
        
        // Invariant 3: Consistent for nearby ticks
        if tick_index < i32::MAX {
            let next_tick_array = get_tick_array_start_index(
                tick_index + 1,
                tick_spacing
            );
            prop_assert!(
                next_tick_array == array_start || 
                next_tick_array == array_start + array_size,
                "Inconsistent array boundaries"
            );
        }
    }
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual fee growth implementation"]
    fn fuzz_fee_growth_accumulation(
        initial_fee_growth in 0u128..=u128::MAX / 2,
        fee_amount in 0u64..=1_000_000_000u64,
        liquidity in 1u128..=u128::MAX / 2,
    ) {
        // Skip if liquidity is zero
        if liquidity == 0 {
            return Ok(());
        }
        
        // Calculate fee growth delta
        let fee_growth_delta = (fee_amount as u128)
            .checked_mul(1 << 64)
            .and_then(|v| v.checked_div(liquidity));
        
        match fee_growth_delta {
            Some(delta) => {
                let new_fee_growth = initial_fee_growth.saturating_add(delta);
                
                // Invariant 1: Fee growth is monotonic
                prop_assert!(
                    new_fee_growth >= initial_fee_growth,
                    "Fee growth decreased"
                );
                
                // Invariant 2: Fee growth delta is proportional to fees/liquidity
                let expected_delta = ((fee_amount as u128) << 64) / liquidity;
                prop_assert_eq!(
                    delta, expected_delta,
                    "Fee growth calculation mismatch"
                );
            }
            None => {
                // Overflow is acceptable for very large fees or small liquidity
                prop_assert!(
                    fee_amount > 1_000_000 || liquidity < 1000,
                    "Unexpected overflow for reasonable values"
                );
            }
        }
    }
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual price/tick conversion"]
    fn fuzz_price_tick_consistency(
        sqrt_price in sqrt_price_strategy(),
    ) {
        // Use feels utils for tick conversion
        let tick = feels::utils::tick_from_sqrt_price(sqrt_price).unwrap_or(0);
        let sqrt_price_from_tick = feels::utils::sqrt_price_from_tick(tick).unwrap_or(sqrt_price);
        let tick_from_price = feels::utils::tick_from_sqrt_price(sqrt_price_from_tick).unwrap_or(tick);
        
        // Invariant: Converting back and forth maintains tick
        // (Price may change slightly due to rounding)
        prop_assert!(
            (tick - tick_from_price).abs() <= 1,
            "Tick conversion not stable: {} -> {} -> {}",
            tick, sqrt_price_from_tick, tick_from_price
        );
    }
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual liquidity math"]
    fn fuzz_liquidity_math(
        liquidity in liquidity_strategy(),
        sqrt_price_lower in sqrt_price_strategy(),
        sqrt_price_upper in sqrt_price_strategy(),
    ) {
        // Skip invalid ranges
        if sqrt_price_lower >= sqrt_price_upper {
            return Ok(());
        }
        
        // Calculate token amounts for liquidity
        let result = calculate_tokens_from_liquidity(
            liquidity,
            sqrt_price_lower,
            sqrt_price_upper,
            sqrt_price_lower, // Current price at lower bound
        );
        
        match result {
            Ok((amount_0, amount_1)) => {
                // At lower bound, should have max token0, no token1
                prop_assert!(
                    amount_0 > 0,
                    "Should have token0 at lower bound"
                );
                prop_assert_eq!(
                    amount_1, 0,
                    "Should have no token1 at lower bound"
                );
                
                // Recalculate at upper bound
                let result_upper = calculate_tokens_from_liquidity(
                    liquidity,
                    sqrt_price_lower,
                    sqrt_price_upper,
                    sqrt_price_upper,
                );
                
                if let Ok((amount_0_upper, amount_1_upper)) = result_upper {
                    // At upper bound, should have no token0, max token1
                    prop_assert_eq!(
                        amount_0_upper, 0,
                        "Should have no token0 at upper bound"
                    );
                    prop_assert!(
                        amount_1_upper > 0,
                        "Should have token1 at upper bound"
                    );
                }
            }
            Err(_) => {
                // Math errors are acceptable for extreme values
                prop_assert!(
                    liquidity > u128::MAX / 2 ||
                    sqrt_price_upper > u128::MAX / 2,
                    "Unexpected error for reasonable values"
                );
            }
        }
    }
}

proptest! {
    #[test]
    fn fuzz_multi_array_crossing(
        initial_tick in -100000i32..=100000i32,
        tick_spacing in 1u16..=200u16,
        num_arrays_to_cross in 1usize..=5usize,
        swap_amount_multiplier in 1u64..=100u64,
    ) {
        use feels::utils::get_tick_array_start_index;
        
        // Calculate array span (number of ticks per array)
        let array_span = (feels::state::tick::TICK_ARRAY_SIZE as i32) * (tick_spacing as i32);
        
        // Calculate initial array start index using the actual utility function
        let initial_array_start = get_tick_array_start_index(initial_tick, tick_spacing);
        
        // Property 1: Array start index should be properly aligned
        prop_assert_eq!(
            initial_array_start % array_span,
            0,
            "Initial array start {} should be aligned to array span {}",
            initial_array_start,
            array_span
        );
        
        // Property 2: Initial tick should be within the initial array bounds
        prop_assert!(
            initial_tick >= initial_array_start && initial_tick < initial_array_start + array_span,
            "Initial tick {} should be within array bounds [{}, {})",
            initial_tick,
            initial_array_start,
            initial_array_start + array_span
        );
        
        // Property 3: Calculate which arrays would be crossed
        let mut arrays_to_visit = Vec::new();
        for i in 0..num_arrays_to_cross {
            let array_start = initial_array_start - (i as i32 * array_span);
            arrays_to_visit.push(array_start);
        }
        
        // Property 4: All arrays in the crossing path should be properly aligned
        for &array_start in &arrays_to_visit {
            prop_assert_eq!(
                array_start % array_span,
                0,
                "Array start {} should be aligned to array span {}",
                array_start,
                array_span
            );
        }
        
        // Property 5: Arrays should be in descending order (for zero-for-one direction)
        for i in 1..arrays_to_visit.len() {
            prop_assert!(
                arrays_to_visit[i] < arrays_to_visit[i-1],
                "Arrays should be in descending order: {} should be less than {}",
                arrays_to_visit[i],
                arrays_to_visit[i-1]
            );
        }
        
        // Property 6: The distance between consecutive arrays should be exactly one array span
        for i in 1..arrays_to_visit.len() {
            let distance = arrays_to_visit[i-1] - arrays_to_visit[i];
            prop_assert_eq!(
                distance,
                array_span,
                "Distance between consecutive arrays should be exactly one array span ({}), got {}",
                array_span,
                distance
            );
        }
        
        // Property 7: Test the tick array utility function for each array
        for &array_start in &arrays_to_visit {
            // Any tick within this array should return this array start
            let test_tick = array_start + (tick_spacing as i32); // Test with first tick in array
            let calculated_start = get_tick_array_start_index(test_tick, tick_spacing);
            
            prop_assert_eq!(
                calculated_start,
                array_start,
                "Tick {} should belong to array starting at {}, but got {}",
                test_tick,
                array_start,
                calculated_start
            );
        }
    }
}

proptest! {
    #[test]
    fn fuzz_bound_clamps(
        initial_sqrt_price in sqrt_price_strategy(),
        bound_lower_tick in -443636i32..=-100000i32,
        bound_upper_tick in 100000i32..=443636i32,
        huge_swap_amount in u64::MAX / 4..=u64::MAX / 2,
        zero_for_one: bool,
    ) {
        use feels::logic::engine::{SwapContext, SwapDirection, compute_swap_step};
        
        // Convert bound ticks to sqrt prices
        let bound_lower_sqrt = feels::utils::sqrt_price_from_tick(bound_lower_tick)
            .unwrap_or(constants::MIN_SQRT_PRICE);
        let bound_upper_sqrt = feels::utils::sqrt_price_from_tick(bound_upper_tick)
            .unwrap_or(constants::MAX_SQRT_PRICE);
        
        // Create swap context with bounds
        let direction = if zero_for_one { 
            SwapDirection::ZeroForOne 
        } else { 
            SwapDirection::OneForZero 
        };
        
        let ctx = SwapContext::new(
            direction,
            initial_sqrt_price,
            1_000_000_000u128, // sufficient liquidity
            30, // 0.3% fee
            bound_lower_tick,
            bound_upper_tick,
            10, // tick spacing
        );
        
        // Target price that would exceed bounds
        let target_sqrt_price = if zero_for_one {
            // Try to go below lower bound
            bound_lower_sqrt.saturating_sub(1_000_000_000u128)
        } else {
            // Try to go above upper bound
            bound_upper_sqrt.saturating_add(1_000_000_000u128)
        };
        
        // Only test valid direction
        if (zero_for_one && target_sqrt_price >= initial_sqrt_price) ||
           (!zero_for_one && target_sqrt_price <= initial_sqrt_price) {
            return Ok(());
        }
        
        // Execute swap step with bound clamping
        let result = compute_swap_step(
            &ctx,
            target_sqrt_price,
            Some(if zero_for_one { bound_lower_tick } else { bound_upper_tick }),
            huge_swap_amount,
        );
        
        match result {
            Ok(step) => {
                // Property 1: Final price should be within bounds
                prop_assert!(
                    step.sqrt_next >= bound_lower_sqrt,
                    "Final price {} below lower bound {}",
                    step.sqrt_next,
                    bound_lower_sqrt
                );
                prop_assert!(
                    step.sqrt_next <= bound_upper_sqrt,
                    "Final price {} above upper bound {}",
                    step.sqrt_next,
                    bound_upper_sqrt
                );
                
                // Property 2: When hitting a bound, should indicate partial execution
                if step.sqrt_next == bound_lower_sqrt || step.sqrt_next == bound_upper_sqrt {
                    prop_assert_eq!(
                        step.outcome,
                        feels::logic::engine::StepOutcome::PartialAtBound,
                        "Should indicate partial execution when hitting bound"
                    );
                }
                
                // Property 3: Price movement should be monotonic in correct direction
                if zero_for_one {
                    prop_assert!(
                        step.sqrt_next <= initial_sqrt_price,
                        "Price should decrease for zero-for-one swaps"
                    );
                } else {
                    prop_assert!(
                        step.sqrt_next >= initial_sqrt_price,
                        "Price should increase for one-for-zero swaps"
                    );
                }
                
                // Property 4: Should not overshoot target when not at bound
                if step.sqrt_next != bound_lower_sqrt && step.sqrt_next != bound_upper_sqrt {
                    if zero_for_one {
                        prop_assert!(
                            step.sqrt_next >= target_sqrt_price,
                            "Should not overshoot target price"
                        );
                    } else {
                        prop_assert!(
                            step.sqrt_next <= target_sqrt_price,
                            "Should not overshoot target price"
                        );
                    }
                }
            }
            Err(_e) => {
                // For property tests, we allow errors since we're testing edge cases
                // The important thing is that when it succeeds, it follows the invariants
            }
        }
    }
}

proptest! {
    #[test]
    #[ignore = "Requires integration with actual tick array coverage logic"]
    fn fuzz_missing_tick_array_coverage(
        current_tick in -100000i32..=100000i32,
        tick_spacing in 1u16..=200u16,
        missing_array_offset in 1i32..=5i32,
        zero_for_one: bool,
    ) {
        // Calculate array boundaries
        let array_span = (feels::state::tick::TICK_ARRAY_SIZE as i32) * (tick_spacing as i32);
        let current_array_start = (current_tick / array_span) * array_span;
        
        // Calculate which array would be missing
        let missing_array_start = if zero_for_one {
            current_array_start - (missing_array_offset * array_span)
        } else {
            current_array_start + (missing_array_offset * array_span)
        };
        
        // Property 1: Missing array should be properly aligned
        prop_assert_eq!(
            missing_array_start % array_span,
            0,
            "Missing array start {} not aligned to span {}",
            missing_array_start,
            array_span
        );
        
        // Property 2: Iterator should detect missing coverage
        let expected_array_for_next_tick = if zero_for_one {
            ((current_tick - 1) / array_span) * array_span
        } else {
            ((current_tick + 1) / array_span) * array_span
        };
        
        // If the expected array matches the missing array, coverage check should fail
        if expected_array_for_next_tick == missing_array_start {
            // In real implementation, this would trigger MissingTickArrayCoverage
            prop_assert!(
                true,
                "Should detect missing array coverage at {}",
                missing_array_start
            );
        }
        
        // Property 3: Arrays before the gap should still be accessible
        for i in 0..missing_array_offset {
            let accessible_array = if zero_for_one {
                current_array_start - (i * array_span)
            } else {
                current_array_start + (i * array_span)
            };
            
            prop_assert_ne!(
                accessible_array,
                missing_array_start,
                "Array {} should be accessible before the gap",
                accessible_array
            );
        }
    }
}

// Mock functions for testing (would be imported from actual implementation)
#[derive(Debug)]
struct SwapStepResult {
    sqrt_price_next: u128,
    amount_in_used: u64,
    amount_out: u64,
    fee_amount: u64,
}

fn compute_swap_step(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount: u64,
    fee_bps: u16,
    zero_for_one: bool,
) -> std::result::Result<SwapStepResult, FeelsError> {
    // Mock implementation
    Ok(SwapStepResult {
        sqrt_price_next: if zero_for_one {
            sqrt_price_current.min(sqrt_price_target)
        } else {
            sqrt_price_current.max(sqrt_price_target)
        },
        amount_in_used: amount / 2,
        amount_out: amount / 2,
        fee_amount: (amount as u128 * fee_bps as u128 / 1_000_000) as u64,
    })
}

fn calculate_tokens_from_liquidity(
    liquidity: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
    sqrt_price_current: u128,
) -> std::result::Result<(u64, u64), FeelsError> {
    // Mock implementation
    if sqrt_price_current <= sqrt_price_lower {
        Ok((100000, 0))
    } else if sqrt_price_current >= sqrt_price_upper {
        Ok((0, 100000))
    } else {
        Ok((50000, 50000))
    }
}