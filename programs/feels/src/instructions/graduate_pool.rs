//! Graduate pool to steady state (MVP)
//!
//! Idempotently marks the market as steady-state and cleanup complete.

use crate::{error::FeelsError, state::Market};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct GraduatePool<'info> {
    /// Market authority performing graduation
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Market to graduate
    #[account(
        mut,
        constraint = market.authority == authority.key() @ FeelsError::UnauthorizedSigner,
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
    )]
    pub market: Account<'info, Market>,
}

pub fn graduate_pool(ctx: Context<GraduatePool>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    // Idempotent flags
    market.steady_state_seeded = true;
    market.cleanup_complete = true;
    Ok(())
}
