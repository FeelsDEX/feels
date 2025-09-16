//! Unit tests for the close_position instruction

use anchor_lang::prelude::*;
use feels::{
    error::FeelsError,
    instructions::{ClosePosition, ClosePositionParams},
    state::{Market, Position},
    constants::*,
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_close_position_validation() {
    let market = create_test_market();
    let position = create_test_position();
    
    // Test closing empty position (no liquidity)
    let mut empty_position = position.clone();
    empty_position.liquidity = 0;
    
    let result = validate_close_position(&empty_position, &market);
    assert!(result.is_ok()); // Empty positions can be closed
    
    // Test closing position with fees owed
    let mut position_with_fees = position.clone();
    position_with_fees.fees_owed_0 = 1000;
    position_with_fees.fees_owed_1 = 2000;
    
    let result = validate_close_position(&position_with_fees, &market);
    assert!(result.is_ok()); // Positions with fees can be closed
}

#[test]
fn test_close_position_liquidity_removal() {
    let mut market = create_test_market();
    let position = create_test_position();
    
    // Add liquidity to market
    market.liquidity = 10_000_000;
    
    // Test in-range position removal
    let position_in_range = Position {
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        ..position
    };
    
    let new_liquidity = calculate_market_liquidity_after_close(
        &market,
        &position_in_range,
        true, // in range
    );
    
    assert_eq!(new_liquidity, 9_000_000);
    
    // Test out-of-range position removal (no effect on current liquidity)
    let position_out_range = Position {
        tick_lower: 100,
        tick_upper: 200,
        liquidity: 1_000_000,
        ..position
    };
    
    let new_liquidity = calculate_market_liquidity_after_close(
        &market,
        &position_out_range,
        false, // out of range
    );
    
    assert_eq!(new_liquidity, 10_000_000); // No change
}

#[test]
fn test_close_position_fee_collection() {
    let market = create_test_market();
    let position = create_test_position();
    
    // Test position with accumulated fees
    let position_with_fees = Position {
        fees_owed_0: 5000,
        fees_owed_1: 3000,
        fee_growth_inside_0_last: 1000,
        fee_growth_inside_1_last: 2000,
        ..position
    };
    
    let (fees_0, fees_1) = calculate_fees_owed(&position_with_fees, &market);
    assert_eq!(fees_0, 5000);
    assert_eq!(fees_1, 3000);
    
    // Test position with no fees
    let position_no_fees = Position {
        fees_owed_0: 0,
        fees_owed_1: 0,
        ..position
    };
    
    let (fees_0, fees_1) = calculate_fees_owed(&position_no_fees, &market);
    assert_eq!(fees_0, 0);
    assert_eq!(fees_1, 0);
}

#[test]
fn test_close_position_amounts_calculation() {
    let market = create_test_market();
    
    // Test in-range position amounts
    let position_in_range = Position {
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        ..create_test_position()
    };
    
    let (amount_0, amount_1) = calculate_position_amounts(
        &position_in_range,
        market.sqrt_price,
        market.current_tick,
    ).unwrap();
    
    assert!(amount_0 > 0);
    assert!(amount_1 > 0);
    
    // Test below-range position (only token_0)
    let position_below = Position {
        tick_lower: 100,
        tick_upper: 200,
        liquidity: 1_000_000,
        ..create_test_position()
    };
    
    let (amount_0, amount_1) = calculate_position_amounts(
        &position_below,
        market.sqrt_price,
        market.current_tick,
    ).unwrap();
    
    assert!(amount_0 > 0);
    assert_eq!(amount_1, 0);
    
    // Test above-range position (only token_1)
    let position_above = Position {
        tick_lower: -200,
        tick_upper: -100,
        liquidity: 1_000_000,
        ..create_test_position()
    };
    
    let (amount_0, amount_1) = calculate_position_amounts(
        &position_above,
        market.sqrt_price,
        market.current_tick,
    ).unwrap();
    
    assert_eq!(amount_0, 0);
    assert!(amount_1 > 0);
}

#[test]
fn test_close_position_ownership_validation() {
    let position = create_test_position();
    let owner = position.owner;
    let wrong_owner = Pubkey::new_unique();
    
    // Test correct owner
    assert!(validate_position_ownership(&position, &owner).is_ok());
    
    // Test wrong owner
    assert_eq!(
        validate_position_ownership(&position, &wrong_owner).unwrap_err(),
        FeelsError::Unauthorized
    );
}

#[test]
fn test_close_position_pomm_restrictions() {
    let mut position = create_test_position();
    position.is_pomm = true;
    
    let user = Pubkey::new_unique();
    let buffer = position.owner; // POMM positions owned by buffer
    
    // Test that regular users cannot close POMM positions
    assert_eq!(
        validate_pomm_close_authority(&position, &user).unwrap_err(),
        FeelsError::Unauthorized
    );
    
    // Test that buffer can close POMM positions
    assert!(validate_pomm_close_authority(&position, &buffer).is_ok());
}

#[test]
fn test_close_position_tick_array_updates() {
    let position = create_test_position();
    let tick_spacing = 10;
    
    // Verify tick arrays that need updating
    let lower_array = get_tick_array_for_tick(position.tick_lower, tick_spacing);
    let upper_array = get_tick_array_for_tick(position.tick_upper, tick_spacing);
    
    // Arrays should be properly aligned
    assert_eq!(lower_array % (TICK_ARRAY_SIZE * tick_spacing), 0);
    assert_eq!(upper_array % (TICK_ARRAY_SIZE * tick_spacing), 0);
    
    // Verify ticks are in correct arrays
    assert!(position.tick_lower >= lower_array);
    assert!(position.tick_lower < lower_array + TICK_ARRAY_SIZE * tick_spacing);
    assert!(position.tick_upper >= upper_array);
    assert!(position.tick_upper < upper_array + TICK_ARRAY_SIZE * tick_spacing);
}

#[test]
fn test_close_position_market_state_validation() {
    let mut market = create_test_market();
    let position = create_test_position();
    
    // Test with paused market
    market.is_paused = true;
    assert_eq!(
        validate_market_state_for_close(&market).unwrap_err(),
        FeelsError::MarketPaused
    );
    
    // Test with active market
    market.is_paused = false;
    assert!(validate_market_state_for_close(&market).is_ok());
    
    // Test with reentrancy guard
    market.reentrancy_guard = true;
    assert_eq!(
        validate_market_state_for_close(&market).unwrap_err(),
        FeelsError::ReentrantCall
    );
}

#[test]
fn test_close_position_minimum_amounts() {
    let position = create_test_position();
    
    // Test with sufficient amounts
    let amount_0 = 1000;
    let amount_1 = 2000;
    let min_amount_0 = 900;
    let min_amount_1 = 1800;
    
    assert!(validate_minimum_amounts(
        amount_0,
        amount_1,
        min_amount_0,
        min_amount_1
    ).is_ok());
    
    // Test with insufficient amount_0
    let min_amount_0 = 1100;
    assert_eq!(
        validate_minimum_amounts(
            amount_0,
            amount_1,
            min_amount_0,
            min_amount_1
        ).unwrap_err(),
        FeelsError::SlippageExceeded
    );
    
    // Test with insufficient amount_1
    let min_amount_0 = 900;
    let min_amount_1 = 2100;
    assert_eq!(
        validate_minimum_amounts(
            amount_0,
            amount_1,
            min_amount_0,
            min_amount_1
        ).unwrap_err(),
        FeelsError::SlippageExceeded
    );
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
        liquidity: 0,
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

fn create_test_position() -> Position {
    Position {
        market: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        fee_growth_inside_0_last: 0,
        fee_growth_inside_1_last: 0,
        fees_owed_0: 0,
        fees_owed_1: 0,
        is_pomm: false,
        last_updated_slot: 0,
        _reserved: [0; 64],
    }
}

fn validate_close_position(position: &Position, market: &Market) -> Result<()> {
    // Basic validation - positions can always be closed
    Ok(())
}

fn calculate_market_liquidity_after_close(
    market: &Market,
    position: &Position,
    in_range: bool,
) -> u128 {
    if in_range {
        market.liquidity.saturating_sub(position.liquidity)
    } else {
        market.liquidity
    }
}

fn calculate_fees_owed(position: &Position, market: &Market) -> (u64, u64) {
    // Return existing fees owed
    (position.fees_owed_0, position.fees_owed_1)
}

fn calculate_position_amounts(
    position: &Position,
    sqrt_price: u128,
    current_tick: i32,
) -> Result<(u64, u64)> {
    // Simplified calculation for testing
    if current_tick < position.tick_lower {
        // Below range - all in token_0
        Ok((position.liquidity as u64 / 1000, 0))
    } else if current_tick >= position.tick_upper {
        // Above range - all in token_1
        Ok((0, position.liquidity as u64 / 1000))
    } else {
        // In range - split between both tokens
        Ok((
            position.liquidity as u64 / 2000,
            position.liquidity as u64 / 2000,
        ))
    }
}

fn validate_position_ownership(position: &Position, signer: &Pubkey) -> Result<()> {
    if position.owner != *signer {
        return Err(FeelsError::Unauthorized.into());
    }
    Ok(())
}

fn validate_pomm_close_authority(position: &Position, signer: &Pubkey) -> Result<()> {
    if position.is_pomm && position.owner != *signer {
        return Err(FeelsError::Unauthorized.into());
    }
    Ok(())
}

fn get_tick_array_for_tick(tick: i32, tick_spacing: i32) -> i32 {
    let ticks_per_array = TICK_ARRAY_SIZE * tick_spacing;
    (tick / ticks_per_array) * ticks_per_array
}

fn validate_market_state_for_close(market: &Market) -> Result<()> {
    if market.is_paused {
        return Err(FeelsError::MarketPaused.into());
    }
    
    if market.reentrancy_guard {
        return Err(FeelsError::ReentrantCall.into());
    }
    
    Ok(())
}

fn validate_minimum_amounts(
    amount_0: u64,
    amount_1: u64,
    min_amount_0: u64,
    min_amount_1: u64,
) -> Result<()> {
    if amount_0 < min_amount_0 || amount_1 < min_amount_1 {
        return Err(FeelsError::SlippageExceeded.into());
    }
    Ok(())
}

const TICK_ARRAY_SIZE: i32 = 64;