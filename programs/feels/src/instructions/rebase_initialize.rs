/// Initialize the rebase accumulator for a pool to enable virtual rebasing with lazy evaluation.
/// This sets up yield accrual and funding rate mechanisms without backwards compatibility concerns.
use anchor_lang::prelude::*;
use crate::state::{Pool, RebaseAccumulator};

// ============================================================================
// Instruction Handler
// ============================================================================

pub fn handler(ctx: Context<InitializeRebase>) -> Result<()> {
    let pool = &mut ctx.accounts.pool.load_mut()?;
    let rebase = &mut ctx.accounts.rebase_accumulator.load_init()?;
    
    // Initialize rebase accumulator with pool's token weights
    // Default weights if not specified: 50/50 for tokens, 50/50 for long/short
    let weight_a = 50;
    let weight_b = 50;
    let weight_long = 50;
    let weight_short = 50;
    
    rebase.initialize(weight_a, weight_b, weight_long, weight_short);
    
    // Link pool to rebase accumulator
    pool.rebase_accumulator = ctx.accounts.rebase_accumulator.key();
    
    msg!("Rebase accumulator initialized for pool {}", ctx.accounts.pool.key());
    msg!("Weights: A={}, B={}, Long={}, Short={}", weight_a, weight_b, weight_long, weight_short);
    
    Ok(())
}

// ============================================================================
// Account Structures  
// ============================================================================

#[derive(Accounts)]
pub struct InitializeRebase<'info> {
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<RebaseAccumulator>(),
        seeds = [b"rebase", pool.key().as_ref()],
        bump
    )]
    pub rebase_accumulator: AccountLoader<'info, RebaseAccumulator>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}