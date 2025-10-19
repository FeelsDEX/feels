use crate::{
    error::FeelsError,
    events::PoolPhaseUpdated,
    state::{Market, PoolPhase, PoolRegistry},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdatePoolPhase<'info> {
    /// Pool registry
    #[account(
        mut,
        seeds = [PoolRegistry::SEED],
        bump = pool_registry.bump,
    )]
    pub pool_registry: Account<'info, PoolRegistry>,

    /// Market whose phase to update
    #[account(
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
    )]
    pub market: Account<'info, Market>,

    /// Authority (must be registry authority or market authority)
    #[account(
        constraint = authority.key() == pool_registry.authority ||
                    authority.key() == market.authority @ FeelsError::InvalidAuthority
    )]
    pub authority: Signer<'info>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn update_pool_phase(ctx: Context<UpdatePoolPhase>, new_phase: PoolPhase) -> Result<()> {
    let registry = &mut ctx.accounts.pool_registry;
    let market = &ctx.accounts.market;
    let clock = &ctx.accounts.clock;

    // Get current phase
    let pool = registry
        .find_pool_by_market(&market.key())
        .ok_or(FeelsError::PoolNotFound)?;
    let old_phase = pool.phase;

    // Update phase
    registry.update_pool_phase(&market.key(), new_phase, clock.unix_timestamp)?;

    // Emit event
    emit!(PoolPhaseUpdated {
        market: market.key(),
        old_phase: old_phase as u8,
        new_phase: new_phase as u8,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
