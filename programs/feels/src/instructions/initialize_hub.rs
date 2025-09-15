//! Initialize FeelsHub (MVP)

use anchor_lang::prelude::*;
use crate::{
    constants::FEELS_HUB_SEED,
    state::FeelsHub,
};

#[derive(Accounts)]
pub struct InitializeHub<'info> {
    /// Authority paying for the account
    #[account(mut)]
    pub payer: Signer<'info>,

    /// FeelsSOL mint the hub manages
    /// CHECK: validated in handler constraints
    pub feelssol_mint: AccountInfo<'info>,

    /// The FeelsHub PDA
    #[account(
        init,
        payer = payer,
        space = FeelsHub::LEN,
        seeds = [FEELS_HUB_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub hub: Account<'info, FeelsHub>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_hub(ctx: Context<InitializeHub>) -> Result<()> {
    let hub = &mut ctx.accounts.hub;
    hub.feelssol_mint = ctx.accounts.feelssol_mint.key();
    hub.reentrancy_guard = false;
    Ok(())
}

