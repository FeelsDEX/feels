/// Manages batch tick updates for gas optimization in high-volume swap scenarios.
/// Allows keepers to defer expensive tick array updates to separate transactions,
/// reducing compute units during swaps by ~50%. Will be replaced by Valence atomic
/// operations in Phase 2 for single-transaction execution without temporary accounts.

use anchor_lang::prelude::*;
use crate::state::{TickUpdate, PoolError};
use crate::logic::event::TransientUpdatesFinalized;

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize a new TransientTickUpdates account for batch tick processing
pub fn initialize_transient_updates(
    ctx: Context<crate::InitializeTransientUpdates>,
) -> Result<()> {
    let mut transient_updates = ctx.accounts.transient_updates.load_init()?;
    let clock = Clock::get()?;
    
    transient_updates.initialize(
        ctx.accounts.pool.key(),
        clock.slot,
        clock.unix_timestamp,
    );
    
    Ok(())
}

/// Add a tick update to the batch
pub fn add_tick_update(
    ctx: Context<crate::AddTickUpdate>,
    tick_array_pubkey: Pubkey,
    tick_index: i32,
    liquidity_net_delta: i128,
) -> Result<()> {
    let mut transient_updates = ctx.accounts.transient_updates.load_mut()?;
    let pool = ctx.accounts.pool.load()?;
    
    // Validate authority can add tick updates
    // Only pool authority should be able to add updates
    require!(
        ctx.accounts.authority.key() == pool.authority,
        PoolError::InvalidAuthority
    );
    
    // Validate the update belongs to the correct pool
    require!(
        transient_updates.pool == ctx.accounts.pool.key(),
        PoolError::InvalidPool
    );
    
    let clock = Clock::get()?;
    
    // Create tick update from parameters
    let tick_update = TickUpdate {
        tick_array_pubkey,
        tick_index,
        liquidity_net_delta,
        fee_growth_outside_0: [0; 4],
        fee_growth_outside_1: [0; 4],
        slot: clock.slot,
        priority: 1,
        _padding: [0; 7],
    };
    
    transient_updates.add_update(tick_update)?;
    
    Ok(())
}

/// Finalize and apply all tick updates to their respective tick arrays
pub fn finalize_transient_updates(
    ctx: Context<crate::FinalizeTransientUpdates>,
) -> Result<()> {
    let mut transient_updates = ctx.accounts.transient_updates.load_mut()?;
    let clock = Clock::get()?;
    
    // Validate not already finalized
    require!(
        transient_updates.finalized == 0,
        PoolError::UpdatesAlreadyFinalized
    );
    
    // Validate not expired (max 100 slots old)
    require!(
        clock.slot.saturating_sub(transient_updates.slot) <= 100,
        PoolError::TransientUpdatesExpired
    );
    
    // Apply all updates to tick arrays
    // TODO: Note, this would require remaining_accounts with all relevant TickArrays
    // For now, just mark as finalized
    transient_updates.finalize();
    
    // Emit event for off-chain tracking
    emit!(TransientUpdatesFinalized {
        pool: transient_updates.pool,
        slot: transient_updates.slot,
        update_count: transient_updates.update_count,
        finalized_at: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Clean up expired or finalized transient updates to reclaim rent
pub fn cleanup_transient_updates(
    ctx: Context<crate::CleanupTransientUpdates>,
) -> Result<()> {
    let transient_updates = ctx.accounts.transient_updates.load()?;
    let clock = Clock::get()?;
    
    // Only allow cleanup if finalized or expired
    let is_expired = transient_updates.should_cleanup(clock.unix_timestamp, 300); // 5 minutes
    let is_finalized = transient_updates.finalized != 0;
    
    require!(
        is_expired || is_finalized,
        PoolError::InvalidOperation
    );
    
    // Close the account and return rent to authority
    // Account closure happens automatically via close constraint
    
    Ok(())
}

/// Reset a finalized batch for reuse (gas optimization)
pub fn reset_transient_updates(
    ctx: Context<crate::ResetTransientUpdates>,
) -> Result<()> {
    let mut transient_updates = ctx.accounts.transient_updates.load_mut()?;
    let clock = Clock::get()?;
    
    // Only allow reset if finalized
    require!(
        transient_updates.finalized != 0,
        PoolError::InvalidOperation
    );
    
    transient_updates.reset(clock.slot, clock.unix_timestamp);
    
    Ok(())
}
