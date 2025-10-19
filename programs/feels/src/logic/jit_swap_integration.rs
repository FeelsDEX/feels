//! JIT v0.5 Swap Integration
//!
//! Integrates JIT liquidity into the swap execution flow with
//! single-transaction place-execute-remove pattern.
//!
//! This module bridges the JIT v0.5 system with the main swap engine:
//! - Calculates effective liquidity by adding virtual JIT liquidity to base
//! - Executes swaps with JIT enhancement when conditions are favorable
//! - Tracks JIT consumption and updates state after execution
//! - Implements the atomic place-execute-remove pattern within swap

use crate::logic::engine::{compute_swap_step, StepResult, SwapContext, SwapDirection};
use crate::logic::jit_core::{
    calculate_virtual_liquidity_at_tick, execute_jit_v05, update_jit_state_after_swap, JitContext,
    JitPlacement,
};
use crate::state::{Buffer, Market, OracleState};
use crate::utils::{sqrt_price_from_tick, tick_from_sqrt_price};
use anchor_lang::prelude::*;
use ethnum::U256;

/// JIT-enhanced swap step computation result
/// Wraps the standard swap outcome with JIT-specific tracking
pub struct JitSwapStep {
    pub base_step: StepResult,               // Standard swap calculation result
    pub jit_consumed: u128,                  // Amount of JIT liquidity used
    pub jit_placement: Option<JitPlacement>, // Placement details if JIT activated
}

/// Calculate JIT-enhanced liquidity at a given price
/// This is where the "virtual" in virtual concentrated liquidity happens
///
/// The function combines:
/// - Base liquidity from actual LPs in the pool
/// - Virtual JIT liquidity that exists only for this swap
///   The result is higher effective liquidity without managing real positions
pub fn get_effective_liquidity_with_jit(
    base_liquidity: u128,                 // Existing pool liquidity from LPs
    current_tick: i32,                    // Current pool price
    target_tick: i32,                     // Price point being evaluated
    jit_placement: Option<&JitPlacement>, // JIT placement if active
    current_slot: u64,                    // For concentration shifts
    market: &Market,                      // Market parameters
) -> u128 {
    let mut total_liquidity = base_liquidity;

    if let Some(placement) = jit_placement {
        // Add virtual concentrated liquidity at this tick
        // The concentration multiplier (up to 10x) makes JIT capital efficient
        let jit_liquidity = calculate_virtual_liquidity_at_tick(
            placement.liquidity_amount,
            current_tick,
            target_tick,
            placement,
            current_slot,
            market,
        );
        total_liquidity = total_liquidity.saturating_add(jit_liquidity);
    }

    total_liquidity
}

/// Execute swap with JIT liquidity integration
/// This is the main integration point implementing the place-execute-remove pattern
///
/// The function orchestrates the entire JIT lifecycle within a single swap:
/// 1. Evaluates if JIT should activate (via execute_jit_v05)
/// 2. Places virtual liquidity if conditions are met
/// 3. Executes the swap with enhanced liquidity
/// 4. Removes the virtual liquidity (automatic - no cleanup needed)
/// 5. Updates state based on execution results
#[allow(clippy::too_many_arguments)]
pub fn execute_swap_with_jit(
    ctx: &SwapContext,               // Current swap execution context
    market: &mut Market,             // Market state (mutable for updates)
    buffer: &mut Buffer,             // Buffer state (mutable for fee tracking)
    oracle: &OracleState,            // Oracle for GTWAP pricing
    amount_in: u64,                  // Swap input amount
    sqrt_price_limit: u128,          // Price limit (reveals taker intent)
    amount_specified_is_input: bool, // Whether amount is input or output
    current_slot: u64,               // Current blockchain slot
    current_timestamp: i64,          // Current timestamp
    current_tick: i32,               // Current market tick
    target_tick: Option<i32>,        // Target tick for step
) -> Result<(StepResult, Option<JitPlacement>, u128)> {
    // Build JIT context from swap parameters
    // This captures all information needed for JIT decisions
    let is_zero_for_one = ctx.direction == SwapDirection::ZeroForOne;
    let needs_base_to_quote = (is_zero_for_one && amount_specified_is_input)
        || (!is_zero_for_one && !amount_specified_is_input);
    let swap_amount_quote = if amount_in == 0 {
        0
    } else if needs_base_to_quote {
        let sqrt_start = if ctx.sqrt_price > 0 {
            Some(ctx.sqrt_price)
        } else {
            sqrt_price_from_tick(current_tick).ok()
        };
        let sqrt_limit = if sqrt_price_limit > 0 {
            Some(sqrt_price_limit)
        } else {
            None
        };

        let relevant_sqrt = match (sqrt_start, sqrt_limit) {
            (Some(start), Some(limit)) => {
                if is_zero_for_one {
                    start.min(limit)
                } else {
                    start.max(limit)
                }
            }
            (Some(start), None) => start,
            (None, Some(limit)) => limit,
            (None, None) => 0,
        };

        if relevant_sqrt == 0 {
            0
        } else {
            let sqrt = U256::from(relevant_sqrt);
            let price_q128 = sqrt * sqrt;
            let amount = U256::from(amount_in);
            let product: U256 = amount * price_q128;
            let shifted: U256 = product >> 128;
            shifted.min(U256::from(u128::MAX)).as_u128()
        }
    } else {
        amount_in as u128
    };

    let jit_ctx = JitContext {
        current_tick,
        current_slot,
        current_timestamp,
        sqrt_price_limit,
        amount_specified_is_input,
        is_token_0_to_1: is_zero_for_one,
        swap_amount_quote,
    };

    // Quick guard before running the heavier JIT pipeline
    if !should_attempt_jit(market, jit_ctx.swap_amount_quote, current_slot, buffer) {
        let base_step = compute_swap_step(ctx, sqrt_price_limit, target_tick, amount_in)?;
        return Ok((base_step, None, 0));
    }

    // Try to place JIT liquidity - this runs all safety checks and calculations
    // Returns None if conditions aren't favorable or safety checks fail
    let jit_placement = execute_jit_v05(&jit_ctx, market, buffer, oracle)?;

    // Calculate base step without JIT first - establishes baseline
    // This helps us measure JIT's contribution to the swap
    let base_step = compute_swap_step(ctx, sqrt_price_limit, target_tick, amount_in)?;

    // If no JIT placement, return base step unchanged
    let (final_step, jit_consumed) = if let Some(ref placement) = jit_placement {
        // JIT is active - recalculate with enhanced liquidity

        // Calculate effective liquidity including virtual JIT liquidity
        let effective_liquidity = get_effective_liquidity_with_jit(
            ctx.liquidity,
            current_tick,
            current_tick, // Target is current for immediate execution
            Some(placement),
            current_slot,
            market,
        );

        // Create enhanced context with JIT-boosted liquidity
        let jit_ctx = SwapContext {
            liquidity: effective_liquidity,
            direction: ctx.direction,
            sqrt_price: ctx.sqrt_price,
            fee_bps: ctx.fee_bps,
            global_lower_tick: ctx.global_lower_tick,
            global_upper_tick: ctx.global_upper_tick,
            tick_spacing: ctx.tick_spacing,
        };

        // Compute enhanced step with virtual liquidity included
        // This simulates having more liquidity at the current price
        let jit_step = compute_swap_step(&jit_ctx, sqrt_price_limit, target_tick, amount_in)?;

        // Calculate how much JIT was consumed by comparing outcomes
        // Input-specified swaps benefit via additional output; output-specified via reduced input
        let jit_consumed = if amount_specified_is_input {
            if jit_step.out > base_step.out {
                jit_step.out.saturating_sub(base_step.out) as u128
            } else {
                0
            }
        } else if base_step.net_in_used > jit_step.net_in_used {
            base_step.net_in_used.saturating_sub(jit_step.net_in_used) as u128
        } else {
            0
        };

        (jit_step, jit_consumed)
    } else {
        // No JIT placement - use base calculations
        (base_step, 0)
    };

    // Update JIT state if any liquidity was consumed
    // This tracks usage, updates toxicity, and manages cooldowns
    if jit_consumed > 0 {
        if let Some(ref placement) = jit_placement {
            let tick_after = tick_from_sqrt_price(final_step.sqrt_next)?;
            update_jit_state_after_swap(
                market,
                buffer,
                &jit_ctx,
                placement,
                jit_consumed,
                tick_after,
            )?;
        }
    }

    Ok((final_step, jit_placement, jit_consumed))
}

/// Check if JIT should be attempted for this swap
/// Quick pre-flight check to avoid expensive JIT calculations for unsuitable swaps
///
/// This function performs basic checks before invoking the full JIT system:
/// - Market has JIT enabled (can be disabled per-market)
/// - Swap size meets minimum threshold (prevents dust griefing)
/// - Not in cooldown period (prevents rapid drain attacks)
/// - Buffer has funds available (no point trying if empty)
pub fn should_attempt_jit(
    market: &Market,       // Market configuration
    amount_in_quote: u128, // Swap size in quote units
    current_slot: u64,     // Current blockchain slot
    buffer: &Buffer,       // Buffer state for cooldown check
) -> bool {
    // All conditions must be true for JIT to be worth attempting
    market.jit_enabled &&                                    // JIT allowed on this market
    amount_in_quote >= 100_000 &&                           // MIN_FOR_JIT threshold
    current_slot >= buffer.jit_last_heavy_usage_slot + 2 && // Not in cooldown
    buffer.tau_spot > 0 // Buffer has funds
}

#[cfg(test)]
mod tests {
    // Tests moved to integration tests due to Market struct requirements
}
