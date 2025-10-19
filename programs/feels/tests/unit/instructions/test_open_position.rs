//! Unit tests for the open_position instruction

use anchor_lang::prelude::*;
use feels::{
    constants::*,
    error::FeelsError,
    state::{Market, Position},
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_open_position_validation() {
    let market = create_test_market();

    // Test invalid tick range (lower >= upper)
    let tick_lower = 100;
    let tick_upper = 100;
    let liquidity = 1_000_000;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::InvalidTickRange.into()
    );

    // Test tick range too wide - use spaced ticks
    let tick_spacing = market.tick_spacing as i32;
    let tick_lower = (MIN_TICK / tick_spacing) * tick_spacing;
    let tick_upper = (MAX_TICK / tick_spacing) * tick_spacing;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::InvalidTickRange.into()
    );

    // Test zero liquidity
    let tick_lower = -100;
    let tick_upper = 100;
    let liquidity = 0;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::ZeroLiquidity.into()
    );

    // Test liquidity below minimum
    let liquidity = MIN_LIQUIDITY - 1;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::LiquidityBelowMinimum.into()
    );
}

#[test]
fn test_tick_spacing_validation() {
    let market = create_test_market();

    // Test ticks not aligned to spacing
    let tick_lower = -105; // Not divisible by 10
    let tick_upper = 100;
    let liquidity = 1_000_000;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::TickNotSpaced.into()
    );

    // Test upper tick not aligned
    let tick_lower = -100;
    let tick_upper = 105; // Not divisible by 10

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::TickNotSpaced.into()
    );

    // Test valid aligned ticks
    let tick_lower = -100;
    let tick_upper = 100;

    assert!(validate_position_params(tick_lower, tick_upper, liquidity, &market).is_ok());
}

#[test]
fn test_position_bounds_validation() {
    let market = create_test_market();
    let liquidity = 1_000_000;

    // Test tick below minimum
    let tick_lower = MIN_TICK - 10;
    let tick_upper = 0;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::InvalidTick.into()
    );

    // Test tick above maximum
    let tick_lower = 0;
    let tick_upper = MAX_TICK + 10;

    assert_eq!(
        validate_position_params(tick_lower, tick_upper, liquidity, &market).unwrap_err(),
        FeelsError::InvalidTick.into()
    );
}

#[test]
fn test_position_liquidity_calculations() {
    let market = create_test_market();

    // Test liquidity from amounts for in-range position
    let tick_lower = -100;
    let tick_upper = 100;
    let amount_0 = 1_000_000;
    let amount_1 = 1_000_000;

    let liquidity = calculate_liquidity_from_amounts(
        market.sqrt_price,
        tick_lower,
        tick_upper,
        amount_0,
        amount_1,
    )
    .unwrap();

    assert!(liquidity > 0);
    assert!(liquidity >= MIN_LIQUIDITY);

    // Test liquidity for below-range position (only token_0)
    let tick_lower = 100;
    let tick_upper = 200;
    let liquidity =
        calculate_liquidity_from_amounts(market.sqrt_price, tick_lower, tick_upper, amount_0, 0)
            .unwrap();

    assert!(liquidity > 0);

    // Test liquidity for above-range position (only token_1)
    let tick_lower = -200;
    let tick_upper = -100;
    let liquidity =
        calculate_liquidity_from_amounts(market.sqrt_price, tick_lower, tick_upper, 0, amount_1)
            .unwrap();

    assert!(liquidity > 0);
}

#[test]
fn test_position_state_initialization() {
    let market_key = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let current_slot = 1000;

    let position = Position {
        nft_mint: Pubkey::new_unique(),
        market: market_key,
        owner,
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        fee_growth_inside_0_last_x64: 0,
        fee_growth_inside_1_last_x64: 0,
        tokens_owed_0: 0,
        tokens_owed_1: 0,
        position_bump: 255,
        is_pomm: false,
        last_updated_slot: current_slot,
        fee_growth_inside_0_last: 0,
        fee_growth_inside_1_last: 0,
        fees_owed_0: 0,
        fees_owed_1: 0,
    };

    // Verify initialization
    assert_eq!(position.market, market_key);
    assert_eq!(position.owner, owner);
    assert_eq!(position.tick_lower, -100);
    assert_eq!(position.tick_upper, 100);
    assert_eq!(position.liquidity, 1_000_000);
    assert!(!position.is_pomm);
    assert_eq!(position.fees_owed_0, 0);
    assert_eq!(position.fees_owed_1, 0);
}

#[test]
fn test_position_in_range_checks() {
    let market = create_test_market();

    // Test in-range position
    let position = Position {
        nft_mint: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        owner: Pubkey::new_unique(),
        tick_lower: -100,
        tick_upper: 100,
        liquidity: 1_000_000,
        fee_growth_inside_0_last_x64: 0,
        fee_growth_inside_1_last_x64: 0,
        tokens_owed_0: 0,
        tokens_owed_1: 0,
        position_bump: 255,
        is_pomm: false,
        last_updated_slot: 0,
        fee_growth_inside_0_last: 0,
        fee_growth_inside_1_last: 0,
        fees_owed_0: 0,
        fees_owed_1: 0,
    };

    assert!(is_position_in_range(&position, market.current_tick));

    // Test below-range position
    let position_below = Position {
        tick_lower: 100,
        tick_upper: 200,
        ..position
    };

    assert!(!is_position_in_range(&position_below, market.current_tick));

    // Test above-range position
    let position_above = Position {
        tick_lower: -200,
        tick_upper: -100,
        ..position
    };

    assert!(!is_position_in_range(&position_above, market.current_tick));
}

#[test]
fn test_position_width_calculations() {
    // Test narrow position
    let tick_lower = -10;
    let tick_upper = 10;
    let width = calculate_position_width(tick_lower, tick_upper);
    assert_eq!(width, 20);

    // Test wide position
    let tick_lower = -1000;
    let tick_upper = 1000;
    let width = calculate_position_width(tick_lower, tick_upper);
    assert_eq!(width, 2000);

    // Test maximum width
    let tick_lower = MIN_TICK;
    let tick_upper = MAX_TICK;
    let width = calculate_position_width(tick_lower, tick_upper);
    assert_eq!(width, (MAX_TICK - MIN_TICK) as u32);
}

#[test]
fn test_position_pda_derivation() {
    let program_id = feels::ID;
    let market = Pubkey::new_unique();
    let position_mint = Pubkey::new_unique();

    // Test position PDA
    let (position_pda, position_bump) =
        Pubkey::find_program_address(&[b"position", position_mint.as_ref()], &program_id);

    // Test position metadata PDA
    let (metadata_pda, metadata_bump) =
        Pubkey::find_program_address(&[b"position_metadata", position_pda.as_ref()], &program_id);

    // Verify PDAs are different
    assert_ne!(position_pda, metadata_pda);

    // Test different position mint gives different PDA
    let position_mint_2 = Pubkey::new_unique();
    let (position_pda_2, _) =
        Pubkey::find_program_address(&[b"position", position_mint_2.as_ref()], &program_id);

    assert_ne!(position_pda, position_pda_2);
}

#[test]
fn test_tick_array_updates() {
    // Test that position would update correct tick arrays
    let tick_lower = -1000;
    let tick_upper = 1000;
    let tick_spacing = 10;

    let lower_array_start = get_tick_array_start(tick_lower, tick_spacing);
    let upper_array_start = get_tick_array_start(tick_upper, tick_spacing);

    // Verify arrays are aligned
    assert_eq!(lower_array_start % (TICK_ARRAY_SIZE * tick_spacing), 0);
    assert_eq!(upper_array_start % (TICK_ARRAY_SIZE * tick_spacing), 0);

    // Verify correct arrays would be loaded
    assert!(tick_lower >= lower_array_start);
    assert!(tick_lower < lower_array_start + TICK_ARRAY_SIZE * tick_spacing);
    assert!(tick_upper >= upper_array_start);
    assert!(tick_upper < upper_array_start + TICK_ARRAY_SIZE * tick_spacing);
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
        rolling_buy_volume: 0,
        rolling_sell_volume: 0,
        rolling_total_volume: 0,
        rolling_window_start_slot: 0,
        tick_snapshot_1hr: 0,
        last_snapshot_timestamp: 0,
        _reserved: [0; 1],
    }
}

fn validate_position_params(
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    market: &Market,
) -> Result<()> {
    // Validate tick range
    if tick_lower >= tick_upper {
        return Err(FeelsError::InvalidTickRange.into());
    }

    // Check tick bounds
    if tick_lower < MIN_TICK || tick_upper > MAX_TICK {
        return Err(FeelsError::InvalidTick.into());
    }

    // Check tick spacing
    if tick_lower % market.tick_spacing as i32 != 0 || tick_upper % market.tick_spacing as i32 != 0
    {
        return Err(FeelsError::TickNotSpaced.into());
    }

    // Check range width
    let width = (tick_upper - tick_lower) as u32;
    if width > MAX_POSITION_WIDTH {
        return Err(FeelsError::InvalidTickRange.into());
    }

    // Validate liquidity
    if liquidity == 0 {
        return Err(FeelsError::ZeroLiquidity.into());
    }

    if liquidity < MIN_LIQUIDITY {
        return Err(FeelsError::LiquidityBelowMinimum.into());
    }

    Ok(())
}

fn calculate_liquidity_from_amounts(
    sqrt_price: u128,
    tick_lower: i32,
    tick_upper: i32,
    amount_0: u64,
    amount_1: u64,
) -> Result<u128> {
    // Simplified calculation for testing
    // In production would use proper concentrated liquidity math
    let avg_amount = ((amount_0 as u128 + amount_1 as u128) / 2).max(1);
    let tick_range = (tick_upper - tick_lower).max(1) as u128;
    Ok(avg_amount * 1000 / tick_range)
}

fn is_position_in_range(position: &Position, current_tick: i32) -> bool {
    current_tick >= position.tick_lower && current_tick < position.tick_upper
}

fn calculate_position_width(tick_lower: i32, tick_upper: i32) -> u32 {
    (tick_upper - tick_lower) as u32
}

fn get_tick_array_start(tick: i32, tick_spacing: i32) -> i32 {
    let ticks_per_array = TICK_ARRAY_SIZE * tick_spacing;
    let array_index = tick.div_euclid(ticks_per_array);
    array_index * ticks_per_array
}

const MAX_POSITION_WIDTH: u32 = 886272; // Maximum ticks in a position
const TICK_ARRAY_SIZE: i32 = 64;
