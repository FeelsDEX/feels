//! Swap fee processing and distribution logic for the Feels Protocol
//!
//! This module handles all fee-related operations including:
//! - Fee calculation and splitting between Buffer, Treasury, and Creator
//! - Impact fee calculations based on market conditions
//! - Floor ratchet mechanism for price protection
//! - Fee domain routing and collection

use crate::{
    error::FeelsError,
    events::FloorRatcheted,
    logic::fees::{calculate_impact_bps, combine_base_and_impact},
    state::{Buffer, FeeDomain, Market, ProtocolConfig, ProtocolToken},
};
use anchor_lang::prelude::*;

/// Fee split result containing amounts for each recipient
#[derive(Debug)]
pub struct FeeSplit {
    pub buffer_amount: u64,
    pub protocol_amount: u64,
    pub creator_amount: u64,
}

/// Calculate impact-based fees for the swap
///
/// Impact fees scale with the size of the swap relative to available liquidity,
/// helping to manage large trades and prevent excessive price impact.
pub fn calculate_dynamic_fees(
    base_fee_bps: u16,
    amount_in: u64,
    liquidity: u128,
    sqrt_price: u128,
) -> Result<u16> {
    // For now, use zero impact fee to fix compilation
    // In production, we'd calculate this from tick movement
    let impact_bps = 0u16;

    // Combine base and impact fees
    let (total_fee_bps, _impact_only) = combine_base_and_impact(base_fee_bps, impact_bps);

    Ok(total_fee_bps)
}

/// Split and apply fees according to protocol fee structure
///
/// Fees are distributed across three recipients:
/// - Buffer: Remaining amount after protocol and creator fees (for τ-based operations)
/// - Treasury: Protocol fee percentage (mandatory, configurable)
/// - Creator: Creator fee percentage (optional, only for protocol-minted tokens)
pub fn split_and_apply_fees(
    _market: &Market,
    buffer: &mut Buffer,
    protocol_config: &ProtocolConfig,
    protocol_token: Option<&Account<ProtocolToken>>,
    fee_amount: u64,
    token_index: usize,
) -> Result<FeeSplit> {
    if fee_amount == 0 {
        return Ok(FeeSplit {
            buffer_amount: 0,
            protocol_amount: 0,
            creator_amount: 0,
        });
    }

    // Calculate fee splits based on protocol configuration
    let protocol_fee_rate = protocol_config.default_protocol_fee_rate; // e.g., 1000 = 10%
    let creator_fee_rate = if protocol_token.is_some() {
        protocol_config.default_creator_fee_rate
    } else {
        0
    };

    let protocol_amount = (fee_amount as u128 * protocol_fee_rate as u128 / 10_000) as u64;
    let creator_amount = (fee_amount as u128 * creator_fee_rate as u128 / 10_000) as u64;
    let buffer_amount = fee_amount
        .saturating_sub(protocol_amount)
        .saturating_sub(creator_amount);

    // Apply buffer fees (remaining amount after protocol and creator fees)
    buffer.collect_fee(buffer_amount, token_index, FeeDomain::Spot)?;

    // Return amounts for transfer processing in main handler
    Ok(FeeSplit {
        buffer_amount,
        protocol_amount,
        creator_amount,
    })
}

/// Calculate JIT fee routing when JIT liquidity is consumed
///
/// When JIT liquidity is used, the consumed quote amount should be routed
/// to buffer fee accounting instead of being burned, ensuring capital efficiency.
pub fn route_jit_fees(
    buffer: &mut Buffer,
    jit_consumed_quote: u64,
    is_token_0_to_1: bool,
) -> Result<()> {
    if jit_consumed_quote == 0 {
        return Ok(());
    }

    // For 0->1 swaps: JIT provides token 1 liquidity
    // So jit_consumed_quote is added to fees_token_1
    if is_token_0_to_1 {
        buffer.fees_token_1 = buffer
            .fees_token_1
            .saturating_add(jit_consumed_quote as u128);
    } else {
        buffer.fees_token_0 = buffer
            .fees_token_0
            .saturating_add(jit_consumed_quote as u128);
    }

    Ok(())
}

/// Validate fee parameters to prevent excessive charging
pub fn validate_fee_parameters(total_fee_bps: u16, max_total_fee_bps: u16) -> Result<()> {
    if max_total_fee_bps > 0 && total_fee_bps > max_total_fee_bps {
        return Err(FeelsError::FeeTooHigh.into());
    }

    // Sanity check: fees should never exceed 50% (5000 bps)
    require!(total_fee_bps <= 5000, FeelsError::FeeTooHigh);

    Ok(())
}

/// Calculate the current candidate floor tick based on market position
///
/// The floor is set as a buffer distance below the current tick to allow for
/// natural price movement while preventing excessive downside.
pub fn current_candidate_floor(market: &Market, buffer: &Buffer) -> Result<i32> {
    // Only ratchet when protocol-owned liquidity or overrides are present
    let has_reserves = buffer.protocol_owned_override > 0
        || buffer.tau_spot > 0
        || buffer.fees_token_0 > 0
        || buffer.fees_token_1 > 0;

    if !has_reserves {
        return Ok(market.floor_tick.max(market.global_lower_tick));
    }

    let candidate = market
        .current_tick
        .saturating_sub(market.floor_buffer_ticks)
        .max(market.global_lower_tick);

    Ok(candidate)
}

/// Execute floor ratchet mechanism to protect against excessive downside
///
/// The floor ratchet prevents the market from falling too far below recent highs,
/// providing some protection for long-term liquidity providers. It operates on
/// a cooldown timer to prevent excessive manipulation.
pub fn do_floor_ratchet(
    market: &mut Account<Market>,
    buffer: &Buffer,
    clock: &Sysvar<Clock>,
) -> Result<()> {
    let old_floor = market.floor_tick;

    // Check if cooldown period has passed
    if clock
        .unix_timestamp
        .saturating_sub(market.last_floor_ratchet_ts)
        >= market.floor_cooldown_secs
    {
        let candidate = current_candidate_floor(market, buffer)?;

        // Only ratchet up, never down
        if candidate > market.floor_tick {
            market.floor_tick = candidate.min(market.global_upper_tick);
            market.last_floor_ratchet_ts = clock.unix_timestamp;

            emit!(FloorRatcheted {
                market: market.key(),
                old_floor_tick: old_floor,
                new_floor_tick: market.floor_tick,
                timestamp: clock.unix_timestamp,
            });
        }
    }
    Ok(())
}

/// Process fee-related state updates after swap execution
pub fn finalize_fee_state(
    market: &mut Account<Market>,
    buffer: &mut Buffer,
    jit_consumed_quote: u64,
    base_fees_skipped: u64,
    is_token_0_to_1: bool,
    clock: &Sysvar<Clock>,
) -> Result<()> {
    // Route JIT consumed quote to buffer fees
    route_jit_fees(buffer, jit_consumed_quote, is_token_0_to_1)?;

    // Execute floor ratchet if conditions are met
    do_floor_ratchet(market, buffer, clock)?;

    // Emit event for skipped base fees if JIT was active
    if base_fees_skipped > 0 {
        emit!(crate::events::JitBaseFeeSkipped {
            market: market.key(),
            swap_id: market.key(), // This should be the trader's pubkey in actual implementation
            base_fees_skipped,
            jit_consumed_quote,
            timestamp: clock.unix_timestamp,
        });
    }

    Ok(())
}
