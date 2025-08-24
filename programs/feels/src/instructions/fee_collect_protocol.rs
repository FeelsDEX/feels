/// Collects protocol fees that accumulate from a portion of all trading fees.
/// Only callable by the protocol authority, these fees fund protocol development,
/// liquidity incentives, and treasury operations. The protocol's fee share is
/// configurable per pool and represents the platform's revenue model.

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};
use crate::state::PoolError;
use crate::logic::event::ProtocolFeeCollectionEvent;

// ============================================================================
// Handler Functions
// ============================================================================

/// Collect accumulated protocol fees from a pool
/// 
/// This is a stub implementation for Phase 1. Protocol fees accumulate
/// in the pool and can be collected by the protocol authority.
pub fn handler(
    ctx: Context<crate::CollectProtocolFees>,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Result<(u64, u64)> {
    let pool = &mut ctx.accounts.pool.load_mut()?;
    
    // Validate authority
    require!(ctx.accounts.authority.key() == pool.authority, PoolError::InvalidAuthority);
    
    // Calculate amounts to collect (min of requested and available)
    let amount_0 = amount_0_requested.min(pool.protocol_fees_0);
    let amount_1 = amount_1_requested.min(pool.protocol_fees_1);
    
    // Update pool state using native checked arithmetic
    pool.protocol_fees_0 = pool.protocol_fees_0
        .checked_sub(amount_0)
        .ok_or(PoolError::ArithmeticUnderflow)?;
    pool.protocol_fees_1 = pool.protocol_fees_1
        .checked_sub(amount_1)
        .ok_or(PoolError::ArithmeticUnderflow)?;
    
    // Get pool authority seeds for CPI signing
    let pool_seeds = crate::utils::CanonicalSeeds::get_pool_seeds(
        &pool.token_a_mint,
        &pool.token_b_mint,
        pool.fee_rate,
        ctx.bumps.pool,
    );

    // Transfer fees to recipient
    // Note: Transfer logic kept inline for Phase 2 Valence hook integration.
    // Protocol fee collection may remain as simple transfers while trading uses Valence.
    if amount_0 > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_0.to_account_info(),
                    to: ctx.accounts.recipient_0.to_account_info(),
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
                    to: ctx.accounts.recipient_1.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&pool_seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>()],
            ),
            amount_1,
        )?;
    }
    
    // Update timestamp with consistent clock
    let clock = Clock::get()?;
    pool.last_update_slot = clock.slot;
    
    // Emit event
    emit!(ProtocolFeeCollectionEvent {
        pool: ctx.accounts.pool.key(),
        collector: ctx.accounts.authority.key(),
        amount_0,
        amount_1,
        timestamp: clock.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}
