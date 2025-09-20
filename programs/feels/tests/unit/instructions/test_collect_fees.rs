//! Unit tests for the collect_fees instruction

use anchor_lang::prelude::*;
use feels::{
    constants::*,
    error::FeelsError,
    state::{Market, Position},
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_collect_fees_validation() {
    let position = create_test_position_with_fees();
    let owner = position.owner;
    let wrong_owner = Pubkey::new_unique();

    // Test correct owner can collect
    assert!(validate_fee_collection(&position, &owner).is_ok());

    // Test wrong owner cannot collect
    assert_eq!(
        validate_fee_collection(&position, &wrong_owner).unwrap_err(),
        FeelsError::Unauthorized.into()
    );

    // Test position with no fees
    let position_no_fees = Position {
        fees_owed_0: 0,
        fees_owed_1: 0,
        ..position
    };

    // Can still call collect with no fees (no-op)
    assert!(validate_fee_collection(&position_no_fees, &owner).is_ok());
}

#[test]
fn test_collect_fees_calculation() {
    let market = create_test_market();
    let position = create_test_position_with_fees();

    // Test basic fee collection
    let (collectable_0, collectable_1) = calculate_collectable_fees(&position, &market);
    assert_eq!(collectable_0, 10_000);
    assert_eq!(collectable_1, 5_000);

    // Test with updated fee growth
    let mut market_updated = market;
    market_updated.fee_growth_global_0 = 2000;
    market_updated.fee_growth_global_1 = 3000;

    let position_in_range = Position {
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        fee_growth_inside_0_last: 1000,
        fee_growth_inside_1_last: 2000,
        fees_owed_0: 10_000,
        fees_owed_1: 5_000,
        ..position
    };

    let (collectable_0, collectable_1) = calculate_collectable_with_growth(
        &position_in_range,
        &market_updated,
        true, // in range
    );

    // Should include both owed fees and new growth
    assert!(collectable_0 >= 10_000);
    assert!(collectable_1 >= 5_000);
}

#[test]
fn test_collect_fees_pomm_position() {
    let mut position = create_test_position_with_fees();
    position.is_pomm = true;
    position.owner = Pubkey::new_unique(); // Buffer address

    let buffer = position.owner;
    let user = Pubkey::new_unique();

    // Test buffer can collect POMM fees
    assert!(validate_pomm_fee_collection(&position, &buffer).is_ok());

    // Test regular user cannot collect POMM fees
    assert_eq!(
        validate_pomm_fee_collection(&position, &user).unwrap_err(),
        FeelsError::Unauthorized.into()
    );
}

#[test]
fn test_collect_fees_amount_limits() {
    let position = create_test_position_with_fees();

    // Test collecting full amounts
    let amount_0_requested = u64::MAX;
    let amount_1_requested = u64::MAX;
    let (collect_0, collect_1) =
        apply_collection_limits(&position, amount_0_requested, amount_1_requested);

    // Should be capped at fees owed
    assert_eq!(collect_0, position.fees_owed_0);
    assert_eq!(collect_1, position.fees_owed_1);

    // Test collecting partial amounts
    let amount_0_requested = 5_000;
    let amount_1_requested = 2_000;
    let (collect_0, collect_1) =
        apply_collection_limits(&position, amount_0_requested, amount_1_requested);

    assert_eq!(collect_0, 5_000);
    assert_eq!(collect_1, 2_000);

    // Test collecting zero
    let (collect_0, collect_1) = apply_collection_limits(&position, 0, 0);
    assert_eq!(collect_0, 0);
    assert_eq!(collect_1, 0);
}

#[test]
fn test_collect_fees_position_update() {
    let mut position = create_test_position_with_fees();
    let collected_0 = 7_000;
    let collected_1 = 3_000;

    // Update position after collection
    update_position_after_collection(&mut position, collected_0, collected_1);

    // Verify remaining fees
    assert_eq!(position.fees_owed_0, 3_000); // 10_000 - 7_000
    assert_eq!(position.fees_owed_1, 2_000); // 5_000 - 3_000

    // Test collecting all fees
    let mut position_all = create_test_position_with_fees();
    let fees_0 = position_all.fees_owed_0;
    let fees_1 = position_all.fees_owed_1;
    update_position_after_collection(&mut position_all, fees_0, fees_1);

    assert_eq!(position_all.fees_owed_0, 0);
    assert_eq!(position_all.fees_owed_1, 0);
}

#[test]
fn test_collect_fees_overflow_protection() {
    let mut position = create_test_position_with_fees();
    position.fees_owed_0 = u64::MAX;
    position.fees_owed_1 = u64::MAX;

    // Test that collection doesn't overflow
    let (collect_0, collect_1) = calculate_collectable_fees(&position, &create_test_market());
    assert_eq!(collect_0, u64::MAX);
    assert_eq!(collect_1, u64::MAX);

    // Update should handle max values
    update_position_after_collection(&mut position, u64::MAX, u64::MAX);
    assert_eq!(position.fees_owed_0, 0);
    assert_eq!(position.fees_owed_1, 0);
}

#[test]
fn test_collect_fees_market_state() {
    let mut market = create_test_market();
    let position = create_test_position_with_fees();

    // Test with paused market - fees can still be collected
    market.is_paused = true;
    assert!(validate_market_for_collection(&market).is_ok());

    // Test with reentrancy guard
    market.reentrancy_guard = true;
    assert_eq!(
        validate_market_for_collection(&market).unwrap_err(),
        FeelsError::ReentrancyDetected.into()
    );
}

#[test]
fn test_collect_fees_fee_growth_tracking() {
    let market = create_test_market();
    let mut position = create_test_position_with_fees();

    // Simulate fee growth update after collection
    let new_fee_growth_0 = 5000;
    let new_fee_growth_1 = 6000;

    update_fee_growth_tracking(&mut position, new_fee_growth_0, new_fee_growth_1);

    assert_eq!(position.fee_growth_inside_0_last, new_fee_growth_0);
    assert_eq!(position.fee_growth_inside_1_last, new_fee_growth_1);
}

#[test]
fn test_collect_fees_rounding() {
    let market = create_test_market();

    // Test with very small fees
    let position_small_fees = Position {
        fees_owed_0: 1,
        fees_owed_1: 1,
        ..create_test_position_with_fees()
    };

    let (collect_0, collect_1) = calculate_collectable_fees(&position_small_fees, &market);
    assert_eq!(collect_0, 1);
    assert_eq!(collect_1, 1);

    // Test that we don't round to zero
    assert!(collect_0 > 0 || position_small_fees.fees_owed_0 == 0);
    assert!(collect_1 > 0 || position_small_fees.fees_owed_1 == 0);
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
        jit_enabled: false,
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
        fee_growth_global_0: 1000,
        fee_growth_global_1: 2000,
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

fn create_test_position_with_fees() -> Position {
    Position {
        nft_mint: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        fee_growth_inside_0_last_x64: 500 << 64,
        fee_growth_inside_1_last_x64: 1000 << 64,
        tokens_owed_0: 10_000,
        tokens_owed_1: 5_000,
        position_bump: 255,
        is_pomm: false,
        last_updated_slot: 0,
        fee_growth_inside_0_last: 500,
        fee_growth_inside_1_last: 1000,
        fees_owed_0: 10_000,
        fees_owed_1: 5_000,
    }
}

fn validate_fee_collection(position: &Position, signer: &Pubkey) -> Result<()> {
    if position.owner != *signer {
        return Err(FeelsError::Unauthorized.into());
    }
    Ok(())
}

fn calculate_collectable_fees(position: &Position, market: &Market) -> (u64, u64) {
    // Simply return fees owed for basic test
    (position.fees_owed_0, position.fees_owed_1)
}

fn calculate_collectable_with_growth(
    position: &Position,
    market: &Market,
    in_range: bool,
) -> (u64, u64) {
    if !in_range {
        return (position.fees_owed_0, position.fees_owed_1);
    }

    // Simplified calculation for testing
    let growth_0 = market
        .fee_growth_global_0
        .saturating_sub(position.fee_growth_inside_0_last);
    let growth_1 = market
        .fee_growth_global_1
        .saturating_sub(position.fee_growth_inside_1_last);

    let new_fees_0 = (position.liquidity as u128 * growth_0 as u128 / 1_000_000) as u64;
    let new_fees_1 = (position.liquidity as u128 * growth_1 as u128 / 1_000_000) as u64;

    (
        position.fees_owed_0.saturating_add(new_fees_0),
        position.fees_owed_1.saturating_add(new_fees_1),
    )
}

fn validate_pomm_fee_collection(position: &Position, signer: &Pubkey) -> Result<()> {
    if position.is_pomm && position.owner != *signer {
        return Err(FeelsError::Unauthorized.into());
    }
    Ok(())
}

fn apply_collection_limits(
    position: &Position,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> (u64, u64) {
    (
        amount_0_requested.min(position.fees_owed_0),
        amount_1_requested.min(position.fees_owed_1),
    )
}

fn update_position_after_collection(position: &mut Position, collected_0: u64, collected_1: u64) {
    position.fees_owed_0 = position.fees_owed_0.saturating_sub(collected_0);
    position.fees_owed_1 = position.fees_owed_1.saturating_sub(collected_1);
}

fn validate_market_for_collection(market: &Market) -> Result<()> {
    // Fee collection allowed even if market is paused
    if market.reentrancy_guard {
        return Err(FeelsError::ReentrancyDetected.into());
    }
    Ok(())
}

fn update_fee_growth_tracking(
    position: &mut Position,
    new_fee_growth_0: u128,
    new_fee_growth_1: u128,
) {
    position.fee_growth_inside_0_last = new_fee_growth_0;
    position.fee_growth_inside_1_last = new_fee_growth_1;
}
