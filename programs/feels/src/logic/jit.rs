//! JIT v0.5 Implementation - Concentrated Virtual Liquidity with Enhanced Caps
//!
//! Provides virtual concentrated liquidity with safety mitigations

use crate::error::FeelsError;
use crate::state::{Buffer, Market};
use anchor_lang::prelude::*;

/// JIT v0.5 budget tracker with enhanced safety features
#[derive(Clone, Copy, Default)]
pub struct JitBudget {
    pub slot: u64,
    pub base_cap_q: u128,
    pub per_slot_cap_q: u128,
    pub slot_remaining_q: u128,
}

impl JitBudget {
    /// Initialize JIT budget for current slot
    pub fn begin(buffer: &mut Buffer, market: &Market, current_slot: u64) -> Self {
        // Reset per-slot tracking on new slot
        if buffer.jit_last_slot != current_slot {
            buffer.jit_last_slot = current_slot;
            buffer.jit_slot_used_q = 0;
        }

        // Calculate caps based on market parameters
        let base_q = buffer.tau_spot;
        let base_cap_q = base_q
            .saturating_mul(market.jit_base_cap_bps as u128)
            .saturating_div(10_000);
        let per_slot_cap_q = base_q
            .saturating_mul(market.jit_per_slot_cap_bps as u128)
            .saturating_div(10_000);
        let slot_remaining_q = per_slot_cap_q.saturating_sub(buffer.jit_slot_used_q);

        Self {
            slot: current_slot,
            base_cap_q,
            per_slot_cap_q,
            slot_remaining_q,
        }
    }
}

/// Calculate safe JIT allowance with all v0.5 mitigations
pub fn calculate_safe_jit_allowance(
    budget: &mut JitBudget,
    buffer: &mut Buffer,
    market: &Market,
    current_slot: u64,
    current_tick: i32,
    target_tick: i32,
    is_buy: bool,
    _trader: &Pubkey,
) -> Result<u128> {
    // 1. Circuit breaker check
    if is_circuit_breaker_active(buffer, market) {
        return Ok(0);
    }

    // 2. Base calculation with directional adjustment
    let base_cap = get_directional_cap(is_buy, market, market.jit_base_cap_bps);
    let base_amount = buffer
        .tau_spot
        .checked_mul(base_cap as u128)
        .ok_or(FeelsError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(FeelsError::MathOverflow)?;

    // 3. Apply concentration with slot-based shifts
    let multiplier =
        calculate_concentration_multiplier(current_tick, target_tick, current_slot, market);
    let concentrated_amount = base_amount
        .checked_mul(multiplier as u128)
        .ok_or(FeelsError::MathOverflow)?;

    // 4. Apply graduated drain protection
    let safe_amount =
        apply_drain_protection(budget, buffer, concentrated_amount, current_slot, market)?;

    // 4a. Enforce per-slot remaining cap explicitly
    let capped_amount = safe_amount.min(budget.slot_remaining_q);

    // 5. Apply tick distance penalty
    let final_amount = apply_impact_penalty(capped_amount, current_tick, target_tick);

    // Update budget tracking
    buffer.jit_slot_used_q = buffer.jit_slot_used_q.saturating_add(final_amount);
    budget.slot_remaining_q = budget.slot_remaining_q.saturating_sub(final_amount);

    Ok(final_amount)
}

/// Calculate concentration multiplier with slot-based shifts
fn calculate_concentration_multiplier(
    current_tick: i32,
    target_tick: i32,
    current_slot: u64,
    market: &Market,
) -> u8 {
    // Shift concentration zone every 100 slots (~40 seconds)
    let shift_interval = 100u64;
    let shift_cycles = current_slot.saturating_div(shift_interval);
    let shift_amount = ((shift_cycles % 20) as i32).saturating_sub(10);

    // Calculate base distance first
    let _base_distance = target_tick.saturating_sub(current_tick);

    // Apply shift to the center point, not the distance
    let shifted_center = current_tick.saturating_add(shift_amount);
    let adjusted_distance = target_tick.saturating_sub(shifted_center).abs() as u32;

    // Apply concentration based on distance
    match adjusted_distance {
        d if d <= market.jit_concentration_width => market.jit_max_multiplier,
        d if d <= market.jit_concentration_width * 2 => market.jit_max_multiplier / 2,
        d if d <= market.jit_concentration_width * 4 => market.jit_max_multiplier / 5,
        _ => 1,
    }
}

/// Apply graduated drain protection
fn apply_drain_protection(
    budget: &mut JitBudget,
    buffer: &mut Buffer,
    requested_amount: u128,
    current_slot: u64,
    market: &Market,
) -> Result<u128> {
    // Reset rolling window every 150 slots (~1 minute)
    if current_slot > buffer.jit_rolling_window_start + 150 {
        buffer.jit_rolling_consumption = 0;
        buffer.jit_rolling_window_start = current_slot;
    }

    // Calculate consumption ratio
    let consumption_ratio = if budget.per_slot_cap_q > 0 {
        buffer
            .jit_rolling_consumption
            .saturating_mul(10_000)
            .saturating_div(budget.per_slot_cap_q)
    } else {
        0
    };

    // Graduated throttling based on drain protection threshold
    let drain_threshold = market.jit_drain_protection_bps as u128;
    let throttle_factor =
        if consumption_ratio < drain_threshold.saturating_mul(50).saturating_div(100) {
            100 // < 50% of threshold: full allowance
        } else if consumption_ratio < drain_threshold.saturating_mul(75).saturating_div(100) {
            50 // 50-75% of threshold: half allowance
        } else if consumption_ratio < drain_threshold.saturating_mul(90).saturating_div(100) {
            20 // 75-90% of threshold: 20% allowance
        } else {
            10 // > 90% of threshold: 10% allowance
        };

    let allowed = requested_amount
        .saturating_mul(throttle_factor)
        .saturating_div(100);

    buffer.jit_rolling_consumption = buffer.jit_rolling_consumption.saturating_add(allowed);

    Ok(allowed)
}

/// Get directional cap based on recent flow
fn get_directional_cap(is_buy: bool, market: &Market, base_cap_bps: u16) -> u16 {
    // Calculate recent buy pressure
    let buy_pressure = if market.rolling_total_volume > 0 {
        market
            .rolling_buy_volume
            .saturating_mul(100)
            .saturating_div(market.rolling_total_volume)
            .min(100) as u16
    } else {
        50 // Default to balanced
    };

    // Reduce cap for crowded direction
    match (is_buy, buy_pressure) {
        (true, bp) if bp > 70 => base_cap_bps / 2, // Heavy buy pressure
        (false, bp) if bp < 30 => base_cap_bps / 2, // Heavy sell pressure
        _ => base_cap_bps,                         // Normal conditions
    }
}

/// Apply penalty based on price impact
fn apply_impact_penalty(base_allowance: u128, start_tick: i32, end_tick: i32) -> u128 {
    let tick_movement = (end_tick - start_tick).abs();

    // Graduated penalty for large price movements
    let penalty_factor = match tick_movement {
        0..=10 => 100,   // No penalty for small moves
        11..=50 => 70,   // 30% penalty
        51..=100 => 40,  // 60% penalty
        101..=200 => 20, // 80% penalty
        _ => 10,         // 90% penalty for huge moves
    };

    base_allowance
        .saturating_mul(penalty_factor as u128)
        .saturating_div(100)
}

/// Check if circuit breaker should activate
fn is_circuit_breaker_active(buffer: &Buffer, market: &Market) -> bool {
    // Check buffer health
    let buffer_health_bps = if buffer.initial_tau_spot > 0 {
        buffer
            .tau_spot
            .saturating_mul(10_000)
            .saturating_div(buffer.initial_tau_spot)
            .min(10_000) as u16
    } else {
        10_000 // If initial not set, assume healthy
    };

    if buffer_health_bps < market.jit_circuit_breaker_bps {
        return true;
    }

    // Check for extreme price movement (>10% in 1 hour)
    let price_movement = market
        .current_tick
        .saturating_sub(market.tick_snapshot_1hr)
        .abs();

    price_movement > 1000 // ~10% movement triggers circuit breaker
}

/// Update directional volume tracking
pub fn update_directional_volume(
    market: &mut Market,
    is_buy: bool,
    volume: u128,
    current_slot: u64,
) -> Result<()> {
    // Reset rolling window every 1000 slots (~400 seconds)
    if current_slot > market.rolling_window_start_slot + 1000 {
        market.rolling_buy_volume = 0;
        market.rolling_sell_volume = 0;
        market.rolling_total_volume = 0;
        market.rolling_window_start_slot = current_slot;
    }

    // Update volumes
    if is_buy {
        market.rolling_buy_volume = market.rolling_buy_volume.saturating_add(volume);
    } else {
        market.rolling_sell_volume = market.rolling_sell_volume.saturating_add(volume);
    }

    market.rolling_total_volume = market.rolling_total_volume.saturating_add(volume);

    Ok(())
}

/// Update price snapshot for circuit breaker
pub fn update_price_snapshot(market: &mut Market, current_timestamp: i64) -> Result<()> {
    // Update hourly snapshot
    if current_timestamp > market.last_snapshot_timestamp + 3600 {
        market.tick_snapshot_1hr = market.current_tick;
        market.last_snapshot_timestamp = current_timestamp;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{PolicyV1, TokenOrigin, TokenType};
    use anchor_lang::prelude::Pubkey;

    fn create_test_buffer() -> Buffer {
        Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: 1_000_000,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 0,
            floor_placement_threshold: 0,
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
            initial_tau_spot: 1_000_000,
            protocol_owned_override: 0,
            pomm_position_count: 0,
            _padding: [0; 7],
        }
    }

    fn create_test_market() -> Market {
        Market {
            version: 1,
            is_initialized: true,
            is_paused: false,
            token_0: Pubkey::default(),
            token_1: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            token_0_type: TokenType::Spl,
            token_1_type: TokenType::Spl,
            token_0_origin: TokenOrigin::External,
            token_1_origin: TokenOrigin::External,
            vault_0: Pubkey::default(),
            vault_1: Pubkey::default(),
            hub_protocol: None,
            sqrt_price: 0,
            liquidity: 0,
            current_tick: 0,
            tick_spacing: 1,
            global_lower_tick: -443636,
            global_upper_tick: 443636,
            floor_liquidity: 0,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
            fee_growth_global_0: 0,
            fee_growth_global_1: 0,
            base_fee_bps: 30,
            buffer: Pubkey::default(),
            authority: Pubkey::default(),
            last_epoch_update: 0,
            epoch_number: 0,
            oracle: Pubkey::default(),
            oracle_bump: 0,
            policy: PolicyV1::default(),
            market_authority_bump: 0,
            vault_0_bump: 0,
            vault_1_bump: 0,
            reentrancy_guard: false,
            initial_liquidity_deployed: false,
            jit_enabled: true,
            jit_base_cap_bps: 300,     // 3%
            jit_per_slot_cap_bps: 500, // 5%
            jit_concentration_width: 10,
            jit_max_multiplier: 10,
            jit_drain_protection_bps: 7000, // 70%
            jit_circuit_breaker_bps: 3000,  // 30%
            floor_tick: 0,
            floor_buffer_ticks: 0,
            last_floor_ratchet_ts: 0,
            floor_cooldown_secs: 0,
            steady_state_seeded: false,
            cleanup_complete: false,
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

    #[test]
    fn test_concentration_multiplier() {
        let market = create_test_market();

        // Use slot 1000 which gives shift_amount = 0 (10 % 20 - 10 = 0)
        let slot = 1000;

        // Test at current tick
        let multiplier = calculate_concentration_multiplier(0, 0, slot, &market);
        assert_eq!(multiplier, 10); // Max multiplier

        // Test within concentration width
        let multiplier = calculate_concentration_multiplier(0, 5, slot, &market);
        assert_eq!(multiplier, 10);

        // Test at 2x width
        let multiplier = calculate_concentration_multiplier(0, 15, slot, &market);
        assert_eq!(multiplier, 5);

        // Test far away
        let multiplier = calculate_concentration_multiplier(0, 100, slot, &market);
        assert_eq!(multiplier, 1);

        // Test with slot-based shift
        let slot_with_shift = 100; // shift_cycles = 1, shift_amount = 1 % 20 - 10 = -9
        let multiplier = calculate_concentration_multiplier(0, 0, slot_with_shift, &market);
        assert_eq!(multiplier, 10); // Distance is |0 - (0 + -9)| = 9, within width 10

        // Test shift moves the concentration zone
        let multiplier = calculate_concentration_multiplier(0, -9, slot_with_shift, &market);
        assert_eq!(multiplier, 10); // Now at the shifted center
    }

    #[test]
    fn test_directional_caps() {
        let mut market = create_test_market();

        // Balanced market
        market.rolling_buy_volume = 50;
        market.rolling_total_volume = 100;
        let cap = get_directional_cap(true, &market, 300);
        assert_eq!(cap, 300); // Full cap

        // Heavy buy pressure
        market.rolling_buy_volume = 80;
        market.rolling_total_volume = 100;
        let cap = get_directional_cap(true, &market, 300);
        assert_eq!(cap, 150); // Half cap for crowded direction

        // Heavy sell pressure
        market.rolling_buy_volume = 20;
        market.rolling_total_volume = 100;
        let cap = get_directional_cap(false, &market, 300);
        assert_eq!(cap, 150); // Half cap for crowded direction
    }

    #[test]
    fn test_circuit_breaker() {
        let mut buffer = create_test_buffer();
        let market = create_test_market();

        // Healthy buffer
        assert!(!is_circuit_breaker_active(&buffer, &market));

        // Depleted buffer
        buffer.tau_spot = 200_000; // 20% of initial
        assert!(is_circuit_breaker_active(&buffer, &market));

        // Price movement check
        let mut market_moved = create_test_market();
        market_moved.current_tick = 1500;
        market_moved.tick_snapshot_1hr = 0;
        assert!(is_circuit_breaker_active(&buffer, &market_moved));
    }
}
