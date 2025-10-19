//! Destroy expired token instruction
//!
//! Allows anyone to destroy an expired token that hasn't had liquidity deployed

use crate::{
    constants::{ESCROW_AUTHORITY_SEED, ESCROW_SEED, MARKET_SEED, PROTOCOL_TOKEN_SEED},
    error::FeelsError,
    events::TokenDestroyed,
    state::{Market, PreLaunchEscrow, ProtocolConfig, ProtocolToken},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

/// Destroy expired token accounts
#[derive(Accounts)]
pub struct DestroyExpiredToken<'info> {
    /// Anyone can call this instruction to destroy expired tokens
    #[account(mut)]
    pub destroyer: Signer<'info>,

    /// Token mint to destroy
    /// CHECK: We verify this is expired through protocol_token
    pub token_mint: AccountInfo<'info>,

    /// Protocol token registry entry
    #[account(
        mut,
        seeds = [PROTOCOL_TOKEN_SEED, token_mint.key().as_ref()],
        bump,
        constraint = protocol_token.mint == token_mint.key() @ FeelsError::InvalidMint,
        close = destroyer, // Return rent to destroyer
    )]
    pub protocol_token: Box<Account<'info, ProtocolToken>>,

    /// Pre-launch escrow account for this token
    #[account(
        mut,
        seeds = [ESCROW_SEED, token_mint.key().as_ref()],
        bump,
        close = destroyer, // Return rent to destroyer
    )]
    pub escrow: Box<Account<'info, PreLaunchEscrow>>,

    /// Escrow's token vault
    #[account(
        mut,
        associated_token::mint = token_mint,
        associated_token::authority = escrow_authority,
        close = destroyer, // Return rent to destroyer
    )]
    pub escrow_token_vault: Box<Account<'info, TokenAccount>>,

    /// Escrow's FeelsSOL vault (contains mint fee)
    #[account(
        mut,
        constraint = escrow_feelssol_vault.mint == escrow.feelssol_mint @ FeelsError::InvalidMint,
        close = destroyer, // Return rent to destroyer
    )]
    pub escrow_feelssol_vault: Box<Account<'info, TokenAccount>>,

    /// Escrow authority PDA
    /// CHECK: PDA that controls escrow vaults
    #[account(
        seeds = [ESCROW_AUTHORITY_SEED, escrow.key().as_ref()],
        bump = escrow.escrow_authority_bump,
    )]
    pub escrow_authority: AccountInfo<'info>,

    /// Protocol config
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    /// Treasury to receive 50% of mint fee
    #[account(
        mut,
        constraint = treasury.key() == protocol_config.treasury @ FeelsError::InvalidAuthority,
    )]
    pub treasury: Box<Account<'info, TokenAccount>>,

    /// Destroyer's FeelsSOL account to receive 50% of mint fee
    #[account(
        mut,
        constraint = destroyer_feelssol.owner == destroyer.key() @ FeelsError::InvalidAuthority,
        constraint = destroyer_feelssol.mint == escrow.feelssol_mint @ FeelsError::InvalidMint,
    )]
    pub destroyer_feelssol: Box<Account<'info, TokenAccount>>,

    /// Optional: Market account if it was created
    /// CHECK: Market may or may not exist
    #[account(mut)]
    pub market: Option<AccountInfo<'info>>,

    /// Associated token program
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

/// Destroy expired token handler
pub fn destroy_expired_token(ctx: Context<DestroyExpiredToken>) -> Result<()> {
    let clock = Clock::get()?;
    let protocol_token = &ctx.accounts.protocol_token;
    let protocol_config = &ctx.accounts.protocol_config;
    let escrow_feelssol_vault = &ctx.accounts.escrow_feelssol_vault;

    // Check if token has expired
    let expiration_time = protocol_token
        .created_at
        .checked_add(protocol_config.token_expiration_seconds)
        .ok_or(FeelsError::MathOverflow)?;

    require!(
        clock.unix_timestamp > expiration_time,
        FeelsError::TokenNotExpired
    );

    // If market was created, verify it hasn't had liquidity deployed
    if let Some(market_info) = &ctx.accounts.market {
        // Derive expected market PDA
        let (expected_market, _bump) = Pubkey::find_program_address(
            &[
                MARKET_SEED,
                ctx.accounts.token_mint.key().as_ref(),
                ctx.accounts.escrow.feelssol_mint.as_ref(),
            ],
            ctx.program_id,
        );

        // If this is indeed the market account
        if market_info.key() == expected_market {
            // Try to deserialize as Market
            if let Ok(market_data) = market_info.try_borrow_data() {
                if market_data.len() >= 8 + Market::LEN {
                    let market: Market = Market::try_from_slice(&market_data[8..])?;

                    // Ensure no liquidity was deployed
                    require!(
                        !market.initial_liquidity_deployed,
                        FeelsError::MarketAlreadyActive
                    );

                    // Close the market account
                    let dest_starting_lamports = ctx.accounts.destroyer.lamports();
                    **ctx.accounts.destroyer.lamports.borrow_mut() = dest_starting_lamports
                        .checked_add(market_info.lamports())
                        .ok_or(FeelsError::MathOverflow)?;
                    **market_info.lamports.borrow_mut() = 0;

                    // Clear data
                    let mut data = market_info.try_borrow_mut_data()?;
                    data.fill(0);
                }
            }
        }
    }

    // Calculate mint fee split (50% to destroyer, 50% to treasury)
    let mint_fee = escrow_feelssol_vault.amount;
    let destroyer_reward = mint_fee / 2;
    let treasury_amount = mint_fee - destroyer_reward; // Ensures no dust

    msg!("Destroying expired token:");
    msg!("  Token: {}", ctx.accounts.token_mint.key());
    msg!("  Created at: {}", protocol_token.created_at);
    msg!("  Expired at: {}", expiration_time);
    msg!("  Mint fee: {}", mint_fee);
    msg!("  Destroyer reward: {}", destroyer_reward);
    msg!("  Treasury amount: {}", treasury_amount);

    // Transfer rewards if there's a mint fee
    if mint_fee > 0 {
        let escrow_key = ctx.accounts.escrow.key();
        let escrow_authority_seeds = &[
            ESCROW_AUTHORITY_SEED,
            escrow_key.as_ref(),
            &[ctx.accounts.escrow.escrow_authority_bump],
        ];

        // Transfer 50% to destroyer
        if destroyer_reward > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.escrow_feelssol_vault.to_account_info(),
                        to: ctx.accounts.destroyer_feelssol.to_account_info(),
                        authority: ctx.accounts.escrow_authority.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                destroyer_reward,
            )?;
        }

        // Transfer remaining to treasury
        if treasury_amount > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.escrow_feelssol_vault.to_account_info(),
                        to: ctx.accounts.treasury.to_account_info(),
                        authority: ctx.accounts.escrow_authority.to_account_info(),
                    },
                    &[escrow_authority_seeds],
                ),
                treasury_amount,
            )?;
        }
    }

    // Token vaults and accounts will be closed automatically by anchor's close constraint
    // This returns rent to the destroyer as incentive

    // Emit event
    emit!(TokenDestroyed {
        token_mint: ctx.accounts.token_mint.key(),
        destroyer: ctx.accounts.destroyer.key(),
        created_at: protocol_token.created_at,
        destroyed_at: clock.unix_timestamp,
        mint_fee_returned: mint_fee,
        destroyer_reward,
        treasury_amount,
    });

    Ok(())
}
