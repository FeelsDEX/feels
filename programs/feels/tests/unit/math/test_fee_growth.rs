//! Tests for fee growth calculation correctness
//! 
//! Ensures that position fee calculations match Uniswap V3 spec

use crate::common::*;
use feels::logic::position_fees::{calculate_position_fee_accrual, PositionFeeAccrual};
use feels::state::Tick;

fn create_tick(fee_growth_0: u128, fee_growth_1: u128) -> Tick {
    Tick {
        liquidity_net: 0,
        liquidity_gross: 0,
        fee_growth_outside_0_x64: fee_growth_0,
        fee_growth_outside_1_x64: fee_growth_1,
        initialized: 1,
        _pad: [0; 15],
    }
}

test_in_memory!(test_fee_growth_inside_range, |ctx: TestContext| async move {
    // Test when current tick is inside the position range
    let current_tick = 100;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1000u128;
    
    // Global fee growth
    let fee_growth_global_0 = 1000u128;
    let fee_growth_global_1 = 2000u128;
    
    // Fee growth outside ticks
    let lower_tick = create_tick(200, 300);  // below tick
    let upper_tick = create_tick(100, 200);  // above tick
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        0, // last fee growth
        0,
    ).unwrap();
    
    // Fee growth inside = global - below - above
    // Token 0: 1000 - 200 - 100 = 700
    // Token 1: 2000 - 300 - 200 = 1500
    assert_eq!(result.fee_growth_inside_0, 700);
    assert_eq!(result.fee_growth_inside_1, 1500);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_fee_growth_below_range, |ctx: TestContext| async move {
    // Test when current tick is below the position range
    let current_tick = 25;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1000u128;
    
    let fee_growth_global_0 = 1000u128;
    let fee_growth_global_1 = 2000u128;
    
    // When below range, only the difference between ticks matters
    let lower_tick = create_tick(600, 900);
    let upper_tick = create_tick(400, 500);
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        0,
        0,
    ).unwrap();
    
    // Fee growth inside = lower_outside - upper_outside
    // Token 0: 600 - 400 = 200
    // Token 1: 900 - 500 = 400
    assert_eq!(result.fee_growth_inside_0, 200);
    assert_eq!(result.fee_growth_inside_1, 400);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_fee_growth_above_range, |ctx: TestContext| async move {
    // Test when current tick is above the position range
    let current_tick = 200;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1000u128;
    
    let fee_growth_global_0 = 1000u128;
    let fee_growth_global_1 = 2000u128;
    
    let lower_tick = create_tick(400, 500);
    let upper_tick = create_tick(600, 900);
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        0,
        0,
    ).unwrap();
    
    // Fee growth inside = upper_outside - lower_outside
    // Token 0: 600 - 400 = 200
    // Token 1: 900 - 500 = 400
    assert_eq!(result.fee_growth_inside_0, 200);
    assert_eq!(result.fee_growth_inside_1, 400);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_fee_delta_calculation, |ctx: TestContext| async move {
    // Test that incremental fee calculation uses wrapping arithmetic correctly
    let current_tick = 100;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1 << 64; // 2^64 for easy calculation
    
    let fee_growth_global_0 = 1000u128;
    let fee_growth_global_1 = 2000u128;
    
    let lower_tick = create_tick(200, 300);
    let upper_tick = create_tick(100, 200);
    
    // Previous fee growth inside
    let last_fee_growth_0 = 500u128;
    let last_fee_growth_1 = 1000u128;
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        last_fee_growth_0,
        last_fee_growth_1,
    ).unwrap();
    
    // Fee growth inside: 700, 1500 (as calculated above)
    // Delta: 700 - 500 = 200, 1500 - 1000 = 500
    // With liquidity = 2^64, tokens owed = delta (no shift needed)
    assert_eq!(result.tokens_owed_0_increment, 200);
    assert_eq!(result.tokens_owed_1_increment, 500);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_saturating_sub_prevents_underflow, |ctx: TestContext| async move {
    // Test edge case where fee growth outside is larger than global
    // This shouldn't happen in normal operation but we handle it safely
    let current_tick = 100;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1000u128;
    
    // Global fee growth is small
    let fee_growth_global_0 = 100u128;
    let fee_growth_global_1 = 200u128;
    
    // Fee growth outside is large (shouldn't happen but test safety)
    let lower_tick = create_tick(150, 250);
    let upper_tick = create_tick(60, 100);
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        0,
        0,
    ).unwrap();
    
    // With saturating sub, this should be 0, not wrap around
    assert_eq!(result.fee_growth_inside_0, 0);
    assert_eq!(result.fee_growth_inside_1, 0);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_wrapping_delta_calculation, |ctx: TestContext| async move {
    // Test that fee delta calculation handles wrapping correctly
    // This simulates a scenario where fee growth has wrapped around u128
    let current_tick = 100;
    let position_tick_lower = 50;
    let position_tick_upper = 150;
    let position_liquidity = 1 << 64;
    
    let fee_growth_global_0 = 500u128; // Wrapped around
    let fee_growth_global_1 = 1000u128;
    
    let lower_tick = create_tick(100, 200);
    let upper_tick = create_tick(50, 100);
    
    // Last fee growth was very high (before wrap)
    let last_fee_growth_0 = u128::MAX - 100;
    let last_fee_growth_1 = u128::MAX - 200;
    
    let result = calculate_position_fee_accrual(
        current_tick,
        position_tick_lower,
        position_tick_upper,
        position_liquidity,
        fee_growth_global_0,
        fee_growth_global_1,
        &lower_tick,
        &upper_tick,
        last_fee_growth_0,
        last_fee_growth_1,
    ).unwrap();
    
    // Current inside: 500 - 100 - 50 = 350
    // Delta with wrapping: 350 - (MAX - 100) = 350 + 100 + 1 = 451
    // This tests that wrapping_sub is used for delta, not regular sub
    assert!(result.tokens_owed_0_increment > 0);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});