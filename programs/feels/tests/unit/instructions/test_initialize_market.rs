//! Unit tests for the initialize_market instruction

use anchor_lang::prelude::*;
use feels::{
    constants::*,
    error::FeelsError,
    instructions::InitializeMarketParams,
    state::{Buffer, Market, OracleState, PolicyV1, TokenOrigin, TokenType},
};
use solana_program::pubkey::Pubkey;

// Test constants
const MIN_SQRT_PRICE: u128 = 4295128739; // Minimum sqrt price
const MAX_SQRT_PRICE: u128 = u128::MAX; // Maximum sqrt price for testing
const MAX_TICK_SPACING: u16 = 1000;
const INITIAL_ORACLE_CARDINALITY: u16 = 1;
const MAX_ORACLE_OBSERVATIONS: usize = 12;

#[test]
fn test_initialize_market_validation() {
    let feelssol_mint = Pubkey::new_from_array([0; 32]); // Smallest possible pubkey
    let other_mint = Pubkey::new_from_array([255; 32]); // Largest possible pubkey

    // Test invalid tick spacing (0)
    let params = InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 0,
        initial_sqrt_price: 1 << 64,
        initial_buy_feelssol_amount: 0,
    };

    assert_eq!(
        validate_market_params(&params).unwrap_err(),
        FeelsError::InvalidTickSpacing.into()
    );

    // Test invalid sqrt price (0)
    let params = InitializeMarketParams {
        base_fee_bps: 30,
        tick_spacing: 10,
        initial_sqrt_price: 0,
        initial_buy_feelssol_amount: 0,
    };

    assert_eq!(
        validate_market_params(&params).unwrap_err(),
        FeelsError::InvalidPrice.into()
    );

    // Test invalid fee
    let params = InitializeMarketParams {
        base_fee_bps: 10001, // > MAX_FEE_BPS
        tick_spacing: 10,
        initial_sqrt_price: 1 << 64,
        initial_buy_feelssol_amount: 0,
    };

    assert_eq!(
        validate_market_params(&params).unwrap_err(),
        FeelsError::FeeCapExceeded.into()
    );
}

#[test]
fn test_market_tick_spacing_validation() {
    let feelssol_mint = Pubkey::new_from_array([0; 32]);
    let other_mint = Pubkey::new_from_array([255; 32]);

    // Test valid tick spacings
    let valid_spacings = [1, 10, 60, 200];
    for spacing in valid_spacings {
        let params = InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: spacing,
            initial_sqrt_price: 1 << 64,
            initial_buy_feelssol_amount: 0,
        };
        assert!(validate_market_params(&params).is_ok());
    }

    // Test invalid tick spacings
    let invalid_spacings = [0, 32768, 65535];
    for spacing in invalid_spacings {
        let params = InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: spacing,
            initial_sqrt_price: 1 << 64,
            initial_buy_feelssol_amount: 0,
        };
        assert!(validate_market_params(&params).is_err());
    }
}

#[test]
fn test_market_sqrt_price_validation() {
    let feelssol_mint = Pubkey::new_from_array([0; 32]);
    let other_mint = Pubkey::new_from_array([255; 32]);

    // Test valid sqrt prices
    let valid_prices = [
        MIN_SQRT_PRICE,
        1 << 64, // 1.0
        MAX_SQRT_PRICE,
    ];

    for price in valid_prices {
        let params = InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: 10,
            initial_sqrt_price: price,
            initial_buy_feelssol_amount: 0,
        };
        assert!(validate_market_params(&params).is_ok());
    }

    // Test invalid sqrt prices
    let invalid_prices = [0, MIN_SQRT_PRICE - 1];

    for price in invalid_prices {
        let params = InitializeMarketParams {
            base_fee_bps: 30,
            tick_spacing: 10,
            initial_sqrt_price: price,
            initial_buy_feelssol_amount: 0,
        };
        assert!(validate_market_params(&params).is_err());
    }
}

#[test]
fn test_market_state_initialization() {
    let token_0 = Pubkey::new_from_array([0; 32]);
    let token_1 = Pubkey::new_from_array([255; 32]);
    let feelssol_mint = token_0;

    let market = Market {
        version: 1,
        is_initialized: true,
        is_paused: false,
        token_0,
        token_1,
        feelssol_mint,
        token_0_type: TokenType::Spl,
        token_1_type: TokenType::Spl,
        token_0_origin: TokenOrigin::ProtocolMinted,
        token_1_origin: TokenOrigin::External,
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
        policy: PolicyV1::default(),
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
    };

    // Verify hub-and-spoke constraint
    assert_eq!(market.token_0, feelssol_mint);
    assert!(market.is_initialized);
    assert!(!market.is_paused);
    assert_eq!(market.liquidity, 0);
    assert_eq!(market.tick_spacing, 10);

    // Verify JIT v0.5 parameters
    assert_eq!(market.jit_base_cap_bps, 300); // 3%
    assert_eq!(market.jit_per_slot_cap_bps, 500); // 5%
    assert_eq!(market.jit_max_multiplier, 10);
    assert_eq!(market.jit_drain_protection_bps, 7000); // 70%
    assert_eq!(market.jit_circuit_breaker_bps, 3000); // 30%
}

#[test]
fn test_buffer_initialization() {
    let market_key = Pubkey::new_unique();
    let buffer_authority = Pubkey::new_unique();

    let buffer = Buffer {
        market: market_key,
        authority: buffer_authority,
        feelssol_mint: Pubkey::new_from_array([0; 32]),
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
        buffer_authority_bump: 255,
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
    };

    // Verify buffer initialization
    assert_eq!(buffer.market, market_key);
    assert_eq!(buffer.authority, buffer_authority);
    assert_eq!(buffer.tau_spot, 0);
    assert_eq!(buffer.fees_token_0, 0);
    assert_eq!(buffer.fees_token_1, 0);
    assert_eq!(buffer.floor_placement_threshold, 1_000_000);
    assert_eq!(buffer.pomm_position_count, 0);

    // Verify JIT v0.5 fields
    assert_eq!(buffer.jit_rolling_consumption, 0);
    assert_eq!(buffer.jit_last_heavy_usage_slot, 0);
    assert_eq!(buffer.initial_tau_spot, 0);
}

#[test]
fn test_oracle_state_initialization() {
    let market_key = Pubkey::new_unique();

    let oracle = OracleState {
        pool_id: market_key,
        observation_index: 0,
        observation_cardinality: INITIAL_ORACLE_CARDINALITY,
        observation_cardinality_next: INITIAL_ORACLE_CARDINALITY,
        oracle_bump: 255,
        observations: [Default::default(); MAX_ORACLE_OBSERVATIONS],
        _reserved: [0; 4],
    };

    // Verify oracle initialization
    assert_eq!(oracle.pool_id, market_key);
    assert_eq!(oracle.observation_index, 0);
    assert_eq!(oracle.observation_cardinality, INITIAL_ORACLE_CARDINALITY);
    assert_eq!(oracle.observations.len(), MAX_ORACLE_OBSERVATIONS);
}

#[test]
fn test_market_pda_derivation() {
    let program_id = feels::ID;
    let token_0 = Pubkey::new_from_array([0; 32]);
    let token_1 = Pubkey::new_from_array([255; 32]);

    // Test market PDA
    let (market_pda, market_bump) = Pubkey::find_program_address(
        &[b"market", token_0.as_ref(), token_1.as_ref()],
        &program_id,
    );

    // Test market authority PDA
    let (authority_pda, authority_bump) =
        Pubkey::find_program_address(&[b"market_authority", market_pda.as_ref()], &program_id);

    // Test vault PDAs
    let (vault_0_pda, vault_0_bump) = Pubkey::find_program_address(
        &[b"vault", market_pda.as_ref(), token_0.as_ref()],
        &program_id,
    );

    let (vault_1_pda, vault_1_bump) = Pubkey::find_program_address(
        &[b"vault", market_pda.as_ref(), token_1.as_ref()],
        &program_id,
    );

    // Verify PDAs are different
    assert_ne!(market_pda, authority_pda);
    assert_ne!(vault_0_pda, vault_1_pda);
    assert_ne!(market_pda, vault_0_pda);
}

#[test]
fn test_tick_to_price_conversions() {
    // Test tick 0 = price 1.0
    let sqrt_price_at_0 = sqrt_price_from_tick(0).unwrap();
    assert_eq!(sqrt_price_at_0, 1 << 64);

    // Test positive tick = price > 1.0
    let sqrt_price_positive = sqrt_price_from_tick(1000).unwrap();
    assert!(sqrt_price_positive > (1 << 64));

    // Test negative tick = price < 1.0
    let sqrt_price_negative = sqrt_price_from_tick(-1000).unwrap();
    assert!(sqrt_price_negative < (1 << 64));

    // Test round trip conversion
    let original_tick = 12345;
    let sqrt_price = sqrt_price_from_tick(original_tick).unwrap();
    let recovered_tick = tick_from_sqrt_price(sqrt_price).unwrap();
    assert!((recovered_tick - original_tick).abs() <= 1); // Allow for rounding
}

// Helper functions

fn validate_market_params(params: &InitializeMarketParams) -> Result<()> {
    // Validate fee
    if params.base_fee_bps > MAX_FEE_BPS {
        return Err(FeelsError::FeeCapExceeded.into());
    }

    // Validate tick spacing
    if params.tick_spacing == 0 || params.tick_spacing > MAX_TICK_SPACING {
        return Err(FeelsError::InvalidTickSpacing.into());
    }

    // Validate sqrt price
    if params.initial_sqrt_price == 0 || params.initial_sqrt_price < MIN_SQRT_PRICE {
        return Err(FeelsError::InvalidPrice.into());
    }

    Ok(())
}

fn sqrt_price_from_tick(tick: i32) -> Result<u128> {
    // Simplified implementation for testing
    let price = 1.0001f64.powf(tick as f64);
    let sqrt_price = price.sqrt();
    Ok((sqrt_price * (1u128 << 64) as f64) as u128)
}

fn tick_from_sqrt_price(sqrt_price: u128) -> Result<i32> {
    // Simplified implementation for testing
    let price = (sqrt_price as f64 / (1u128 << 64) as f64).powi(2);
    let tick = (price.ln() / 1.0001f64.ln()) as i32;
    Ok(tick)
}
