//! Tests for POMM u128 saturation edge case
//!
//! Ensures that the POMM threshold check handles the extremely unlikely
//! case where both fee buffers approach u128::MAX without causing
//! unintended behavior.

use crate::common::*;
use feels::state::{Buffer, FeeDomain};

test_in_memory!(test_normal_threshold_check, |ctx: TestContext| async move {
    // Normal case: fees below threshold
    let mut buffer = create_test_buffer();
    buffer.floor_placement_threshold = 1000;
    buffer.fees_token_0 = 400;
    buffer.fees_token_1 = 500;

    // Total is 900, which is less than 1000
    assert!(!should_trigger_pomm(&buffer));

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_threshold_exceeded, |ctx: TestContext| async move {
    // Normal case: fees exceed threshold
    let mut buffer = create_test_buffer();
    buffer.floor_placement_threshold = 1000;
    buffer.fees_token_0 = 600;
    buffer.fees_token_1 = 500;

    // Total is 1100, which exceeds 1000
    assert!(should_trigger_pomm(&buffer));

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(
    test_single_fee_exceeds_threshold,
    |ctx: TestContext| async move {
        // Edge case: single fee exceeds threshold
        let mut buffer = create_test_buffer();
        buffer.floor_placement_threshold = 1000;
        buffer.fees_token_0 = 1500; // Exceeds threshold alone
        buffer.fees_token_1 = 100;

        // Token 0 alone exceeds threshold
        assert!(should_trigger_pomm(&buffer));

        // Test with token 1 exceeding
        buffer.fees_token_0 = 100;
        buffer.fees_token_1 = 1500; // Exceeds threshold alone
        assert!(should_trigger_pomm(&buffer));

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(test_u128_saturation_case, |ctx: TestContext| async move {
    // Edge case: both fees near u128::MAX
    let mut buffer = create_test_buffer();
    buffer.floor_placement_threshold = 1000;

    // Set both fees to values that would overflow if added naively
    buffer.fees_token_0 = u128::MAX - 100;
    buffer.fees_token_1 = u128::MAX - 100;

    // Either fee alone far exceeds the threshold
    assert!(should_trigger_pomm(&buffer));

    // The fix ensures we don't rely on the sum in this case
    // If we did add them, saturating_add would give u128::MAX
    let sum = buffer.fees_token_0.saturating_add(buffer.fees_token_1);
    assert_eq!(sum, u128::MAX);

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_large_threshold, |ctx: TestContext| async move {
    // Test with a large threshold value
    let mut buffer = create_test_buffer();
    buffer.floor_placement_threshold = u64::MAX / 2;

    // Both fees are large but don't individually exceed threshold
    buffer.fees_token_0 = (u64::MAX / 3) as u128;
    buffer.fees_token_1 = (u64::MAX / 3) as u128;

    // Combined they exceed threshold
    assert!(should_trigger_pomm(&buffer));

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_zero_threshold, |ctx: TestContext| async move {
    // Edge case: zero threshold (should always trigger)
    let mut buffer = create_test_buffer();
    buffer.floor_placement_threshold = 0;
    buffer.fees_token_0 = 0;
    buffer.fees_token_1 = 0;

    // Even with zero fees, zero threshold means always trigger
    assert!(should_trigger_pomm(&buffer));

    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(
    test_floor_placement_due_method,
    |ctx: TestContext| async move {
        // Test the existing floor_placement_due method for comparison
        let buffer = create_test_buffer();

        // This method takes u64 values and internally converts to u128
        assert!(buffer.floor_placement_due(600, 500)); // 1100 > 1000
        assert!(!buffer.floor_placement_due(400, 500)); // 900 < 1000

        // Test with max u64 values (should not overflow)
        assert!(buffer.floor_placement_due(u64::MAX, u64::MAX));

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

test_in_memory!(
    test_fee_collection_overflow_safety,
    |ctx: TestContext| async move {
        // Test that fee collection properly handles overflow
        let mut buffer = create_test_buffer();
        buffer.fees_token_0 = u128::MAX - 100;

        // Try to collect more fees
        let result = buffer.collect_fee(200, 0, FeeDomain::Spot);

        // Should return overflow error
        assert!(matches!(result, Err(ref e) if e.to_string().contains("Math overflow")));

        // tau_spot should also handle overflow
        buffer.tau_spot = u128::MAX - 50;
        let result = buffer.collect_fee(100, 0, FeeDomain::Spot);
        assert!(matches!(result, Err(ref e) if e.to_string().contains("Math overflow")));

        Ok::<(), Box<dyn std::error::Error>>(())
    }
);

// Helper function to create a test buffer
fn create_test_buffer() -> Buffer {
    Buffer {
        market: Pubkey::default(),
        authority: Pubkey::default(),
        feelssol_mint: Pubkey::default(),
        fees_token_0: 0,
        fees_token_1: 0,
        tau_spot: 0,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 0,
        floor_placement_threshold: 1000, // Default threshold
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 0,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
        jit_rolling_consumption: 0,
        jit_rolling_window_start: 0,
        jit_last_heavy_usage_slot: 0,
        jit_total_consumed_epoch: 0,
        initial_tau_spot: 0,
        protocol_owned_override: 0,
        pomm_position_count: 0,
        _padding: [0; 7],
    }
}

// Helper function that mimics the POMM threshold check logic
fn should_trigger_pomm(buffer: &Buffer) -> bool {
    let threshold_u128 = buffer.floor_placement_threshold as u128;

    // If either individual fee exceeds threshold, we definitely have enough
    if buffer.fees_token_0 >= threshold_u128 || buffer.fees_token_1 >= threshold_u128 {
        true
    } else {
        // Otherwise, check the sum (using saturating_add for safety)
        let total_fees = buffer.fees_token_0.saturating_add(buffer.fees_token_1);
        total_fees >= threshold_u128
    }
}
