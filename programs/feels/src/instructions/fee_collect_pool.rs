/// Collects accumulated trading fees earned by a liquidity position.
/// Calculates fees based on the position's share of liquidity and fee growth
/// since last collection. Transfers earned fees from pool vaults to the position
/// owner's token accounts. Critical for LP profitability in the AMM model.

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};
use crate::state::PoolError;
use crate::logic::event::FeeCollectionEvent;

// ============================================================================
// Handler Functions
// ============================================================================

/// Collect accumulated fees for a liquidity position
/// 
/// This is a stub implementation for Phase 1. In a complete implementation,
/// this would:
/// 1. Calculate fees owed based on fee growth since last collection
/// 2. Update position's fee growth checkpoints
/// 3. Transfer fees from pool to user
/// 4. Reset tokens_owed counters
pub fn handler(
    ctx: Context<crate::CollectFees>,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Result<(u64, u64)> {
    let pool = &ctx.accounts.pool.load()?;
    let position = &mut ctx.accounts.position;
    
    // Validate position belongs to pool
    require!(position.pool == ctx.accounts.pool.key(), PoolError::InvalidPool);
    
    // Validate pool token vaults match
    require!(pool.token_a_vault == ctx.accounts.token_vault_0.key(), PoolError::InvalidPool);
    require!(pool.token_b_vault == ctx.accounts.token_vault_1.key(), PoolError::InvalidPool);
    
    // TODO: In a full implementation, we would:
    // 1. Calculate fee growth inside the position's range
    // 2. Calculate fees owed: liquidity * (fee_growth_inside - fee_growth_inside_last) / 2^128
    // 3. Add to tokens_owed_0 and tokens_owed_1
    
    // For now, just collect what's already in tokens_owed
    let amount_0 = amount_0_requested.min(position.tokens_owed_0);
    let amount_1 = amount_1_requested.min(position.tokens_owed_1);
    
    // Update position state using native checked arithmetic
    position.tokens_owed_0 = position.tokens_owed_0
        .checked_sub(amount_0)
        .ok_or(PoolError::ArithmeticUnderflow)?;
    position.tokens_owed_1 = position.tokens_owed_1
        .checked_sub(amount_1)
        .ok_or(PoolError::ArithmeticUnderflow)?;
    
    // Get pool PDA seeds for authority
    let pool_data = ctx.accounts.pool.load()?;
    let pool_seeds = crate::utils::CanonicalSeeds::get_pool_seeds(
        &pool_data.token_a_mint,
        &pool_data.token_b_mint,
        pool_data.fee_rate,
        ctx.bumps.pool,
    );

    // Transfer fees to user
    // Note: Transfer logic kept inline for Phase 2 Valence hook integration.
    // Fee collection may have different atomic requirements than swaps/liquidity.
    if amount_0 > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_0.to_account_info(),
                    to: ctx.accounts.token_account_0.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&pool_seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>()],
            ),
            amount_0,
        )?;
    }
    
    if amount_1 > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_1.to_account_info(),
                    to: ctx.accounts.token_account_1.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&pool_seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>()],
            ),
            amount_1,
        )?;
    }
    
    // Emit event
    emit!(FeeCollectionEvent {
        pool: ctx.accounts.pool.key(),
        position: ctx.accounts.position.key(),
        amount_0,
        amount_1,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}
