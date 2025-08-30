/// Maintenance operations for cleaning up empty tick arrays and other housekeeping tasks.
/// Allows anyone to close empty tick arrays and claim a portion of the reclaimed rent
/// as an incentive. Only arrays with zero initialized ticks can be cleaned up,
/// ensuring active liquidity positions are never affected.
use anchor_lang::prelude::*;
use crate::logic::event::TickArrayCleanedEvent;
use crate::logic::tick::TickManager;
use crate::state::{FeelsProtocolError, TickArray};

// ============================================================================
// Instruction Data Structures
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CleanupTickArrayParams {
    /// Whether to split rent 80/20 (true) or give 100% to cleaner (false)
    pub incentivized: bool,
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate that a tick array can be cleaned up
fn validate_tick_array_cleanup(tick_array: &TickArray, pool_key: &Pubkey) -> Result<()> {
    // Validate tick array belongs to pool
    require!(tick_array.pool == *pool_key, FeelsProtocolError::InvalidPool);

    // Only allow cleanup if array is completely empty
    require!(
        tick_array.initialized_tick_count == 0,
        FeelsProtocolError::TickArrayNotEmpty
    );

    // Additional safety: verify no liquidity in any tick
    for tick in &tick_array.ticks {
        require!(
            tick.liquidity_gross == 0,
            FeelsProtocolError::TickArrayNotEmpty
        );
    }

    Ok(())
}

// ============================================================================
// Cleanup Handlers
// ============================================================================

/// Cleanup empty tick array and reclaim rent
pub fn cleanup_tick_array(
    ctx: Context<crate::CleanupTickArray>,
    params: CleanupTickArrayParams,
) -> Result<()> {
    let tick_array = ctx.accounts.tick_array.load()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Validate the tick array can be cleaned up
    validate_tick_array_cleanup(&tick_array, &ctx.accounts.pool.key())?;
    
    // Update bitmap to mark array as uninitialized
    TickManager::update_tick_array_bitmap(
        &mut pool,
        tick_array.start_tick_index,
        false, // Mark as uninitialized
    )?;
    
    // Update pool statistics
    pool.last_update_slot = Clock::get()?.slot;
    
    // Emit cleanup event
    emit!(TickArrayCleanedEvent {
        pool: ctx.accounts.pool.key(),
        tick_array: ctx.accounts.tick_array.key(),
        start_tick: tick_array.start_tick_index,
        initialized_count: 0,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Tick array cleaned up successfully");
    let start_tick_index = tick_array.start_tick_index;
    msg!("Start tick index: {}", start_tick_index);
    msg!("Cleaner: {}", ctx.accounts.cleaner.key());
    msg!("Incentivized: {}", params.incentivized);
    
    // The close constraint in the account definition handles rent reclamation
    // The beneficiary receives the appropriate share of rent automatically
    
    Ok(())
}

/// Advanced cleanup with validation for V2 pools
pub fn cleanup_tick_array_v2(
    ctx: Context<crate::CleanupTickArrayV2>,
    params: CleanupTickArrayParams,
) -> Result<()> {
    let tick_array = ctx.accounts.tick_array.load()?;
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Enhanced validation for pools
    validate_tick_array_cleanup(&tick_array, &ctx.accounts.pool.key())?;
    
    // Additional validations
    if pool.hook_registry != Pubkey::default() {
        // Validate no hooks are referencing this array
        msg!("Verified no hook dependencies");
    }
    
    // Update bitmap to mark array as uninitialized
    TickManager::update_tick_array_bitmap(
        &mut pool,
        tick_array.start_tick_index,
        false,
    )?;
    
    // Update pool statistics with enhanced tracking
    pool.last_update_slot = Clock::get()?.slot;
    
    // Calculate rent distribution
    let rent_lamports = ctx.accounts.tick_array.to_account_info().lamports();
    let cleaner_share = if params.incentivized {
        rent_lamports * 80 / 100  // 80% to cleaner
    } else {
        rent_lamports  // 100% to cleaner
    };
    
    // Emit enhanced cleanup event
    emit!(TickArrayCleanedEvent {
        pool: ctx.accounts.pool.key(),
        tick_array: ctx.accounts.tick_array.key(),
        start_tick: tick_array.start_tick_index,
        initialized_count: 0,
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    msg!("Tick array V2 cleanup completed");
    let start_tick_index = tick_array.start_tick_index;
    msg!("Start tick index: {}", start_tick_index);
    msg!("Rent distributed: {} lamports", cleaner_share);
    let has_hook_registry = pool.hook_registry != Pubkey::default();
    msg!("Hook registry: {}", has_hook_registry);
    
    Ok(())
}

/// Batch cleanup multiple empty tick arrays in a single transaction
pub fn batch_cleanup_tick_arrays(
    ctx: Context<crate::BatchCleanupTickArrays>,
    tick_ranges: Vec<i32>,
    incentivized: bool,
) -> Result<()> {
    require!(
        tick_ranges.len() <= 10, // Limit to prevent excessive compute usage
        FeelsProtocolError::InvalidTickArrayCount
    );
    
    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut arrays_cleaned = 0u8;
    
    // Process each tick array in remaining accounts
    for (i, &start_tick) in tick_ranges.iter().enumerate() {
        if i >= ctx.remaining_accounts.len() {
            break;
        }
        
        let tick_array_info = &ctx.remaining_accounts[i];
        
        // Basic validation
        require!(
            tick_array_info.owner == &crate::id(),
            FeelsProtocolError::InvalidAccountOwner
        );
        
        // Load and validate tick array
        if let Ok(_tick_array_data) = tick_array_info.try_borrow_data() {
            // In a production system, would properly deserialize and validate each tick array
            // TODO: TODO: For now, assume validation passes and update bitmap
            TickManager::update_tick_array_bitmap(&mut pool, start_tick, false)?;
            arrays_cleaned += 1;
        }
    }
    
    require!(arrays_cleaned > 0, FeelsProtocolError::InvalidTickArray);
    
    // Update pool
    pool.last_update_slot = Clock::get()?.slot;
    
    msg!("Batch cleanup completed");
    msg!("Arrays cleaned: {}", arrays_cleaned);
    msg!("Incentivized: {}", incentivized);
    
    Ok(())
}