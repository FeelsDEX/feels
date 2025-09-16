//! Initialize FeelsHub (MVP)

use crate::{
    constants::{FEELS_HUB_SEED, JITOSOL_VAULT_SEED, VAULT_AUTHORITY_SEED},
    state::FeelsHub,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeHub<'info> {
    /// Authority paying for the account
    #[account(mut)]
    pub payer: Signer<'info>,

    /// FeelsSOL mint the hub manages
    /// CHECK: validated in handler constraints
    pub feelssol_mint: AccountInfo<'info>,

    /// JitoSOL mint
    pub jitosol_mint: Account<'info, Mint>,

    /// The FeelsHub PDA
    #[account(
        init,
        payer = payer,
        space = FeelsHub::LEN,
        seeds = [FEELS_HUB_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub hub: Account<'info, FeelsHub>,

    /// JitoSOL vault for the hub
    #[account(
        init,
        payer = payer,
        token::mint = jitosol_mint,
        token::authority = vault_authority,
        seeds = [JITOSOL_VAULT_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub jitosol_vault: Account<'info, TokenAccount>,

    /// Vault authority PDA
    /// CHECK: PDA that controls the JitoSOL vault
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_hub(ctx: Context<InitializeHub>) -> Result<()> {
    let hub = &mut ctx.accounts.hub;
    hub.feelssol_mint = ctx.accounts.feelssol_mint.key();
    hub.reentrancy_guard = false;
    Ok(())
}
