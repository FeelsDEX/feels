/// Collects protocol fees that accumulate from a portion of all trading fees.
/// Only callable by the protocol authority, these fees fund protocol development,
/// liquidity incentives, and treasury operations. The protocol's fee share is
/// configurable per pool and represents the platform's revenue model.

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};
use crate::state::PoolError;
use crate::utils::SafeMath;

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
    
    // Update pool state using safe arithmetic
    pool.protocol_fees_0 = pool.protocol_fees_0.safe_sub(amount_0)?;
    pool.protocol_fees_1 = pool.protocol_fees_1.safe_sub(amount_1)?;
    
    // Get the canonical token order to derive proper seeds
    let token_a_key = pool.token_a_mint;
    let token_b_key = pool.token_b_mint;
    let pool_fee_rate = pool.fee_rate;

    // Transfer fees to recipient
    if amount_0 > 0 {
        #[allow(deprecated)]
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_0.to_account_info(),
                    to: ctx.accounts.recipient_0.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&[
                    b"pool",
                    token_a_key.as_ref(),
                    token_b_key.as_ref(),
                    &pool_fee_rate.to_le_bytes(),
                    &[ctx.bumps.pool],
                ]],
            ),
            amount_0,
        )?;
    }
    
    if amount_1 > 0 {
        #[allow(deprecated)]
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_1.to_account_info(),
                    to: ctx.accounts.recipient_1.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&[
                    b"pool",
                    token_a_key.as_ref(),
                    token_b_key.as_ref(),
                    &pool_fee_rate.to_le_bytes(),
                    &[ctx.bumps.pool],
                ]],
            ),
            amount_1,
        )?;
    }
    
    // Update timestamp
    pool.last_update_slot = Clock::get()?.slot;
    
    // Emit event
    emit!(ProtocolFeeCollectionEvent {
        pool: ctx.accounts.pool.key(),
        authority: ctx.accounts.authority.key(),
        amount_0,
        amount_1,
        total_collected_0: pool.total_volume_0, // Could track separately
        total_collected_1: pool.total_volume_1,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Protocol fee collection: {} token0, {} token1", amount_0, amount_1);
    msg!("TODO: Implement fee distribution to TickPositionVault in Phase 2");
    
    Ok((amount_0, amount_1))
}

// ============================================================================
// Events
// ============================================================================

/// Event emitted when protocol fees are collected
#[event]
pub struct ProtocolFeeCollectionEvent {
    #[index]
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
    pub total_collected_0: u128,
    pub total_collected_1: u128,
    pub timestamp: i64,
}

impl crate::logic::EventBase for ProtocolFeeCollectionEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn actor(&self) -> Pubkey {
        self.authority
    }
}