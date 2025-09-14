//! Exit FeelsSOL instruction

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::{
    constants::{VAULT_AUTHORITY_SEED, JITOSOL_VAULT_SEED, MARKET_SEED},
    error::FeelsError,
    events::FeelsSOLBurned,
    state::Market,
    utils::{validate_amount, transfer_from_vault_to_user, burn_from_user},
};

/// Exit FeelsSOL accounts
#[derive(Accounts)]
pub struct ExitFeelsSOL<'info> {
    /// User exiting FeelsSOL
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
    
    /// Market account for FeelsSOL hub
    /// SECURITY: Provides re-entrancy guard protection
    #[account(
        mut,
        seeds = [MARKET_SEED, feelssol_mint.key().as_ref(), feelssol_mint.key().as_ref()],
        bump,
        constraint = !market.reentrancy_guard @ FeelsError::ReentrancyDetected
    )]
    pub market: Account<'info, Market>,
    
    /// JitoSOL vault (pool-owned by the FeelsSOL hub pool)
    #[account(
        mut,
        seeds = [JITOSOL_VAULT_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub jitosol_vault: Account<'info, TokenAccount>,
    
    /// Vault authority PDA
    /// CHECK: PDA signer for vault operations
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, feelssol_mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: AccountInfo<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Exit FeelsSOL handler
pub fn exit_feelssol(ctx: Context<ExitFeelsSOL>, amount: u64) -> Result<()> {
    // SECURITY: Set re-entrancy guard at the very beginning
    // This prevents re-entrant calls during the burn-transfer sequence
    ctx.accounts.market.reentrancy_guard = true;
    
    // Validate amount
    validate_amount(amount)?;
    
    // Burn FeelsSOL from user
    // CRITICAL: This CPI could potentially be exploited if the token program
    // is malicious or compromised. The re-entrancy guard prevents double-withdrawal.
    burn_from_user(
        &ctx.accounts.feelssol_mint,
        &ctx.accounts.user_feelssol,
        &ctx.accounts.user,
        &ctx.accounts.token_program,
        amount,
    )?;
    
    // Transfer JitoSOL from vault to user (1:1 for MVP)
    let vault_authority_bump = ctx.bumps.vault_authority;
    let mint_key = ctx.accounts.feelssol_mint.key();
    let seeds = &[
        VAULT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[vault_authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    transfer_from_vault_to_user(
        &ctx.accounts.jitosol_vault,
        &ctx.accounts.user_jitosol,
        &ctx.accounts.vault_authority,
        &ctx.accounts.token_program,
        signer_seeds,
        amount,
    )?;
    
    // SECURITY: Clear re-entrancy guard before returning
    // This must happen after all state changes are complete
    ctx.accounts.market.reentrancy_guard = false;
    
    // Emit event
    emit!(FeelsSOLBurned {
        user: ctx.accounts.user.key(),
        feelssol_amount: amount,
        jitosol_amount: amount,
        timestamp: Clock::get()?.unix_timestamp,
        version: 1,
    });
    
    Ok(())
}