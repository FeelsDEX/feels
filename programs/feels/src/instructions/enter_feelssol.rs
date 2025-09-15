//! Enter FeelsSOL instruction

use crate::{
    constants::{FEELS_HUB_SEED, JITOSOL_VAULT_SEED, MINT_AUTHORITY_SEED},
    error::FeelsError,
    events::FeelsSOLMinted,
    state::FeelsHub,
    utils::{mint_to_with_authority, transfer_from_user_to_vault, validate_amount},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

/// Enter FeelsSOL accounts
#[derive(Accounts)]
pub struct EnterFeelsSOL<'info> {
    /// User entering FeelsSOL
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = user.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub user: Signer<'info>,

    /// User's JitoSOL account
    #[account(
        mut,
        constraint = user_jitosol.owner == user.key() @ FeelsError::InvalidAuthority,
        constraint = user_jitosol.mint == jitosol_mint.key() @ FeelsError::InvalidMint,
    )]
    pub user_jitosol: Account<'info, TokenAccount>,

    /// User's FeelsSOL account
    #[account(
        mut,
        constraint = user_feelssol.owner == user.key() @ FeelsError::InvalidAuthority,
        constraint = user_feelssol.mint == feelssol_mint.key() @ FeelsError::InvalidMint,
    )]
    pub user_feelssol: Account<'info, TokenAccount>,

    /// JitoSOL mint
    pub jitosol_mint: Account<'info, Mint>,

    /// FeelsSOL mint
    #[account(mut)]
    pub feelssol_mint: Account<'info, Mint>,

    /// FeelsHub PDA for reentrancy guard
    #[account(
        mut,
        seeds = [FEELS_HUB_SEED, feelssol_mint.key().as_ref()],
        bump,
        constraint = !hub.reentrancy_guard @ FeelsError::ReentrancyDetected,
    )]
    pub hub: Account<'info, FeelsHub>,

    /// JitoSOL vault (pool-owned by the FeelsSOL hub pool)
    #[account(
        mut,
        seeds = [JITOSOL_VAULT_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub jitosol_vault: Account<'info, TokenAccount>,

    /// Mint authority PDA
    /// CHECK: PDA signer for minting
    #[account(
        seeds = [MINT_AUTHORITY_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

/// Enter FeelsSOL handler
pub fn enter_feelssol(ctx: Context<EnterFeelsSOL>, amount: u64) -> Result<()> {
    // SECURITY: Set guard early
    ctx.accounts.hub.reentrancy_guard = true;
    // Validate amount
    validate_amount(amount)?;

    // Transfer JitoSOL from user to vault
    transfer_from_user_to_vault(
        &ctx.accounts.user_jitosol,
        &ctx.accounts.jitosol_vault,
        &ctx.accounts.user,
        &ctx.accounts.token_program,
        amount,
    )?;

    // Mint FeelsSOL to user (1:1 for MVP)
    let mint_authority_bump = ctx.bumps.mint_authority;
    let mint_key = ctx.accounts.feelssol_mint.key();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[mint_authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    mint_to_with_authority(
        &ctx.accounts.feelssol_mint,
        &ctx.accounts.user_feelssol,
        &ctx.accounts.mint_authority,
        &ctx.accounts.token_program,
        signer_seeds,
        amount,
    )?;

    // Emit event
    emit!(FeelsSOLMinted {
        user: ctx.accounts.user.key(),
        jitosol_amount: amount,
        feelssol_amount: amount,
        timestamp: Clock::get()?.unix_timestamp,
        version: 1,
    });
    // Clear guard
    ctx.accounts.hub.reentrancy_guard = false;
    Ok(())
}
