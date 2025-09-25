use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{math::price_to_tick, state::pool::Pool};

#[derive(Accounts)]
#[instruction(fee_bps: u16)]
pub struct CreatePool<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [
            b"pool",
            token_mint_a.key().as_ref(),
            token_mint_b.key().as_ref(),
            &fee_bps.to_le_bytes()
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// Token A mint (ordered: token_a < token_b)
    pub token_mint_a: Account<'info, Mint>,

    /// Token B mint (ordered: token_a < token_b)
    pub token_mint_b: Account<'info, Mint>,

    /// Pool's token A vault
    #[account(
        init,
        payer = payer,
        token::mint = token_mint_a,
        token::authority = pool,
        seeds = [b"vault_a", pool.key().as_ref()],
        bump
    )]
    pub token_vault_a: Account<'info, TokenAccount>,

    /// Pool's token B vault
    #[account(
        init,
        payer = payer,
        token::mint = token_mint_b,
        token::authority = pool,
        seeds = [b"vault_b", pool.key().as_ref()],
        bump
    )]
    pub token_vault_b: Account<'info, TokenAccount>,

    /// Pool creator/payer
    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_pool(
    ctx: Context<CreatePool>,
    fee_bps: u16,             // Pool fee tier in basis points (e.g., 50 = 0.5%)
    protocol_fee_bps: u16,    // Portion of fee_bps that goes to protocol (in basis points)
    tick_spacing: i32,        // Minimum tick spacing for this fee tier
    initial_sqrt_price: u128, // Initial sqrt price (Q64.64 format)
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    // Initialize pool state
    pool.token_mint_a = ctx.accounts.token_mint_a.key();
    pool.token_mint_b = ctx.accounts.token_mint_b.key();
    pool.token_vault_a = ctx.accounts.token_vault_a.key();
    pool.token_vault_b = ctx.accounts.token_vault_b.key();

    pool.fee_bps = fee_bps;
    pool.protocol_fee_bps = protocol_fee_bps;
    pool.tick_spacing = tick_spacing;
    pool.sqrt_price = initial_sqrt_price;
    pool.tick = price_to_tick(initial_sqrt_price)?;

    pool.liquidity = 0;
    pool.fee_growth_global_a = 0;
    pool.fee_growth_global_b = 0;

    Ok(())
}
