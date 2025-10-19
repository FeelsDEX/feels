//! Update position fee accrual for lower tick
//!
//! This instruction updates fee accrual for the lower tick only.
//! Combined with update_position_fee_upper, it allows fee collection
//! for positions spanning tick arrays too far apart for one transaction.

use crate::{
    constants::POSITION_SEED,
    error::FeelsError,
    state::{Market, Position, TickArray},
};
use anchor_lang::prelude::*;

/// Update position fee lower accounts
#[derive(Accounts)]
pub struct UpdatePositionFeeLower<'info> {
    /// Position owner
    #[account(
        constraint = owner.key() == position.owner @ FeelsError::InvalidAuthority
    )]
    pub owner: Signer<'info>,

    /// Market
    #[account(
        constraint = market.is_initialized,
        constraint = !market.is_paused,
    )]
    pub market: Box<Account<'info, Market>>,

    /// Position
    #[account(
        mut,
        seeds = [POSITION_SEED, position.nft_mint.as_ref()],
        bump,
        constraint = position.market == market.key() @ FeelsError::InvalidMarket,
    )]
    pub position: Box<Account<'info, Position>>,

    /// Tick array containing the lower tick
    #[account(
        constraint = lower_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray
    )]
    pub lower_tick_array: AccountLoader<'info, TickArray>,
}

/// Update position fee lower handler
pub fn update_position_fee_lower(ctx: Context<UpdatePositionFeeLower>) -> Result<()> {
    let market = &ctx.accounts.market;
    let position = &mut ctx.accounts.position;

    // Load and validate tick array
    let lower_array = ctx.accounts.lower_tick_array.load()?;
    crate::utils::validate_tick_array_for_tick(
        &lower_array,
        position.tick_lower,
        market.tick_spacing,
    )?;
    let lower_tick = lower_array.get_tick(position.tick_lower, market.tick_spacing)?;

    // Calculate fee growth inside for lower tick
    let fee_growth_below_0 = if market.current_tick >= position.tick_lower {
        lower_tick.fee_growth_outside_0_x64
    } else {
        market
            .fee_growth_global_0_x64
            .wrapping_sub(lower_tick.fee_growth_outside_0_x64)
    };

    let fee_growth_below_1 = if market.current_tick >= position.tick_lower {
        lower_tick.fee_growth_outside_1_x64
    } else {
        market
            .fee_growth_global_1_x64
            .wrapping_sub(lower_tick.fee_growth_outside_1_x64)
    };

    // Store the lower tick contribution in position's reserved space
    // We pack the data as follows:
    // _reserved[0..2]: Update flag (1 = lower updated, 2 = both updated)
    // _reserved[2..3]: Padding
    // _reserved[3..4]: Market current tick snapshot (truncated to u8)
    // _reserved[4..8]: Reserved for future use

    // Mark lower tick as updated by setting the slot
    let current_slot = Clock::get()?.slot;
    position.last_updated_slot = current_slot;

    // Calculate partial fee accrual for the lower tick only
    // This will be combined with upper tick in the second instruction
    let partial_fee_0 = calculate_partial_fees(
        position.liquidity,
        position.fee_growth_inside_0_last_x64,
        fee_growth_below_0,
        market.fee_growth_global_0_x64,
        market.current_tick >= position.tick_lower,
    );

    let partial_fee_1 = calculate_partial_fees(
        position.liquidity,
        position.fee_growth_inside_1_last_x64,
        fee_growth_below_1,
        market.fee_growth_global_1_x64,
        market.current_tick >= position.tick_lower,
    );

    // Accumulate partial fees
    position.tokens_owed_0 = position.tokens_owed_0.saturating_add(partial_fee_0);
    position.tokens_owed_1 = position.tokens_owed_1.saturating_add(partial_fee_1);

    Ok(())
}

/// Calculate partial fees for a single tick boundary
fn calculate_partial_fees(
    liquidity: u128,
    last_fee_growth: u128,
    fee_growth_outside: u128,
    fee_growth_global: u128,
    is_initialized: bool,
) -> u64 {
    if liquidity == 0 || !is_initialized {
        return 0;
    }

    // For wide positions, we use a simplified calculation
    // that avoids requiring both ticks in the same transaction
    let fee_growth_delta = fee_growth_global
        .wrapping_sub(fee_growth_outside)
        .wrapping_sub(last_fee_growth);

    // Calculate fees with proper scaling
    ((liquidity.saturating_mul(fee_growth_delta)) >> 64) as u64
}
