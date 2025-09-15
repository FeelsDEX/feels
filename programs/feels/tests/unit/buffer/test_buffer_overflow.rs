//! Test for buffer overflow vulnerabilities

use crate::common::*;
use feels::state::Buffer;
use feels::state::FeeDomain;

test_in_memory!(test_floor_placement_due_no_overflow, |ctx: TestContext| async move {
    // Create a buffer with a reasonable threshold
    let buffer = Buffer {
        market: Pubkey::default(),
        authority: Pubkey::default(),
        feelssol_mint: Pubkey::default(),
        fees_token_0: 0,
        fees_token_1: 0,
        tau_spot: 0,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 100,
        floor_placement_threshold: 1_000_000_000, // 1 billion
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 0,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
    };

    // Test with values that would overflow u64 if added naively
    let token_0_value: u64 = u64::MAX / 2 + 1000;
    let token_1_value: u64 = u64::MAX / 2 + 1000;
    
    // This should not panic - saturating_add prevents overflow
    let result = buffer.floor_placement_due(token_0_value, token_1_value);
    
    // The result should be true since the sum exceeds the threshold
    assert!(result);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_floor_placement_due_normal_case, |ctx: TestContext| async move {
    let buffer = Buffer {
        market: Pubkey::default(),
        authority: Pubkey::default(),
        feelssol_mint: Pubkey::default(),
        fees_token_0: 0,
        fees_token_1: 0,
        tau_spot: 0,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 100,
        floor_placement_threshold: 1_000_000,
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 0,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
    };

    // Test with values below threshold
    let token_0_value: u64 = 400_000;
    let token_1_value: u64 = 500_000;
    
    let result = buffer.floor_placement_due(token_0_value, token_1_value);
    
    // Should be false since 400k + 500k = 900k < 1M
    assert!(!result);
    
    // Test with values at threshold
    let token_0_value: u64 = 600_000;
    let token_1_value: u64 = 400_000;
    
    let result = buffer.floor_placement_due(token_0_value, token_1_value);
    
    // Should be true since 600k + 400k = 1M
    assert!(result);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_buffer_tau_overflow_protection, |ctx: TestContext| async move {
    let mut buffer = Buffer {
        market: Pubkey::default(),
        authority: Pubkey::default(),
        feelssol_mint: Pubkey::default(),
        fees_token_0: 0,
        fees_token_1: 0,
        tau_spot: u128::MAX - 1000,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 100,
        floor_placement_threshold: 1_000_000,
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 0,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
    };

    // Test get_total_tau with near-max values
    let total = buffer.get_total_tau();
    assert_eq!(total, u128::MAX - 1000);

    // Test adding more would saturate, not overflow
    buffer.tau_time = 2000;
    let total = buffer.get_total_tau();
    assert_eq!(total, u128::MAX); // Should saturate at MAX
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_buffer_fee_collection_overflow_protection, |ctx: TestContext| async move {
    let mut buffer = Buffer {
        market: Pubkey::default(),
        authority: Pubkey::default(),
        feelssol_mint: Pubkey::default(),
        fees_token_0: u128::MAX - 100,
        fees_token_1: 0,
        tau_spot: 0,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 100,
        floor_placement_threshold: 1_000_000,
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 0,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
    };

    // Test that collect_fee handles overflow correctly
    
    // Try to add 200 when we're at MAX - 100
    let result = buffer.collect_fee(200, 0, FeeDomain::Spot);
    
    // Should return error for overflow
    assert!(result.is_err());
    
    // But adding 50 should work
    let result = buffer.collect_fee(50, 0, FeeDomain::Spot);
    assert!(result.is_ok());
    assert_eq!(buffer.tau_spot, 50);
    assert_eq!(buffer.fees_token_0, u128::MAX - 50);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});