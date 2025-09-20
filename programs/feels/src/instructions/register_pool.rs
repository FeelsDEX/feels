use crate::{
    error::FeelsError,
    events::PoolRegistered,
    state::{Market, PoolEntry, PoolPhase, PoolRegistry},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct RegisterPool<'info> {
    /// Pool registry
    #[account(
        mut,
        seeds = [PoolRegistry::SEED],
        bump = pool_registry.bump,
        realloc = pool_registry.to_account_info().data_len() + PoolRegistry::POOL_ENTRY_SIZE,
        realloc::payer = payer,
        realloc::zero = false,
    )]
    pub pool_registry: Account<'info, PoolRegistry>,

    /// Market to register
    #[account(
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
    )]
    pub market: Account<'info, Market>,

    // Token metadata removed for MVP - will use hardcoded symbol
    /// Project token mint (non-FeelsSOL token)
    /// CHECK: Validated against market tokens
    pub project_mint: AccountInfo<'info>,

    /// Creator registering the pool
    pub creator: Signer<'info>,

    /// Payer for realloc
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program
    pub system_program: Program<'info, System>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn register_pool(ctx: Context<RegisterPool>) -> Result<()> {
    let registry = &mut ctx.accounts.pool_registry;
    let market = &ctx.accounts.market;
    let clock = &ctx.accounts.clock;

    // Determine which token is the project token (non-FeelsSOL)
    let project_mint = if market.token_0 == market.feelssol_mint {
        market.token_1
    } else {
        market.token_0
    };

    // Validate project mint matches
    require!(
        ctx.accounts.project_mint.key() == project_mint,
        FeelsError::InvalidMint
    );

    // Extract symbol from metadata
    // Use hardcoded symbol for MVP - will be replaced with metadata later
    let symbol_str = "TOKEN";
    let mut symbol = [0u8; 10];
    let symbol_bytes = symbol_str.as_bytes();
    let symbol_len = symbol_bytes.len().min(10);
    symbol[..symbol_len].copy_from_slice(&symbol_bytes[..symbol_len]);

    // Determine initial phase based on market state
    let phase = if market.steady_state_seeded {
        PoolPhase::SteadyState
    } else {
        PoolPhase::BondingCurve
    };

    // Create pool entry
    let entry = PoolEntry {
        market: market.key(),
        token_mint: project_mint,
        feelssol_mint: market.feelssol_mint,
        phase,
        created_at: clock.unix_timestamp,
        updated_at: clock.unix_timestamp,
        creator: ctx.accounts.creator.key(),
        symbol,
        symbol_len: symbol_len as u8,
        _reserved: [0; 32],
    };

    // Add to registry
    registry.add_pool(entry)?;

    // Emit event
    emit!(PoolRegistered {
        market: market.key(),
        token_mint: project_mint,
        feelssol_mint: market.feelssol_mint,
        creator: ctx.accounts.creator.key(),
        symbol: symbol_str.to_string(),
        phase: phase as u8,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
