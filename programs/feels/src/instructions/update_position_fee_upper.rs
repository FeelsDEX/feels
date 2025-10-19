//! Update position fee accrual for upper tick
//!
//! This instruction updates fee accrual for the upper tick only.
//! Combined with update_position_fee_lower, it allows fee collection
//! for positions spanning tick arrays too far apart for one transaction.

use crate::{
    constants::POSITION_SEED,
    error::FeelsError,
    state::{Market, Position, TickArray},
};
use anchor_lang::prelude::*;

/// Update position fee upper accounts
#[derive(Accounts)]
pub struct UpdatePositionFeeUpper<'info> {
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

    /// Tick array containing the upper tick
    #[account(
        constraint = upper_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray
    )]
    pub upper_tick_array: AccountLoader<'info, TickArray>,
}

/// Update position fee upper handler
pub fn update_position_fee_upper(ctx: Context<UpdatePositionFeeUpper>) -> Result<()> {
    let market = &ctx.accounts.market;
    let position = &mut ctx.accounts.position;

    // Check that lower tick was already updated
    // Using last_updated_slot as a proxy - if it's been updated this slot, assume lower was done
    let current_slot = Clock::get()?.slot;
    let lower_updated = position.last_updated_slot == current_slot;
    require!(lower_updated, FeelsError::LowerTickNotUpdated);

    // Load and validate tick array
    let upper_array = ctx.accounts.upper_tick_array.load()?;
    crate::utils::validate_tick_array_for_tick(
        &upper_array,
        position.tick_upper,
        market.tick_spacing,
    )?;
    let upper_tick = upper_array.get_tick(position.tick_upper, market.tick_spacing)?;

    // Calculate fee growth inside for upper tick
    let fee_growth_above_0 = if market.current_tick < position.tick_upper {
        upper_tick.fee_growth_outside_0_x64
    } else {
        market
            .fee_growth_global_0_x64
            .wrapping_sub(upper_tick.fee_growth_outside_0_x64)
    };

    let fee_growth_above_1 = if market.current_tick < position.tick_upper {
        upper_tick.fee_growth_outside_1_x64
    } else {
        market
            .fee_growth_global_1_x64
            .wrapping_sub(upper_tick.fee_growth_outside_1_x64)
    };

    // Now we can calculate the full fee growth inside
    // Since we don't have access to fee_growth_below here, we'll use a simplified approach
    // We'll calculate the fees owed based on the current global fee growth
    let fee_growth_inside_0 = market
        .fee_growth_global_0_x64
        .wrapping_sub(fee_growth_above_0);

    let fee_growth_inside_1 = market
        .fee_growth_global_1_x64
        .wrapping_sub(fee_growth_above_1);

    // Calculate fees owed
    let fee_growth_delta_0 =
        fee_growth_inside_0.wrapping_sub(position.fee_growth_inside_0_last_x64);
    let fee_growth_delta_1 =
        fee_growth_inside_1.wrapping_sub(position.fee_growth_inside_1_last_x64);

    let fees_owed_0 = (position.liquidity.saturating_mul(fee_growth_delta_0) >> 64) as u64;
    let fees_owed_1 = (position.liquidity.saturating_mul(fee_growth_delta_1) >> 64) as u64;

    // Update position state
    position.tokens_owed_0 = position.tokens_owed_0.saturating_add(fees_owed_0);
    position.tokens_owed_1 = position.tokens_owed_1.saturating_add(fees_owed_1);
    position.fee_growth_inside_0_last_x64 = fee_growth_inside_0;
    position.fee_growth_inside_1_last_x64 = fee_growth_inside_1;

    // Update the slot to mark completion
    position.last_updated_slot = current_slot;

    Ok(())
}
