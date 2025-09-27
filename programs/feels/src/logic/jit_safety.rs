//! JIT v0.5 Safety Functions - Core protection mechanisms
//!
//! This module implements the 5-layer safety system that prevents JIT exploitation:
//! 1. Graduated drain protection - throttles based on consumption
//! 2. Slot-based concentration shifts - prevents tick camping
//! 3. Directional caps - reduces participation in crowded trades
//! 4. Impact penalties - discourages large price movements
//! 5. Circuit breakers - emergency halt mechanism
//!
//! These functions are used by jit_v05.rs to size liquidity safely

use crate::error::FeelsError;
use crate::state::{Buffer, Market};
use anchor_lang::prelude::*;

/// JIT v0.5 budget tracker - enforces per-slot and per-swap limits
/// This prevents attackers from draining the buffer in a single slot
#[derive(Clone, Copy, Default)]
pub struct JitBudget {
    pub slot: u64,              // Current blockchain slot
    pub base_cap_q: u128,       // Max per-swap amount (e.g., 3% of buffer)
    pub per_slot_cap_q: u128,   // Max per-slot amount (e.g., 5% of buffer)
    pub slot_remaining_q: u128, // Remaining budget for this slot
}

impl JitBudget {
    /// Initialize JIT budget for current slot
    /// Resets tracking when entering a new slot and calculates caps
    pub fn begin(buffer: &mut Buffer, market: &Market, current_slot: u64) -> Self {
        // Reset per-slot tracking on new slot
        // This ensures each slot starts with a fresh budget
        if buffer.jit_last_slot != current_slot {
            buffer.jit_last_slot = current_slot;
            buffer.jit_slot_used_q = 0;
        }

        // Calculate caps based on market parameters and current buffer balance
        let base_q = buffer.tau_spot; // Current buffer balance (tau)

        // Per-swap cap: e.g., 300 bps = 3% of buffer
        let base_cap_q = base_q
            .saturating_mul(market.jit_base_cap_bps as u128)
            .saturating_div(10_000);

        // Per-slot cap: e.g., 500 bps = 5% of buffer
        let per_slot_cap_q = base_q
            .saturating_mul(market.jit_per_slot_cap_bps as u128)
            .saturating_div(10_000);

        // Calculate remaining budget for this slot
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
/// This is the core safety function that applies all 6 protection layers
///
/// The function progressively reduces JIT allowance based on risk factors:
/// 1. Circuit breaker can completely halt JIT
/// 2. Directional caps reduce participation in crowded trades
/// 3. Concentration multiplier enhances capital efficiency
/// 4. Drain protection throttles based on recent usage
/// 5. Impact penalty discourages large price movements
/// 6. Floor guard prevents asks below protocol floor price
pub fn calculate_safe_jit_allowance(
    budget: &mut JitBudget, // Budget tracker (mutable for updates)
    buffer: &mut Buffer,    // Buffer state (mutable for tracking)
    market: &Market,        // Market parameters
    current_slot: u64,      // Current blockchain slot
    current_tick: i32,      // Current price tick
    target_tick: i32,       // Target placement tick
    is_buy: bool,           // JIT direction (buy/bid vs sell/ask)
    _trader: &Pubkey,       // Trader address (for future tracking)
) -> Result<u128> {
    // 1. Circuit breaker check - immediate halt if triggered
    // Protects against extreme conditions (buffer depletion, price crashes)
    if is_circuit_breaker_active(buffer, market) {
        return Ok(0); // No JIT allowed during emergency conditions
    }

    // 2. Base calculation with directional adjustment
    // Reduces caps by 50% when trade direction is crowded
    let base_cap = get_directional_cap(is_buy, market, market.jit_base_cap_bps);
    let base_amount = buffer
        .tau_spot
        .checked_mul(base_cap as u128)
        .ok_or(FeelsError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(FeelsError::MathOverflow)?;

    // 3. Apply concentration with slot-based shifts
    // Multiplies base amount by up to 10x based on tick distance
    // Shifts prevent attackers from camping optimal ticks
    let multiplier =
        calculate_concentration_multiplier(current_tick, target_tick, current_slot, market);
    let concentrated_amount = base_amount
        .checked_mul(multiplier as u128)
        .ok_or(FeelsError::MathOverflow)?;

    // 4. Apply graduated drain protection
    // Throttles allowance as rolling consumption increases
    let safe_amount =
        apply_drain_protection(budget, buffer, concentrated_amount, current_slot, market)?;

    // 4a. Enforce per-slot remaining cap explicitly
    // Never exceed the remaining slot budget
    let capped_amount = safe_amount.min(budget.slot_remaining_q);

    // 5. Apply tick distance penalty
    // Reduces allowance for trades that move price significantly
    let penalty_adjusted_amount = apply_impact_penalty(capped_amount, current_tick, target_tick);

    // 6. Floor guard for ask liquidity (critical solvency check)
    // If this is ask liquidity and target is below floor, reject entirely
    // This ensures JIT never undermines protocol solvency by selling below floor
    let final_amount = if !is_buy && target_tick < market.floor_tick {
        0 // No ask liquidity allowed below floor price
    } else {
        penalty_adjusted_amount
    };

    // Update budget tracking for next iteration
    // These updates persist in buffer state across swaps
    buffer.jit_slot_used_q = buffer.jit_slot_used_q.saturating_add(final_amount);
    budget.slot_remaining_q = budget.slot_remaining_q.saturating_sub(final_amount);

    Ok(final_amount)
}

/// Calculate concentration multiplier with slot-based shifts
/// This creates the virtual concentrated liquidity effect
///
/// Two key mechanisms:
/// 1. Concentration: Higher multipliers near current price (capital efficiency)
/// 2. Slot shifts: Moves concentration zone to prevent tick camping attacks
pub fn calculate_concentration_multiplier(
    current_tick: i32, // Current pool price
    target_tick: i32,  // Where JIT liquidity is being evaluated
    current_slot: u64, // For time-based shifts
    market: &Market,   // Contains concentration parameters
) -> u8 {
    // Shift concentration zone every 100 slots (~40 seconds)
    // This prevents attackers from predicting and camping optimal ticks
    let shift_interval = 100u64;
    let shift_cycles = current_slot.saturating_div(shift_interval);

    // Create a shift between -10 and +9 ticks that changes over time
    // shift_cycles % 20 gives 0-19, subtract 10 for -10 to +9 range
    let shift_amount = ((shift_cycles % 20) as i32).saturating_sub(10);

    // Apply shift to the center point, not the distance
    // This moves the entire concentration zone around the current price
    let shifted_center = current_tick.saturating_add(shift_amount);
    let adjusted_distance = target_tick.saturating_sub(shifted_center).abs() as u32;

    // Apply concentration multiplier based on distance from shifted center
    // Default width = 10 ticks, max multiplier = 10x
    let max_multiplier = market.jit_max_multiplier.max(1);
    match adjusted_distance {
        d if d <= market.jit_concentration_width => max_multiplier, // 10x at center
        d if d <= market.jit_concentration_width * 2 => (max_multiplier / 2).max(1), // 5x at 1-2 widths
        d if d <= market.jit_concentration_width * 4 => (max_multiplier / 5).max(1), // 2x at 2-4 widths
        _ => 1,                                                                      // 1x beyond
    }
}

/// Apply graduated drain protection
/// Prevents rapid buffer depletion by throttling based on recent consumption
///
/// The protection works on a rolling window basis:
/// - Tracks consumption over ~1 minute windows
/// - Progressively reduces allowance as consumption increases
/// - Resets every window to allow recovery
fn apply_drain_protection(
    budget: &mut JitBudget, // Budget tracker
    buffer: &mut Buffer,    // Buffer state for consumption tracking
    requested_amount: u128, // Amount JIT wants to provide
    current_slot: u64,      // Current slot for window management
    market: &Market,        // Market parameters
) -> Result<u128> {
    // Reset rolling window every 150 slots (~1 minute)
    // This allows JIT to recover from temporary throttling
    if current_slot > buffer.jit_rolling_window_start + 150 {
        buffer.jit_rolling_consumption = 0;
        buffer.jit_rolling_window_start = current_slot;
    }

    // Calculate consumption ratio as basis points
    // Shows how much of the per-slot cap has been consumed in this window
    let consumption_ratio = if budget.per_slot_cap_q > 0 {
        buffer
            .jit_rolling_consumption
            .saturating_mul(10_000)
            .saturating_div(budget.per_slot_cap_q)
    } else {
        0
    };

    // Graduated throttling based on drain protection threshold
    // Default threshold = 7000 bps (70% of cap triggers throttling)
    let drain_threshold = market.jit_drain_protection_bps as u128;
    let throttle_factor =
        if consumption_ratio < drain_threshold.saturating_mul(50).saturating_div(100) {
            100 // < 50% of threshold (35% consumed): full allowance
        } else if consumption_ratio < drain_threshold.saturating_mul(75).saturating_div(100) {
            50 // 50-75% of threshold (35-52.5% consumed): half allowance
        } else if consumption_ratio < drain_threshold.saturating_mul(90).saturating_div(100) {
            20 // 75-90% of threshold (52.5-63% consumed): 20% allowance
        } else {
            10 // > 90% of threshold (>63% consumed): 10% allowance
        };

    // Apply throttle factor to requested amount
    let allowed = requested_amount
        .saturating_mul(throttle_factor)
        .saturating_div(100);

    // Update rolling consumption for next iteration
    // Note: We track the allowed amount, not requested amount
    buffer.jit_rolling_consumption = buffer.jit_rolling_consumption.saturating_add(allowed);

    Ok(allowed)
}

/// Get directional cap based on recent flow
/// Reduces JIT participation when one trade direction dominates
///
/// This prevents JIT from amplifying directional momentum and
/// reduces exposure during potential manipulation or cascading liquidations
fn get_directional_cap(is_buy: bool, market: &Market, base_cap_bps: u16) -> u16 {
    // Calculate recent buy pressure as percentage of total volume
    let buy_pressure = if market.rolling_total_volume > 0 {
        market
            .rolling_buy_volume
            .saturating_mul(100)
            .saturating_div(market.rolling_total_volume)
            .min(100) as u16
    } else {
        50 // Default to balanced (50/50) if no volume history
    };

    // Reduce cap by 50% for crowded trade directions
    // This makes JIT more conservative during one-sided markets
    match (is_buy, buy_pressure) {
        (true, bp) if bp > 70 => base_cap_bps / 2, // Heavy buy pressure (>70% buys)
        (false, bp) if bp < 30 => base_cap_bps / 2, // Heavy sell pressure (<30% buys = >70% sells)
        _ => base_cap_bps,                         // Normal balanced conditions
    }
}

/// Apply penalty based on price impact
/// Discourages JIT from participating in trades that move price significantly
///
/// Large price movements often indicate:
/// - Thin liquidity (higher risk of adverse selection)
/// - Potential manipulation attempts
/// - Cascading liquidations or stop hunts
fn apply_impact_penalty(base_allowance: u128, start_tick: i32, end_tick: i32) -> u128 {
    let tick_movement = (end_tick - start_tick).abs();

    // Graduated penalty for large price movements
    // Each tick represents ~0.01% price change
    let penalty_factor = match tick_movement {
        0..=10 => 100,   // No penalty for small moves (<0.1%)
        11..=50 => 70,   // 30% penalty for medium moves (0.1-0.5%)
        51..=100 => 40,  // 60% penalty for large moves (0.5-1%)
        101..=200 => 20, // 80% penalty for very large moves (1-2%)
        _ => 10,         // 90% penalty for extreme moves (>2%)
    };

    // Apply penalty to reduce JIT participation
    base_allowance
        .saturating_mul(penalty_factor as u128)
        .saturating_div(100)
}

/// Check if circuit breaker should activate
/// Emergency halt mechanism for extreme market conditions
///
/// Circuit breaker triggers on:
/// 1. Buffer depletion - protects protocol solvency
/// 2. Extreme price movements - indicates potential attacks or liquidation cascades
fn is_circuit_breaker_active(buffer: &Buffer, market: &Market) -> bool {
    // Check buffer health as percentage of initial
    // Buffer depletion threatens JIT's ability to provide liquidity
    let buffer_health_bps = if buffer.initial_tau_spot > 0 {
        buffer
            .tau_spot
            .saturating_mul(10_000)
            .saturating_div(buffer.initial_tau_spot)
            .min(10_000) as u16
    } else {
        10_000 // If initial not set, assume healthy (new pool)
    };

    // Trigger if buffer drops below threshold (default: 30% = 3000 bps)
    if buffer_health_bps < market.jit_circuit_breaker_bps {
        return true;
    }

    // Check for extreme price movement (>10% in 1 hour)
    // Large movements suggest manipulation or mass liquidations
    let price_movement = market
        .current_tick
        .saturating_sub(market.tick_snapshot_1hr)
        .abs();

    // 1000 ticks ≈ 10% price movement (each tick ≈ 0.01%)
    price_movement > 1000
}

/// Update directional volume tracking
/// Maintains rolling window of buy/sell volume for directional cap calculations
///
/// This tracking enables JIT to detect when one trade direction dominates
/// and reduce participation accordingly (crowding detection)
pub fn update_directional_volume(
    market: &mut Market, // Market state to update
    is_buy: bool,        // Trade direction (true = buy, false = sell)
    volume: u128,        // Volume in quote units
    current_slot: u64,   // Current slot for window management
) -> Result<()> {
    // Reset rolling window every 1000 slots (~400 seconds / ~6.7 minutes)
    // This provides a reasonable timeframe for detecting directional bias
    if current_slot > market.rolling_window_start_slot + 1000 {
        market.rolling_buy_volume = 0;
        market.rolling_sell_volume = 0;
        market.rolling_total_volume = 0;
        market.rolling_window_start_slot = current_slot;
    }

    // Update directional volumes
    // Separate tracking allows calculation of buy/sell pressure ratio
    if is_buy {
        market.rolling_buy_volume = market.rolling_buy_volume.saturating_add(volume);
    } else {
        market.rolling_sell_volume = market.rolling_sell_volume.saturating_add(volume);
    }

    // Update total for ratio calculations
    market.rolling_total_volume = market.rolling_total_volume.saturating_add(volume);

    Ok(())
}

/// Update price snapshot for circuit breaker
/// Takes periodic price snapshots to detect extreme movements
///
/// The circuit breaker uses these snapshots to identify potential
/// manipulation or liquidation cascades (>10% movement in 1 hour)
pub fn update_price_snapshot(market: &mut Market, current_timestamp: i64) -> Result<()> {
    // Update hourly snapshot if more than 3600 seconds (1 hour) have passed
    // This provides the baseline for detecting extreme price movements
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

    #[test]
    fn test_floor_guard_enforcement() {
        let mut buffer = create_test_buffer();
        let mut market = create_test_market();
        let mut budget = JitBudget::begin(&mut buffer, &market, 100);

        // Set floor tick to 100
        market.floor_tick = 100;

        // Test 1: Ask liquidity above floor should be allowed
        let ask_above_floor = calculate_safe_jit_allowance(
            &mut budget,
            &mut buffer,
            &market,
            100,   // current_slot
            50,    // current_tick (below floor)
            150,   // target_tick (above floor)
            false, // is_buy = false means this is ask liquidity
            &Pubkey::default(),
        )
        .unwrap();

        // Should get non-zero amount since ask is above floor
        assert!(
            ask_above_floor > 0,
            "Ask liquidity above floor should be allowed"
        );

        // Reset budget for next test
        budget = JitBudget::begin(&mut buffer, &market, 101);

        // Test 2: Ask liquidity below floor should be rejected
        let ask_below_floor = calculate_safe_jit_allowance(
            &mut budget,
            &mut buffer,
            &market,
            101,   // current_slot
            50,    // current_tick
            80,    // target_tick (below floor of 100)
            false, // is_buy = false means this is ask liquidity
            &Pubkey::default(),
        )
        .unwrap();

        // Should get zero amount since ask is below floor
        assert_eq!(
            ask_below_floor, 0,
            "Ask liquidity below floor should be rejected"
        );

        // Reset budget for next test
        budget = JitBudget::begin(&mut buffer, &market, 102);

        // Test 3: Buy liquidity below floor should still be allowed
        let buy_below_floor = calculate_safe_jit_allowance(
            &mut budget,
            &mut buffer,
            &market,
            102,  // current_slot
            50,   // current_tick
            80,   // target_tick (below floor)
            true, // is_buy = true means this is bid liquidity
            &Pubkey::default(),
        )
        .unwrap();

        // Should get non-zero amount since buy liquidity is allowed below floor
        assert!(
            buy_below_floor > 0,
            "Buy liquidity below floor should be allowed"
        );
    }
}
