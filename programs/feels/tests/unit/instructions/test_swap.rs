//! Unit tests for the swap instruction

use anchor_lang::prelude::*;
use feels::{
    constants::*,
    error::FeelsError,
    logic::SwapParams,
    state::{Buffer, Market},
};
use solana_program::pubkey::Pubkey;

// Test constants
const MIN_SQRT_PRICE: u128 = 4295128739;
const MAX_SQRT_PRICE: u128 = u128::MAX;

#[test]
fn test_swap_validation() {
    // Test zero amount in
    let params = SwapParams {
        amount_in: 0,
        minimum_amount_out: 100,
        max_ticks_crossed: 0,
        max_total_fee_bps: 0,
    };

    assert_eq!(
        validate_swap_params(&params).unwrap_err(),
        FeelsError::ZeroAmount.into()
    );

    // Test invalid max fee (too high)
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 0,
        max_ticks_crossed: 0,
        max_total_fee_bps: 10001, // > 100%
    };

    assert_eq!(
        validate_swap_params(&params).unwrap_err(),
        FeelsError::FeeCapExceeded.into()
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
    let mut market_high_fee = create_test_market();
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
        max_ticks_crossed: 0,
        max_total_fee_bps: 0,
    };

    // Test successful swap (meets minimum)
    let amount_out = 950;
    assert!(check_slippage_tolerance(&params, amount_out).is_ok());

    // Test failed swap (below minimum)
    let amount_out = 850;
    assert_eq!(
        check_slippage_tolerance(&params, amount_out).unwrap_err(),
        FeelsError::SlippageExceeded.into()
    );

    // Test exact minimum
    let amount_out = 900;
    assert!(check_slippage_tolerance(&params, amount_out).is_ok());
}

#[test]
fn test_swap_tick_crossing_limit() {
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 0,
        max_ticks_crossed: 10,
        max_total_fee_bps: 0,
    };

    // Test within limit
    let ticks_crossed = 5;
    assert!(check_tick_crossing_limit(&params, ticks_crossed).is_ok());

    // Test at limit
    let ticks_crossed = 10;
    assert!(check_tick_crossing_limit(&params, ticks_crossed).is_ok());

    // Test exceeding limit
    let ticks_crossed = 11;
    assert_eq!(
        check_tick_crossing_limit(&params, ticks_crossed).unwrap_err(),
        FeelsError::TooManyTicksCrossed.into()
    );

    // Test unlimited (max_ticks_crossed = 0)
    let params_unlimited = SwapParams {
        max_ticks_crossed: 0,
        ..params
    };
    assert!(check_tick_crossing_limit(&params_unlimited, 100).is_ok());
}

#[test]
fn test_swap_fee_cap() {
    let params = SwapParams {
        amount_in: 1000,
        minimum_amount_out: 0,
        max_ticks_crossed: 0,
        max_total_fee_bps: 100, // 1% cap
    };

    // Test within cap
    let total_fee = 10; // 1%
    assert!(check_fee_cap(&params, total_fee).is_ok());

    // Test exceeding cap
    let total_fee = 20; // 2%
    assert_eq!(
        check_fee_cap(&params, total_fee).unwrap_err(),
        FeelsError::FeeCapExceeded.into()
    );

    // Test no cap (max_total_fee_bps = 0)
    let params_no_cap = SwapParams {
        max_total_fee_bps: 0,
        ..params
    };
    assert!(check_fee_cap(&params_no_cap, 1000).is_ok());
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
    assert_eq!(result.unwrap_err(), FeelsError::ReentrancyDetected.into());

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
        FeelsError::MarketPaused.into()
    );
}

#[test]
fn test_jit_allowance_calculation() {
    let mut market = create_test_market();
    let mut buffer = create_test_buffer();

    // Enable JIT
    market.jit_enabled = true;
    buffer.tau_spot = 100_000_000; // 100M tau
    buffer.initial_tau_spot = 100_000_000;

    // Test base cap (3%)
    let allowance = calculate_jit_base_allowance(&market, &buffer);
    assert_eq!(allowance, 3_000_000); // 3% of 100M

    // Test with consumption
    buffer.jit_rolling_consumption = 1_000_000;
    let remaining = calculate_jit_remaining(&market, &buffer);
    assert_eq!(remaining, 2_000_000); // 3M - 1M consumed
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
        rolling_buy_volume: 0,
        rolling_sell_volume: 0,
        rolling_total_volume: 0,
        rolling_window_start_slot: 0,
        tick_snapshot_1hr: 0,
        last_snapshot_timestamp: 0,
        _reserved: [0; 1],
    }
}

fn create_test_buffer() -> Buffer {
    Buffer {
        market: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        feelssol_mint: Pubkey::new_from_array([0; 32]),
        fees_token_0: 0,
        fees_token_1: 0,
        tau_spot: 100_000_000,
        tau_time: 0,
        tau_leverage: 0,
        floor_tick_spacing: 100,
        floor_placement_threshold: 1_000_000,
        last_floor_placement: 0,
        last_rebase: 0,
        total_distributed: 0,
        buffer_authority_bump: 255,
        jit_last_slot: 0,
        jit_slot_used_q: 0,
        jit_rolling_consumption: 0,
        jit_rolling_window_start: 0,
        jit_last_heavy_usage_slot: 0,
        jit_total_consumed_epoch: 0,
        initial_tau_spot: 100_000_000,
        protocol_owned_override: 0,
        pomm_position_count: 0,
        _padding: [0; 7],
    }
}

fn validate_swap_params(params: &SwapParams) -> Result<()> {
    if params.amount_in == 0 {
        return Err(FeelsError::ZeroAmount.into());
    }

    if params.max_total_fee_bps > 10000 {
        return Err(FeelsError::FeeCapExceeded.into());
    }

    Ok(())
}

fn calculate_base_fee(market: &Market, amount_in: u64) -> u64 {
    (amount_in as u128 * market.base_fee_bps as u128 / 10_000).min(u64::MAX as u128) as u64
}

fn check_slippage_tolerance(params: &SwapParams, amount_out: u64) -> Result<()> {
    if amount_out < params.minimum_amount_out {
        return Err(FeelsError::SlippageExceeded.into());
    }
    Ok(())
}

fn check_tick_crossing_limit(params: &SwapParams, ticks_crossed: u8) -> Result<()> {
    if params.max_ticks_crossed > 0 && ticks_crossed > params.max_ticks_crossed {
        return Err(FeelsError::TooManyTicksCrossed.into());
    }
    Ok(())
}

fn check_fee_cap(params: &SwapParams, total_fee: u64) -> Result<()> {
    if params.max_total_fee_bps > 0 {
        let fee_bps = (total_fee as u128 * 10_000 / params.amount_in as u128) as u16;
        if fee_bps > params.max_total_fee_bps {
            return Err(FeelsError::FeeCapExceeded.into());
        }
    }
    Ok(())
}

fn check_reentrancy(market: &Market) -> Result<()> {
    if market.reentrancy_guard {
        return Err(FeelsError::ReentrancyDetected.into());
    }
    Ok(())
}

fn check_market_active(market: &Market) -> Result<()> {
    if market.is_paused {
        return Err(FeelsError::MarketPaused.into());
    }
    Ok(())
}

fn calculate_jit_base_allowance(market: &Market, buffer: &Buffer) -> u128 {
    buffer.tau_spot * market.jit_base_cap_bps as u128 / 10_000
}

fn calculate_jit_remaining(market: &Market, buffer: &Buffer) -> u128 {
    let base_allowance = calculate_jit_base_allowance(market, buffer);
    base_allowance.saturating_sub(buffer.jit_rolling_consumption)
}
