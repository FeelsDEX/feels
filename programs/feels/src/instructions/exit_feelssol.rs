//! Exit FeelsSOL instruction

use crate::{
    constants::{FEELS_HUB_SEED, JITOSOL_VAULT_SEED, VAULT_AUTHORITY_SEED},
    error::FeelsError,
    events::FeelsSOLBurned,
    state::{FeelsHub, ProtocolConfig, ProtocolOracle, SafetyController},
    utils::{burn_from_user, transfer_from_vault_to_user, validate_amount},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

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

    /// FeelsHub PDA for FeelsSOL mint
    /// SECURITY: Provides re-entrancy guard protection
    #[account(
        mut,
        seeds = [FEELS_HUB_SEED, feelssol_mint.key().as_ref()],
        bump,
        constraint = !hub.reentrancy_guard @ FeelsError::ReentrancyDetected
    )]
    pub hub: Account<'info, FeelsHub>,

    /// Safety controller (protocol-level)
    #[account(
        mut,
        seeds = [SafetyController::SEED],
        bump,
    )]
    pub safety: Account<'info, SafetyController>,

    /// Protocol config (for rate limits)
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// Protocol oracle (rates)
    #[account(
        mut,
        seeds = [ProtocolOracle::SEED],
        bump,
    )]
    pub protocol_oracle: Account<'info, ProtocolOracle>,

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
}

/// Exit FeelsSOL handler
pub fn exit_feelssol(ctx: Context<ExitFeelsSOL>, amount: u64) -> Result<()> {
    // SECURITY: Set re-entrancy guard at the very beginning
    ctx.accounts.hub.reentrancy_guard = true;

    // Validate amount
    validate_amount(amount)?;

    // Check if redemptions are allowed (read-only check, no state updates)
    ctx.accounts.safety.check_redemptions_allowed(
        &ctx.accounts.protocol_oracle,
        &ctx.accounts.protocol_config,
        Clock::get()?.unix_timestamp,
    )?;

    // Rate limit: enforce per-slot redemption cap if configured BEFORE burning
    let current_slot = Clock::get()?.slot;
    let safety = &mut ctx.accounts.safety;
    // Reset counter on new slot
    if safety.redeem_last_slot != current_slot {
        safety.redeem_last_slot = current_slot;
        safety.redeem_slot_amount = 0;
    }
    let cap = ctx.accounts.protocol_config.redeem_per_slot_cap_feelssol;
    if cap > 0 {
        let new_used = safety.redeem_slot_amount.saturating_add(amount);
        if new_used > cap {
            emit!(crate::events::RateLimitTriggered {
                scope: 1,
                amount,
                cap,
                slot: current_slot,
                timestamp: Clock::get()?.unix_timestamp,
            });
            return err!(FeelsError::RateLimitExceeded);
        }
        safety.redeem_slot_amount = new_used;
    }

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
    ctx.accounts.hub.reentrancy_guard = false;

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
