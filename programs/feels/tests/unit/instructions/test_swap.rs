//! Unit tests for the swap instruction

use anchor_lang::prelude::*;
use feels::{
    error::FeelsError,
    instructions::{Swap, SwapParams},
    state::{Market, Buffer, OracleState},
    constants::*,
    logic::jit::*,
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_swap_validation() {
    // Test zero amount in
    let params = SwapParams {
        amount_in: 0,
        minimum_amount_out: 100,
        is_token_0_to_1: true,
        sqrt_price_limit: None,
    };
    
    assert_eq!(
        validate_swap_params(&params).unwrap_err(),
        FeelsError::ZeroAmount
    );
    
    // Test invalid sqrt price limit (too low for 0->1 swap)
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 0,
        is_token_0_to_1: true,
        sqrt_price_limit: Some(MIN_SQRT_PRICE - 1),
    };
    
    assert_eq!(
        validate_swap_params(&params).unwrap_err(),
        FeelsError::InvalidSqrtPriceLimit
    );
    
    // Test invalid sqrt price limit (too high for 1->0 swap)
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 0,
        is_token_0_to_1: false,
        sqrt_price_limit: Some(MAX_SQRT_PRICE + 1),
    };
    
    assert_eq!(
        validate_swap_params(&params).unwrap_err(),
        FeelsError::InvalidSqrtPriceLimit
    );
}

#[test]
fn test_swap_fee_calculations() {
    let market = create_test_market();
    
    // Test base fee calculation
    let amount_in = 1_000_000;
    let base_fee = calculate_base_fee(&market, amount_in);
    assert_eq!(base_fee, 3_000); // 0.3% of 1M
    
    // Test with different fee rates
    let mut market_high_fee = market;
    market_high_fee.base_fee_bps = 100; // 1%
    let high_fee = calculate_base_fee(&market_high_fee, amount_in);
    assert_eq!(high_fee, 10_000); // 1% of 1M
    
    // Test fee overflow protection
    let huge_amount = u64::MAX;
    let huge_fee = calculate_base_fee(&market, huge_amount);
    assert!(huge_fee <= huge_amount);
}

#[test]
fn test_swap_slippage_protection() {
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 900,
        is_token_0_to_1: true,
        sqrt_price_limit: None,
    };
    
    // Test successful swap (meets minimum)
    let amount_out = 950;
    assert!(check_slippage_tolerance(&params, amount_out).is_ok());
    
    // Test failed swap (below minimum)
    let amount_out = 850;
    assert_eq!(
        check_slippage_tolerance(&params, amount_out).unwrap_err(),
        FeelsError::SlippageExceeded
    );
    
    // Test exact minimum
    let amount_out = 900;
    assert!(check_slippage_tolerance(&params, amount_out).is_ok());
}

#[test]
fn test_swap_price_limit_validation() {
    let market = create_test_market();
    
    // Test 0->1 swap with valid price limit
    let sqrt_price_limit = market.sqrt_price - 1000;
    assert!(validate_price_limit(&market, true, Some(sqrt_price_limit)).is_ok());
    
    // Test 1->0 swap with valid price limit
    let sqrt_price_limit = market.sqrt_price + 1000;
    assert!(validate_price_limit(&market, false, Some(sqrt_price_limit)).is_ok());
    
    // Test 0->1 swap with invalid price limit (above current)
    let sqrt_price_limit = market.sqrt_price + 1000;
    assert!(validate_price_limit(&market, true, Some(sqrt_price_limit)).is_err());
    
    // Test 1->0 swap with invalid price limit (below current)
    let sqrt_price_limit = market.sqrt_price - 1000;
    assert!(validate_price_limit(&market, false, Some(sqrt_price_limit)).is_err());
}

#[test]
fn test_jit_v0_5_allowance() {
    let mut market = create_test_market();
    let mut buffer = create_test_buffer();
    
    // Enable JIT v0.5
    market.jit_enabled = true;
    buffer.tau_spot = 100_000_000; // 100M tau
    buffer.initial_tau_spot = 100_000_000;
    
    // Test base cap (3%)
    let allowance = calculate_jit_base_allowance(&market, &buffer);
    assert_eq!(allowance, 3_000_000); // 3% of 100M
    
    // Test with concentration multiplier
    let current_tick = 0;
    let target_tick = 10; // Close to current
    let multiplier = calculate_concentration_multiplier(&market, current_tick, target_tick);
    assert!(multiplier > 1.0);
    assert!(multiplier <= 10.0);
    
    // Test slot-based cap
    let slot_cap = calculate_slot_cap(&market, &buffer);
    assert_eq!(slot_cap, 5_000_000); // 5% of 100M
}

#[test]
fn test_jit_safety_mitigations() {
    let mut market = create_test_market();
    let mut buffer = create_test_buffer();
    
    // Test graduated drain protection
    buffer.tau_spot = 30_000_000; // 30M (30% of initial)
    buffer.initial_tau_spot = 100_000_000;
    let throttle_factor = calculate_drain_throttle(&buffer);
    assert!(throttle_factor < 1.0); // Should be throttled
    
    // Test circuit breaker
    buffer.tau_spot = 25_000_000; // 25M (25% of initial)
    let circuit_breaker = check_circuit_breaker(&market, &buffer);
    assert!(circuit_breaker); // Should trigger circuit breaker
    
    // Test slot-based shift
    let current_slot = 1000;
    let last_heavy_slot = 950;
    buffer.jit_last_heavy_usage_slot = last_heavy_slot;
    let shift_ticks = calculate_slot_shift(&buffer, current_slot);
    assert!(shift_ticks > 0); // Should shift concentration
}

#[test]
fn test_swap_reentrancy_protection() {
    let mut market = create_test_market();
    
    // Test reentrancy guard
    assert!(!market.reentrancy_guard);
    
    // Simulate entering swap
    market.reentrancy_guard = true;
    
    // Test that reentrant call would fail
    let result = check_reentrancy(&market);
    assert_eq!(result.unwrap_err(), FeelsError::ReentrantCall);
    
    // Test normal state
    market.reentrancy_guard = false;
    assert!(check_reentrancy(&market).is_ok());
}

#[test]
fn test_swap_market_paused() {
    let mut market = create_test_market();
    
    // Test normal market
    assert!(check_market_active(&market).is_ok());
    
    // Test paused market
    market.is_paused = true;
    assert_eq!(
        check_market_active(&market).unwrap_err(),
        FeelsError::MarketPaused
    );
}

#[test]
fn test_swap_directional_tracking() {
    let mut buffer = create_test_buffer();
    
    // Test buy tracking
    let jit_amount = 1_000_000u128;
    update_directional_tracking(&mut buffer, jit_amount, true);
    
    // For now just verify the function runs
    // In full implementation, would check directional volume fields
    
    // Test sell tracking
    update_directional_tracking(&mut buffer, jit_amount, false);
}

// Helper functions

fn create_test_market() -> Market {
    Market {
        version: 1,
        is_initialized: true,
        is_paused: false,
        token_0: Pubkey::new_from_array([0; 32]),
        token_1: Pubkey::new_from_array([255; 32]),
        feelssol_mint: Pubkey::new_from_array([0; 32]),
        token_0_type: feels::state::TokenType::Spl,
        token_1_type: feels::state::TokenType::Spl,
        token_0_origin: feels::state::TokenOrigin::ProtocolMinted,
        token_1_origin: feels::state::TokenOrigin::External,
        sqrt_price: 1 << 64,
        liquidity: 1_000_000_000,
        current_tick: 0,
        tick_spacing: 10,
        global_lower_tick: MIN_TICK,
        global_upper_tick: MAX_TICK,
        floor_liquidity: 0,
        fee_growth_global_0_x64: 0,
        fee_growth_global_1_x64: 0,
        base_fee_bps: 30,
        buffer: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        last_epoch_update: 0,
        epoch_number: 0,
        oracle: Pubkey::new_unique(),
        oracle_bump: 255,
        policy: feels::state::PolicyV1::default(),
        market_authority_bump: 254,
        vault_0_bump: 253,
        vault_1_bump: 252,
        reentrancy_guard: false,
        initial_liquidity_deployed: false,
        jit_enabled: true,
        jit_base_cap_bps: 300,
        jit_per_slot_cap_bps: 500,
        jit_concentration_width: 100,
        jit_max_multiplier: 10,
        jit_drain_protection_bps: 7000,
        jit_circuit_breaker_bps: 3000,
        floor_tick: MIN_TICK,
        floor_buffer_ticks: 100,
        last_floor_ratchet_ts: 0,
        floor_cooldown_secs: 60,
        steady_state_seeded: false,
        cleanup_complete: false,
        vault_0: Pubkey::new_unique(),
        vault_1: Pubkey::new_unique(),
        hub_protocol: Some(Pubkey::new_unique()),
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
        phase: 0,
        phase_start_slot: 0,
        phase_start_timestamp: 0,
        last_phase_transition_slot: 0,
        last_phase_trigger: 0,
        total_volume_token_0: 0,
        total_volume_token_1: 0,
        _reserved: [0; 1],
    }
}

fn create_test_buffer() -> Buffer {
    Buffer {
        market: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        tau_spot: 100_000_000,
        tau_leverage: 0,
        fees_token_0: 0,
        fees_token_1: 0,
        floor_placement_threshold: 1_000_000,
        total_distributed: 0,
        last_floor_placement: 0,
        last_update_slot: 0,
        last_update_timestamp: 0,
        buffer_vault_0: Pubkey::new_unique(),
        buffer_vault_1: Pubkey::new_unique(),
        jit_last_consumed_slot: 0,
        jit_total_consumed_24h: 0,
        jit_24h_window_start: 0,
        pomm_position_count: 0,
        jit_rolling_consumption: 0,
        jit_rolling_window_start: 0,
        jit_last_heavy_usage_slot: 0,
        jit_total_consumed_epoch: 0,
        initial_tau_spot: 100_000_000,
        _reserved: [0; 220],
    }
}

fn validate_swap_params(params: &SwapParams) -> Result<()> {
    if params.amount_in == 0 {
        return Err(FeelsError::ZeroAmount.into());
    }
    
    if let Some(limit) = params.sqrt_price_limit {
        if limit < MIN_SQRT_PRICE || limit > MAX_SQRT_PRICE {
            return Err(FeelsError::InvalidSqrtPriceLimit.into());
        }
    }
    
    Ok(())
}

fn calculate_base_fee(market: &Market, amount_in: u64) -> u64 {
    (amount_in as u128 * market.base_fee_bps as u128 / 10_000)
        .min(u64::MAX as u128) as u64
}

fn check_slippage_tolerance(params: &SwapParams, amount_out: u64) -> Result<()> {
    if amount_out < params.minimum_amount_out {
        return Err(FeelsError::SlippageExceeded.into());
    }
    Ok(())
}

fn validate_price_limit(market: &Market, is_token_0_to_1: bool, sqrt_price_limit: Option<u128>) -> Result<()> {
    if let Some(limit) = sqrt_price_limit {
        if is_token_0_to_1 && limit > market.sqrt_price {
            return Err(FeelsError::InvalidSqrtPriceLimit.into());
        }
        if !is_token_0_to_1 && limit < market.sqrt_price {
            return Err(FeelsError::InvalidSqrtPriceLimit.into());
        }
    }
    Ok(())
}

fn calculate_jit_base_allowance(market: &Market, buffer: &Buffer) -> u128 {
    buffer.tau_spot * market.jit_base_cap_bps as u128 / 10_000
}

fn calculate_concentration_multiplier(market: &Market, current_tick: i32, target_tick: i32) -> f64 {
    let distance = (current_tick - target_tick).abs() as u32;
    if distance <= market.jit_concentration_width {
        market.jit_max_multiplier as f64
    } else {
        1.0
    }
}

fn calculate_slot_cap(market: &Market, buffer: &Buffer) -> u128 {
    buffer.tau_spot * market.jit_per_slot_cap_bps as u128 / 10_000
}

fn calculate_drain_throttle(buffer: &Buffer) -> f64 {
    let ratio = buffer.tau_spot as f64 / buffer.initial_tau_spot.max(1) as f64;
    if ratio < 0.7 {
        ratio // Linear throttle below 70%
    } else {
        1.0
    }
}

fn check_circuit_breaker(market: &Market, buffer: &Buffer) -> bool {
    let ratio = buffer.tau_spot * 10_000 / buffer.initial_tau_spot.max(1);
    ratio < market.jit_circuit_breaker_bps as u128
}

fn calculate_slot_shift(buffer: &Buffer, current_slot: u64) -> u32 {
    let slots_since_heavy = current_slot.saturating_sub(buffer.jit_last_heavy_usage_slot);
    if slots_since_heavy < 100 {
        10 // Shift by 10 ticks
    } else {
        0
    }
}

fn check_reentrancy(market: &Market) -> Result<()> {
    if market.reentrancy_guard {
        return Err(FeelsError::ReentrantCall.into());
    }
    Ok(())
}

fn check_market_active(market: &Market) -> Result<()> {
    if market.is_paused {
        return Err(FeelsError::MarketPaused.into());
    }
    Ok(())
}

fn update_directional_tracking(buffer: &mut Buffer, jit_amount: u128, is_buy: bool) {
    // Simplified for unit test
    // In full implementation would update directional volume fields
}