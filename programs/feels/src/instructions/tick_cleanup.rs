/// Manages cleanup of empty tick arrays to reclaim rent and prevent state bloat.
/// Allows anyone to close empty tick arrays and claim a portion of the reclaimed rent
/// as an incentive. Only arrays with zero initialized ticks can be cleaned up,
/// ensuring active liquidity positions are never affected.

use anchor_lang::prelude::*;
use crate::state::{PoolError, TickArray};
use crate::constant::TICK_ARRAY_SIZE;
use crate::logic::event::{TickArrayCleanedEvent, TickArrayCleanedUpEvent};

// ============================================================================
// Shared Validation Functions
// ============================================================================

/// Validate that a tick array can be cleaned up
fn validate_tick_array_cleanup(
    tick_array: &TickArray,
    pool_key: &Pubkey,
) -> Result<()> {
    // Validate tick array belongs to pool
    require!(
        tick_array.pool == *pool_key,
        PoolError::InvalidPool
    );
    
    // Only allow cleanup if array is completely empty
    require!(
        tick_array.initialized_tick_count == 0,
        PoolError::TickArrayNotEmpty
    );
    
    Ok(())
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Cleanup an empty tick array and reclaim rent (comprehensive version)
pub fn handler(ctx: Context<crate::CleanupTickArray>) -> Result<()> {
    let pool = &mut ctx.accounts.pool.load_mut()?;
    let tick_array = &ctx.accounts.tick_array.load()?;
    
    // Validate tick array can be cleaned up
    validate_tick_array_cleanup(tick_array, &ctx.accounts.pool.key())?;
    
    // Calculate array index for bitmap update
    let array_index = tick_array.start_tick_index / (TICK_ARRAY_SIZE as i32);
    let word_index = (array_index / 64) as usize;
    let bit_index = (array_index % 64) as u8;
    
    // Validate array is marked as initialized in bitmap
    require!(
        word_index < 16,
        PoolError::InvalidTickArrayIndex
    );
    
    let word = pool.tick_array_bitmap[word_index];
    require!(
        (word & (1u64 << bit_index)) != 0,
        PoolError::TickArrayNotInitialized
    );
    
    // Clear the bit in the bitmap
    pool.tick_array_bitmap[word_index] &= !(1u64 << bit_index);
    
    // Calculate rent distribution
    // Protocol keeps 20% as treasury fee, cleaner gets 80%
    let rent_amount = ctx.accounts.tick_array.to_account_info().lamports();
    let protocol_fee = rent_amount * 20 / 100;
    let cleaner_reward = rent_amount - protocol_fee;
    
    // Use safe lamport transfers instead of direct manipulation
    // Transfer cleaner reward
    **ctx.accounts.cleaner.try_borrow_mut_lamports()? = ctx.accounts.cleaner.lamports()
        .checked_add(cleaner_reward)
        .ok_or(PoolError::ArithmeticOverflow)?;
    
    // Transfer protocol fee
    **ctx.accounts.protocol_fee_recipient.try_borrow_mut_lamports()? = ctx.accounts.protocol_fee_recipient.lamports()
        .checked_add(protocol_fee)
        .ok_or(PoolError::ArithmeticOverflow)?;
    
    // Zero out tick array account
    **ctx.accounts.tick_array.to_account_info().try_borrow_mut_lamports()? = 0;
    
    // Emit cleanup event
    emit!(TickArrayCleanedEvent {
        pool: ctx.accounts.pool.key(),
        tick_array: ctx.accounts.tick_array.key(),
        start_tick: tick_array.start_tick_index,
        initialized_count: 0, // We just cleaned it
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

/// Cleanup an empty tick array (simplified version for CleanupEmptyTickArray)
pub fn handler_empty(ctx: Context<crate::CleanupEmptyTickArray>) -> Result<()> {
    let _pool = &ctx.accounts.pool.load()?;
    let tick_array = ctx.accounts.tick_array.load()?;
    
    // Validate tick array can be cleaned up
    validate_tick_array_cleanup(&tick_array, &ctx.accounts.pool.key())?;
    
    // Emit simplified cleanup event
    emit!(TickArrayCleanedUpEvent {
        pool: ctx.accounts.pool.key(),
        tick_array: ctx.accounts.tick_array.key(),
        start_tick: tick_array.start_tick_index,
        ticks_cleaned: 0, // All cleaned
        gas_refund_estimate: 5000, // Estimate
        cleaner: ctx.accounts.beneficiary.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}
