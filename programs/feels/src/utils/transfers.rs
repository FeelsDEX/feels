//! Token transfer utilities
//!
//! Helper functions for common token transfer patterns

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

/// Transfer tokens from a user account to a vault
pub fn transfer_from_user_to_vault<'info>(
    user_token: &Account<'info, TokenAccount>,
    vault: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: user_token.to_account_info(),
        to: vault.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)
}

/// Transfer tokens from a vault to a user account using PDA authority
pub fn transfer_from_vault_to_user<'info>(
    vault: &Account<'info, TokenAccount>,
    user_token: &Account<'info, TokenAccount>,
    vault_authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    authority_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: vault.to_account_info(),
        to: user_token.to_account_info(),
        authority: vault_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        authority_seeds,
    );
    token::transfer(cpi_ctx, amount)
}

/// Transfer tokens from buffer vault to market vault using PDA authority
pub fn transfer_from_buffer_vault<'info>(
    buffer_vault: &AccountInfo<'info>,
    market_vault: &AccountInfo<'info>,
    buffer_authority: &AccountInfo<'info>,
    authority_seeds: &[&[u8]],
    amount: u64,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: buffer_vault.to_account_info(),
        to: market_vault.to_account_info(),
        authority: buffer_authority.to_account_info(),
    };
    let seeds = [authority_seeds];
    let cpi_ctx =
        CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, &seeds);
    token::transfer(cpi_ctx, amount)
}

/// Transfer between two accounts with PDA authority
pub fn transfer_with_authority<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    authority_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        authority_seeds,
    );
    token::transfer(cpi_ctx, amount)
}

/// Mint tokens using PDA authority
pub fn mint_to_with_authority<'info>(
    mint: &Account<'info, Mint>,
    to: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    authority_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: mint.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        authority_seeds,
    );
    token::mint_to(cpi_ctx, amount)
}

/// Burn tokens with user authority
pub fn burn_from_user<'info>(
    mint: &Account<'info, Mint>,
    from: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Burn {
        mint: mint.to_account_info(),
        from: from.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::burn(cpi_ctx, amount)
}

/// Transfer tokens from a user account to a vault (with AccountInfo)
pub fn transfer_from_user_to_vault_unchecked<'info>(
    user_token: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: user_token.clone(),
        to: vault.clone(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)
}

/// Transfer tokens from a vault to a user account using PDA authority (with AccountInfo)
pub fn transfer_from_vault_to_user_unchecked<'info>(
    vault: &AccountInfo<'info>,
    user_token: &AccountInfo<'info>,
    vault_authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    authority_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: vault.clone(),
        to: user_token.clone(),
        authority: vault_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        authority_seeds,
    );
    token::transfer(cpi_ctx, amount)
}
