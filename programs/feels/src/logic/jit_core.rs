//! JIT v0.5 Complete Implementation - Virtual Concentrated Liquidity
//!
//! Implements the full JIT v0.5 specification with GTWAP anchoring,
//! contrarian placement, and single-transaction execution.
//!
//! Key components:
//! - GTWAP-based price anchoring with floor integration
//! - Contrarian liquidity placement (opposite to taker direction)
//! - Virtual concentrated liquidity (10x multiplier at current price)
//! - Comprehensive safety mechanisms (6 layers of protection)
//! - Single atomic transaction pattern (place-execute-remove)

use crate::error::FeelsError;
use crate::logic::jit_safety::{
    calculate_safe_jit_allowance, update_directional_volume, update_price_snapshot, JitBudget,
};
use crate::state::{Buffer, Market, OracleState};
use crate::utils::validations::{MAX_SQRT_PRICE, MIN_SQRT_PRICE};
use crate::utils::{sqrt_price_from_tick, tick_from_sqrt_price};
use anchor_lang::prelude::*;

/// JIT v0.5 execution context - captures all swap parameters needed for JIT decisions
pub struct JitContext {
    pub current_tick: i32,               // Current pool price in ticks
    pub current_slot: u64,               // Blockchain slot for time-based logic
    pub current_timestamp: i64,          // Unix timestamp for oracle freshness
    pub sqrt_price_limit: u128,          // Swap price limit (reveals taker intent)
    pub amount_specified_is_input: bool, // Whether amount is input or output
    pub is_token_0_to_1: bool,           // Swap direction (token0->token1 or reverse)
    pub swap_amount_quote: u128,         // Swap size in quote units for sizing
}

/// JIT placement result - defines where and how much virtual liquidity to provide
pub struct JitPlacement {
    pub liquidity_amount: u128, // Amount of virtual liquidity in quote units
    pub lower_tick: i32,        // Lower bound of liquidity range
    pub upper_tick: i32,        // Upper bound of liquidity range
    pub is_ask: bool,           // True if selling (ask), false if buying (bid)
    pub anchor_tick: i32,       // GTWAP anchor used for placement
}

/// Constants from spec - these control JIT behavior and safety limits
const DEV_CLAMP_TICKS: i32 = 100; // Max deviation from current price for anchor
const BASE_SPREAD_TICKS: i32 = 1; // Base spread for contrarian placement
                                  // const BASE_SPREAD_TICKS_SYM: i32 = 2;   // Wider spread for symmetric mode (ambiguous) - Reserved for future use
const MAX_SPREAD_TICKS: i32 = 4; // Maximum spread adjustment from toxicity
const RANGE_TICKS: i32 = 1; // Width of JIT liquidity range
const L_MIN_TICKS: i32 = 5; // Min price limit distance for contrarian mode
const Q_MIN_FOR_JIT: u128 = 100_000; // Min swap size (0.1 USDC equivalent)
const MAX_DEV_TICKS: i32 = 500; // Max GTWAP deviation before skipping JIT
                                // const D_MIN_SLOTS: u64 = 3;             // Consecutive slots needed for deviation - Reserved for future use
const COOLDOWN_SLOTS: u64 = 2; // General JIT cooldown after heavy usage
                               // const ASK_COOLDOWN_SLOTS: u64 = 10;     // Extra cooldown for ask placement - Reserved for future use

/// Entry guards for JIT execution - validates all preconditions before JIT activation
/// These guards prevent JIT from operating in unsafe conditions or being exploited
pub fn check_jit_entry_guards(
    ctx: &JitContext,
    market: &Market,
    buffer: &Buffer,
    oracle: &OracleState,
) -> Result<()> {
    // 1. Global safety check - respect market pause state
    // Markets can be paused by governance during emergencies
    require!(!market.is_paused, FeelsError::MarketPaused);

    // 2. Oracle health check - ensure we have valid price data
    // Check if oracle has any observations
    require!(
        oracle.observation_cardinality > 0,
        FeelsError::InvalidOracle
    );

    // 3. Oracle freshness check - prevent using stale prices
    // Get most recent observation timestamp
    let latest_observation = &oracle.observations[oracle.observation_index as usize];
    require!(
        latest_observation.initialized,
        FeelsError::OracleNotInitialized
    );

    let oracle_age = ctx
        .current_timestamp
        .saturating_sub(latest_observation.block_timestamp);
    require!(oracle_age < 300, FeelsError::OracleStale); // 5 min max age

    // 4. Deviation check - skip JIT during extreme price movements
    // Get TWAP tick with minimum 60 second duration
    let gtwap_tick = oracle.get_twap_tick(ctx.current_timestamp, 60)?;
    let deviation = (ctx.current_tick - gtwap_tick).abs();
    require!(
        deviation <= MAX_DEV_TICKS,
        FeelsError::PriceMovementTooLarge
    );

    // 5. Cooldown check - prevent rapid JIT usage after heavy activity
    // Protects against drain attacks by enforcing time gaps
    require!(
        ctx.current_slot >= buffer.jit_last_heavy_usage_slot + COOLDOWN_SLOTS,
        FeelsError::CooldownActive
    );

    // 6. Minimum swap size - prevent dust griefing
    // Small swaps cost more in compute than they generate in fees
    require!(
        ctx.swap_amount_quote >= Q_MIN_FOR_JIT,
        FeelsError::ZeroAmount
    );

    // 7. JIT enabled check - allow markets to disable JIT if needed
    require!(market.jit_enabled, FeelsError::NotImplemented);

    Ok(())
}

/// Calculate GTWAP anchor with clamping and floor integration
/// This is the core pricing mechanism that prevents JIT from being manipulated
///
/// The anchor serves as the reference price around which JIT places liquidity:
/// 1. Uses GTWAP (time-weighted average) to resist short-term manipulation
/// 2. Enforces floor price to maintain protocol solvency
/// 3. Clamps to current price to keep liquidity relevant to actual trading
pub fn calculate_jit_anchor(
    ctx: &JitContext,
    oracle: &OracleState,
    market: &Market,
) -> Result<i32> {
    // Get GTWAP from oracle - this is our manipulation-resistant price
    // Use 60 second TWAP as minimum duration
    let gtwap_tick = if oracle.observation_cardinality > 0 {
        oracle.get_twap_tick(ctx.current_timestamp, 60)?
    } else {
        ctx.current_tick
    };

    // Get floor safe ask tick - ensures JIT never places asks below floor
    // The floor price monotonically increases, providing price support
    let floor_tick = market.floor_tick;

    // Anchor = max(GTWAP, floor) - respects both oracle price and floor constraint
    let anchor = gtwap_tick.max(floor_tick);

    // Clamp to current price - keeps JIT liquidity near actual trading activity
    // Without clamping, JIT could place liquidity far from current price during volatility
    let clamped_anchor = anchor
        .max(ctx.current_tick - DEV_CLAMP_TICKS)
        .min(ctx.current_tick + DEV_CLAMP_TICKS)
        .max(floor_tick);

    Ok(clamped_anchor)
}

/// Determine contrarian direction and placement
/// This is the heart of JIT's market making strategy - always taking the opposite side
///
/// Contrarian placement ensures JIT provides liquidity where it's needed most:
/// - When takers are buying, JIT offers to sell (ask)
/// - When takers are selling, JIT offers to buy (bid)
/// This maximizes fill probability while avoiding adverse selection
pub fn calculate_contrarian_placement(
    ctx: &JitContext,
    anchor_tick: i32,
    spread_adjustment: i32,
    market: &Market,
) -> Result<Option<JitPlacement>> {
    // Infer taker direction from swap params - the price limit reveals intent
    let sqrt_price_limit = ctx.sqrt_price_limit;
    let current_sqrt_price = sqrt_price_from_tick(ctx.current_tick)?;

    // Check if limit is meaningful (not just max/min safety values)
    // Swaps with extreme limits don't reveal true directional intent
    let is_meaningful_limit =
        sqrt_price_limit > MIN_SQRT_PRICE && sqrt_price_limit < MAX_SQRT_PRICE;

    if !is_meaningful_limit {
        return Ok(None); // Ambiguous direction, skip JIT to avoid mispricing
    }

    // Determine taker direction by comparing limit to current price
    // The logic differs based on whether amount is specified as input or output
    let is_buy = if ctx.amount_specified_is_input {
        // Input specified: buying if limit allows price to decrease (0->1) or increase (1->0)
        ctx.is_token_0_to_1 && sqrt_price_limit < current_sqrt_price
            || !ctx.is_token_0_to_1 && sqrt_price_limit > current_sqrt_price
    } else {
        // Output specified: same direction logic applies
        ctx.is_token_0_to_1 && sqrt_price_limit < current_sqrt_price
            || !ctx.is_token_0_to_1 && sqrt_price_limit > current_sqrt_price
    };

    // Check minimum tick distance for contrarian mode
    // Too-close limits suggest market orders or ambiguous intent
    let limit_tick = tick_from_sqrt_price(sqrt_price_limit)?;
    if (limit_tick - ctx.current_tick).abs() < L_MIN_TICKS {
        return Ok(None); // Too close for safe contrarian placement
    }

    // Calculate final spread with toxicity adjustment
    // Wider spreads during toxic flow, tighter during normal conditions
    let final_spread = (BASE_SPREAD_TICKS + spread_adjustment).clamp(0, MAX_SPREAD_TICKS);

    // Add edge offset to prevent tick pinning attacks
    // Alternates between slot & 1 = 0 or 1 to vary placement
    let edge_offset = (ctx.current_slot & 1) as i32;

    // Place contrarian liquidity (opposite of taker direction)
    let mut placement = if is_buy {
        // Taker is buying, JIT places ask liquidity above anchor
        JitPlacement {
            liquidity_amount: 0, // Will be set by budget calculation
            lower_tick: anchor_tick + final_spread + edge_offset,
            upper_tick: anchor_tick + final_spread + edge_offset + RANGE_TICKS,
            is_ask: true,
            anchor_tick,
        }
    } else {
        // Taker is selling, JIT places bid liquidity below anchor
        JitPlacement {
            liquidity_amount: 0, // Will be set by budget calculation
            lower_tick: anchor_tick - final_spread - RANGE_TICKS,
            upper_tick: anchor_tick - final_spread,
            is_ask: false,
            anchor_tick,
        }
    };

    let min_lower_bound = if placement.is_ask {
        market.floor_tick.max(market.global_lower_tick)
    } else {
        market.global_lower_tick
    };
    let max_upper_bound = market.global_upper_tick;

    if !align_range_with_bounds(
        &mut placement,
        ctx.current_tick,
        min_lower_bound,
        max_upper_bound,
    ) {
        return Ok(None);
    }

    Ok(Some(placement))
}

fn align_range_with_bounds(
    placement: &mut JitPlacement,
    current_tick: i32,
    min_lower_bound: i32,
    max_upper_bound: i32,
) -> bool {
    let mut width = placement.upper_tick - placement.lower_tick;
    if width < 0 {
        width = 0;
    }
    let max_width = if max_upper_bound >= min_lower_bound {
        max_upper_bound - min_lower_bound
    } else {
        0
    };
    if width > max_width {
        width = max_width;
    }

    let mut lower = placement.lower_tick;
    let mut upper = lower.saturating_add(width);

    if current_tick < lower {
        let shift = lower - current_tick;
        let available_left = if lower > min_lower_bound {
            lower - min_lower_bound
        } else {
            0
        };
        let allowed_shift = available_left.min(shift);
        lower -= allowed_shift;
        upper = lower.saturating_add(width);
        if current_tick < lower {
            lower = current_tick;
            upper = lower.saturating_add(width);
        }
    } else if current_tick > upper {
        let shift = current_tick - upper;
        let available_right = if max_upper_bound > upper {
            max_upper_bound - upper
        } else {
            0
        };
        let allowed_shift = available_right.min(shift);
        upper += allowed_shift;
        lower = upper.saturating_sub(width);
        if current_tick > upper {
            upper = current_tick;
            lower = upper.saturating_sub(width);
        }
    }

    if lower < min_lower_bound {
        lower = min_lower_bound;
        upper = lower.saturating_add(width);
    }
    if upper > max_upper_bound {
        upper = max_upper_bound;
        lower = upper.saturating_sub(width);
    }

    if current_tick < lower {
        lower = current_tick;
        upper = lower.saturating_add(width);
    } else if current_tick > upper {
        upper = current_tick;
        lower = upper.saturating_sub(width);
    }

    if lower < min_lower_bound {
        lower = min_lower_bound;
    }
    if upper > max_upper_bound {
        upper = max_upper_bound;
    }

    if lower > upper {
        lower = upper;
    }

    placement.lower_tick = lower;
    placement.upper_tick = upper;

    current_tick >= lower && current_tick <= upper
}

/// Execute JIT v0.5 with virtual concentrated liquidity
/// This is the main entry point that orchestrates the entire JIT process
///
/// The function follows a strict sequence to ensure safety:
/// 1. Validates all preconditions through entry guards
/// 2. Calculates price anchor using GTWAP with floor integration
/// 3. Determines contrarian placement based on taker intent
/// 4. Applies all safety mechanisms to size the liquidity
/// 5. Returns placement info for virtual execution in swap
pub fn execute_jit_v05(
    ctx: &JitContext,
    market: &mut Market,
    buffer: &mut Buffer,
    oracle: &OracleState,
) -> Result<Option<JitPlacement>> {
    // 1. Entry guards - fail fast if conditions aren't safe for JIT
    check_jit_entry_guards(ctx, market, buffer, oracle)?;

    // 2. Initialize budget tracker for this slot
    // Tracks per-slot and per-swap limits to prevent drain attacks
    let mut budget = JitBudget::begin(buffer, market, ctx.current_slot);

    // 3. Calculate anchor with clamping - the foundation of our pricing
    // Combines GTWAP (manipulation resistant) with floor (solvency) and clamping (relevance)
    let anchor_tick = calculate_jit_anchor(ctx, oracle, market)?;

    // 4. Get contrarian placement based on inferred taker direction
    // In v0.5 we use fixed spread; future versions will use flow signals
    let spread_adjustment = 0; // Would come from toxicity signals in full version
    let mut placement =
        match calculate_contrarian_placement(ctx, anchor_tick, spread_adjustment, market)? {
            Some(p) => p,
            None => return Ok(None), // Skip JIT for ambiguous/unsafe trades
        };

    // 5. Calculate safe allowance with all mitigations
    // This applies the 5-layer safety system to determine final size
    let is_buy = !placement.is_ask; // JIT direction is contrarian to taker
    let target_tick = if placement.is_ask {
        placement.lower_tick // For asks, we care about lower bound
    } else {
        placement.upper_tick // For bids, we care about upper bound
    };

    let safe_amount = calculate_safe_jit_allowance(
        &mut budget,
        buffer,
        market,
        ctx.current_slot,
        ctx.current_tick,
        target_tick,
        is_buy,
        &Pubkey::default(), // Simplified - would use actual trader for tracking
    )?;

    // 6. Enforce floor constraint for asks - critical solvency check
    // Even with all other checks passed, never place asks below floor
    if placement.is_ask && placement.lower_tick < market.floor_tick {
        placement.lower_tick = market.floor_tick;
        placement.upper_tick = placement.lower_tick + RANGE_TICKS;
        let min_lower_bound = market.floor_tick.max(market.global_lower_tick);
        let max_upper_bound = market.global_upper_tick;
        if !align_range_with_bounds(
            &mut placement,
            ctx.current_tick,
            min_lower_bound,
            max_upper_bound,
        ) {
            return Ok(None);
        }
    }

    // 7. Set final liquidity amount from safety calculations
    placement.liquidity_amount = safe_amount;

    // Final check - skip if amount too small after all reductions
    if placement.liquidity_amount < Q_MIN_FOR_JIT {
        return Ok(None);
    }

    Ok(Some(placement))
}

/// Update state after JIT execution - tracks usage and detects toxic flow
/// This post-trade analysis is crucial for the adaptive safety system
///
/// Updates three key subsystems:
/// 1. Directional volume tracking for crowding detection
/// 2. Price snapshots for circuit breaker monitoring
/// 3. Toxicity detection for future size/spread adjustments
pub fn update_jit_state_after_swap(
    market: &mut Market,
    buffer: &mut Buffer,
    ctx: &JitContext,
    placement: &JitPlacement,
    amount_consumed: u128,
    tick_after_swap: i32,
) -> Result<()> {
    // Update directional volume - tracks buy/sell pressure over rolling windows
    // Used to detect crowded trades and reduce JIT participation
    let is_buy = !placement.is_ask;
    update_directional_volume(market, is_buy, amount_consumed, ctx.current_slot)?;

    // Update price snapshot for circuit breaker - detects extreme movements
    // If price moves >10% in 1 hour, circuit breaker will halt JIT
    update_price_snapshot(market, ctx.current_timestamp)?;

    // Track JIT hit and toxicity - the learning mechanism
    if amount_consumed > 0 {
        // Calculate price movement during swap
        let tick_movement = tick_after_swap - ctx.current_tick;

        // Adverse = price moved against JIT after fill
        // For asks: adverse if price went up (we sold too cheap)
        // For bids: adverse if price went down (we bought too high)
        let adverse =
            (placement.is_ask && tick_movement > 0) || (!placement.is_ask && tick_movement < 0);

        if adverse {
            // Mark heavy usage to trigger cooldown period
            // In full implementation would update toxicity EMA here
            buffer.jit_last_heavy_usage_slot = ctx.current_slot;
        }
    }

    Ok(())
}

/// Virtual liquidity calculation for v0.5 - the key innovation
/// This simulates concentrated liquidity without actual positions
///
/// Instead of managing real liquidity positions, v0.5 provides virtual liquidity
/// that appears during swap execution. The concentration effect (up to 10x)
/// makes JIT highly capital efficient while keeping implementation simple.
pub fn calculate_virtual_liquidity_at_tick(
    base_liquidity: u128,     // JIT's base liquidity amount
    current_tick: i32,        // Current pool price
    target_tick: i32,         // Tick being evaluated
    placement: &JitPlacement, // JIT placement range
    current_slot: u64,        // For concentration shifts
    market: &Market,          // Market parameters
) -> u128 {
    // Check if target tick is within JIT range - no liquidity outside range
    if target_tick < placement.lower_tick || target_tick > placement.upper_tick {
        return 0;
    }

    // Apply concentration multiplier based on distance from current price
    // This creates the concentrated liquidity effect:
    // - 10x multiplier at current price (maximum concentration)
    // - 5x multiplier within 1 width
    // - 2x multiplier within 2 widths
    // - 1x multiplier beyond that
    use crate::logic::jit_safety::calculate_concentration_multiplier;
    let multiplier =
        calculate_concentration_multiplier(current_tick, target_tick, current_slot, market);

    // Return concentrated virtual liquidity
    // This liquidity only exists during this swap execution
    base_liquidity.saturating_mul(multiplier as u128)
}

// Tests for JIT v0.5 are in integration tests where they can properly
// interact with initialized on-chain state accounts
