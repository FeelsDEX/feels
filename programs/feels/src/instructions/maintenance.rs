//! # Maintenance Operations
//! 
//! This module consolidates all keeper and administrative maintenance operations
//! including cleanup, keeper registry management, fee enforcement, and rebase
//! applications. These operations ensure the protocol runs efficiently and
//! maintains its thermodynamic invariants.
//!
//! ## Operations
//! 
//! 1. **Cleanup**: Remove empty tick arrays to reclaim rent
//! 2. **Keeper Management**: Add/remove authorized keepers
//! 3. **Fee Enforcement**: Validate fees and manage pool status
//! 4. **Rebase**: Apply growth factors with conservation law compliance

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::invoke_signed,
    system_instruction,
};
use crate::error::FeelsProtocolError;
use crate::state::*;
use crate::logic::{
    ConservationProof, verify_conservation,
    event::{CleanupEvent, KeeperEvent, RebaseEvent, PoolStatusEvent},
};
use feels_core::constants::*;

// ============================================================================
// Maintenance Operations
// ============================================================================

/// Unified maintenance operation enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum MaintenanceOperation {
    /// Clean up empty tick arrays
    CleanupTickArray {
        /// Single array cleanup
        array_index: i32,
    },
    
    /// Batch cleanup multiple arrays
    BatchCleanupArrays {
        /// Array indices to clean
        array_indices: Vec<i32>,
    },
    
    /// Initialize keeper registry
    InitializeRegistry {
        /// Initial keeper list
        initial_keepers: Vec<Pubkey>,
    },
    
    /// Add a new keeper
    AddKeeper {
        /// Keeper to add
        new_keeper: Pubkey,
    },
    
    /// Remove a keeper
    RemoveKeeper {
        /// Keeper to remove
        keeper: Pubkey,
    },
    
    /// Initialize pool status tracking
    InitializePoolStatus,
    
    /// Update pool operational status
    UpdatePoolStatus {
        /// New status
        new_status: PoolOperationalStatus,
        /// Reason for update
        reason: String,
    },
    
    /// Apply rebase with new indices
    ApplyRebase {
        /// Rebase type
        rebase_type: RebaseType,
        /// Conservation proof
        proof: ConservationProof,
    },
}

/// Types of rebase operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RebaseType {
    /// Update lending/funding indices
    UpdateIndices {
        /// Growth factor for token 0 lending (Q64)
        growth_lending_0: u128,
        /// Growth factor for token 1 lending (Q64)
        growth_lending_1: u128,
        /// Growth factor for long funding (Q64)
        growth_funding_long: u128,
        /// Growth factor for short funding (Q64)
        growth_funding_short: u128,
    },
    
    /// Apply weight rebase when parameters change
    ApplyWeightRebase {
        /// New domain weights
        new_weights: [u32; 4],
        /// Rebase factors for each dimension
        rebase_factors: [u128; 4],
    },
}

/// Pool operational status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum PoolOperationalStatus {
    /// Normal operation
    Active,
    /// Paused for maintenance
    Paused,
    /// Emergency shutdown
    Emergency,
    /// Restricted operations (e.g., withdrawals only)
    Restricted,
}

// ============================================================================
// State Structures
// ============================================================================

/// Keeper registry for authorized maintenance operators
#[account]
#[derive(Default)]
pub struct KeeperRegistry {
    /// Pool this registry belongs to
    pub pool: Pubkey,
    /// Protocol authority
    pub authority: Pubkey,
    /// List of authorized keepers
    pub keepers: [Pubkey; MAX_KEEPERS],
    /// Active status for each keeper
    pub keeper_active: [bool; MAX_KEEPERS],
    /// Registration timestamp for each keeper
    pub keeper_registered_at: [i64; MAX_KEEPERS],
    /// Total number of active keepers
    pub active_keeper_count: u8,
    /// Last update timestamp
    pub last_update: i64,
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

/// Pool status tracking
#[account]
#[derive(Default)]
pub struct PoolStatus {
    /// Pool this status belongs to
    pub pool: Pubkey,
    /// Current operational status
    pub status: PoolOperationalStatus,
    /// Last status update
    pub last_update: i64,
    /// Authority that made last update
    pub last_update_authority: Pubkey,
    /// Reason for last status change
    pub last_update_reason: [u8; 128], // UTF-8 encoded string
    /// Stress indicators
    pub spot_stress: u64,
    pub time_stress: u64,
    pub leverage_stress: u64,
    /// Conservation check failures
    pub conservation_failures: u64,
    /// Total maintenance operations
    pub total_cleanups: u64,
    pub total_rebases: u64,
    /// Reserved
    pub _reserved: [u8; 64],
}

// ============================================================================
// Constants
// ============================================================================

const MAX_KEEPERS: usize = 32;
const MAX_BATCH_CLEANUP: usize = 10;
const CLEANUP_INCENTIVE_RATE: u64 = 8000; // 80% to cleaner
const PROTOCOL_CLEANUP_SHARE: u64 = 2000; // 20% to protocol

// ============================================================================
// Handler Function
// ============================================================================

/// Main handler for all maintenance operations
pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    operation: MaintenanceOperation,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    
    // ========== PHASE 1: VALIDATION ==========
    msg!("Phase 1: Validating maintenance operation");
    
    // Validate authority based on operation type
    validate_maintenance_authority(&operation, &ctx)?;
    
    // ========== PHASE 2: EXECUTION ==========
    msg!("Phase 2: Executing maintenance operation");
    
    match operation {
        MaintenanceOperation::CleanupTickArray { array_index } => {
            execute_cleanup_array(&ctx, array_index, current_time)?;
        }
        
        MaintenanceOperation::BatchCleanupArrays { array_indices } => {
            execute_batch_cleanup(&ctx, array_indices, current_time)?;
        }
        
        MaintenanceOperation::InitializeRegistry { initial_keepers } => {
            execute_initialize_registry(ctx, initial_keepers, current_time)?;
        }
        
        MaintenanceOperation::AddKeeper { new_keeper } => {
            execute_add_keeper(ctx, new_keeper, current_time)?;
        }
        
        MaintenanceOperation::RemoveKeeper { keeper } => {
            execute_remove_keeper(ctx, keeper, current_time)?;
        }
        
        MaintenanceOperation::InitializePoolStatus => {
            execute_initialize_pool_status(ctx, current_time)?;
        }
        
        MaintenanceOperation::UpdatePoolStatus { new_status, reason } => {
            execute_update_pool_status(ctx, new_status, reason, current_time)?;
        }
        
        MaintenanceOperation::ApplyRebase { rebase_type, proof } => {
            execute_apply_rebase(ctx, rebase_type, proof, current_time)?;
        }
    }
    
    // ========== PHASE 3: FINALIZATION ==========
    msg!("Phase 3: Maintenance operation completed");
    
    Ok(())
}

// ============================================================================
// Execution Functions
// ============================================================================

/// Execute tick array cleanup
fn execute_cleanup_array<'info>(
    ctx: &Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    array_index: i32,
    current_time: i64,
) -> Result<()> {
    let tick_array = &ctx.accounts.tick_array;
    
    // Validate array is empty
    require!(
        tick_array.initialized_tick_count == 0,
        FeelsProtocolError::ArrayNotEmpty
    );
    
    // Calculate rent to reclaim
    let rent = Rent::get()?;
    let array_account = tick_array.to_account_info();
    let rent_lamports = rent.minimum_balance(array_account.data_len());
    
    // Distribute rent
    let cleaner_share = if ctx.accounts.market.incentivized_cleanup {
        (rent_lamports * CLEANUP_INCENTIVE_RATE) / 10000
    } else {
        rent_lamports
    };
    let protocol_share = rent_lamports - cleaner_share;
    
    // Transfer rent to cleaner
    **array_account.lamports.borrow_mut() = 0;
    **ctx.accounts.cleaner.lamports.borrow_mut() += cleaner_share;
    
    if protocol_share > 0 {
        **ctx.accounts.protocol_treasury.lamports.borrow_mut() += protocol_share;
    }
    
    // Update pool status if available
    if let Some(pool_status) = &ctx.accounts.pool_status {
        let mut status = pool_status.load_mut()?;
        status.total_cleanups += 1;
    }
    
    // Emit event
    emit!(CleanupEvent {
        pool: ctx.accounts.pool.key(),
        cleaner: ctx.accounts.cleaner.key(),
        array_index,
        rent_reclaimed: rent_lamports,
        cleaner_reward: cleaner_share,
        protocol_share,
        timestamp: current_time,
    });
    
    msg!("Cleaned up tick array {} - reclaimed {} lamports", array_index, rent_lamports);
    
    Ok(())
}

/// Execute batch cleanup
fn execute_batch_cleanup<'info>(
    ctx: &Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    array_indices: Vec<i32>,
    current_time: i64,
) -> Result<()> {
    require!(
        array_indices.len() <= MAX_BATCH_CLEANUP,
        FeelsProtocolError::BatchSizeExceeded
    );
    
    let mut total_rent_reclaimed = 0u64;
    let mut total_cleaner_reward = 0u64;
    let mut total_protocol_share = 0u64;
    let mut arrays_cleaned = 0u32;
    
    // Process each array
    for (i, array_index) in array_indices.iter().enumerate() {
        if i >= ctx.remaining_accounts.len() {
            break;
        }
        
        let array_account = &ctx.remaining_accounts[i];
        
        // Load and validate tick array
        let tick_array = Account::<TickArray>::try_from(array_account)?;
        require!(
            tick_array.initialized_tick_count == 0,
            FeelsProtocolError::ArrayNotEmpty
        );
        
        // Calculate rent
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(array_account.data_len());
        
        // Distribute rent
        let cleaner_share = if ctx.accounts.market_manager.incentivized_cleanup {
            (rent_lamports * CLEANUP_INCENTIVE_RATE) / 10000
        } else {
            rent_lamports
        };
        let protocol_share = rent_lamports - cleaner_share;
        
        total_rent_reclaimed += rent_lamports;
        total_cleaner_reward += cleaner_share;
        total_protocol_share += protocol_share;
        arrays_cleaned += 1;
        
        // Close account
        **array_account.lamports.borrow_mut() = 0;
    }
    
    // Transfer accumulated rewards
    **ctx.accounts.cleaner.lamports.borrow_mut() += total_cleaner_reward;
    if total_protocol_share > 0 {
        **ctx.accounts.protocol_treasury.lamports.borrow_mut() += total_protocol_share;
    }
    
    // Update pool status
    if let Some(pool_status) = &ctx.accounts.pool_status {
        let mut status = pool_status.load_mut()?;
        status.total_cleanups += arrays_cleaned as u64;
    }
    
    // Emit event
    emit!(CleanupEvent {
        pool: ctx.accounts.pool.key(),
        cleaner: ctx.accounts.cleaner.key(),
        array_index: -1, // Batch indicator
        rent_reclaimed: total_rent_reclaimed,
        cleaner_reward: total_cleaner_reward,
        protocol_share: total_protocol_share,
        timestamp: current_time,
    });
    
    msg!("Batch cleaned {} arrays - reclaimed {} lamports", 
        arrays_cleaned, total_rent_reclaimed);
    
    Ok(())
}

/// Initialize keeper registry
fn execute_initialize_registry<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    initial_keepers: Vec<Pubkey>,
    current_time: i64,
) -> Result<()> {
    require!(
        initial_keepers.len() <= MAX_KEEPERS,
        FeelsProtocolError::TooManyKeepers
    );
    
    let registry = &mut ctx.accounts.keeper_registry;
    
    registry.pool = ctx.accounts.pool.key();
    registry.authority = ctx.accounts.authority.key();
    registry.active_keeper_count = 0;
    registry.last_update = current_time;
    
    // Add initial keepers
    for (i, keeper) in initial_keepers.iter().enumerate() {
        registry.keepers[i] = *keeper;
        registry.keeper_active[i] = true;
        registry.keeper_registered_at[i] = current_time;
        registry.active_keeper_count += 1;
    }
    
    emit!(KeeperEvent {
        pool: ctx.accounts.pool.key(),
        event_type: "RegistryInitialized".to_string(),
        keeper: Pubkey::default(),
        authority: ctx.accounts.authority.key(),
        active_count: registry.active_keeper_count,
        timestamp: current_time,
    });
    
    msg!("Initialized keeper registry with {} keepers", initial_keepers.len());
    
    Ok(())
}

/// Add a new keeper
fn execute_add_keeper<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    new_keeper: Pubkey,
    current_time: i64,
) -> Result<()> {
    let registry = &mut ctx.accounts.keeper_registry;
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == registry.authority,
        FeelsProtocolError::Unauthorized
    );
    
    // Check if keeper already exists
    for i in 0..MAX_KEEPERS {
        if registry.keepers[i] == new_keeper && registry.keeper_active[i] {
            return Err(FeelsProtocolError::KeeperAlreadyExists.into());
        }
    }
    
    // Find empty slot
    let mut slot_found = false;
    for i in 0..MAX_KEEPERS {
        if !registry.keeper_active[i] {
            registry.keepers[i] = new_keeper;
            registry.keeper_active[i] = true;
            registry.keeper_registered_at[i] = current_time;
            registry.active_keeper_count += 1;
            slot_found = true;
            break;
        }
    }
    
    require!(slot_found, FeelsProtocolError::TooManyKeepers);
    
    registry.last_update = current_time;
    
    emit!(KeeperEvent {
        pool: ctx.accounts.pool.key(),
        event_type: "KeeperAdded".to_string(),
        keeper: new_keeper,
        authority: ctx.accounts.authority.key(),
        active_count: registry.active_keeper_count,
        timestamp: current_time,
    });
    
    msg!("Added keeper {} - total active: {}", new_keeper, registry.active_keeper_count);
    
    Ok(())
}

/// Remove a keeper
fn execute_remove_keeper<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    keeper: Pubkey,
    current_time: i64,
) -> Result<()> {
    let registry = &mut ctx.accounts.keeper_registry;
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == registry.authority,
        FeelsProtocolError::Unauthorized
    );
    
    // Find and remove keeper
    let mut found = false;
    for i in 0..MAX_KEEPERS {
        if registry.keepers[i] == keeper && registry.keeper_active[i] {
            registry.keeper_active[i] = false;
            registry.active_keeper_count -= 1;
            found = true;
            break;
        }
    }
    
    require!(found, FeelsProtocolError::KeeperNotFound);
    
    registry.last_update = current_time;
    
    emit!(KeeperEvent {
        pool: ctx.accounts.pool.key(),
        event_type: "KeeperRemoved".to_string(),
        keeper,
        authority: ctx.accounts.authority.key(),
        active_count: registry.active_keeper_count,
        timestamp: current_time,
    });
    
    msg!("Removed keeper {} - total active: {}", keeper, registry.active_keeper_count);
    
    Ok(())
}

/// Initialize pool status
fn execute_initialize_pool_status<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    current_time: i64,
) -> Result<()> {
    let pool_status = &mut ctx.accounts.pool_status.unwrap();
    
    pool_status.pool = ctx.accounts.pool.key();
    pool_status.status = PoolOperationalStatus::Active;
    pool_status.last_update = current_time;
    pool_status.last_update_authority = ctx.accounts.authority.key();
    
    // Initialize reason as "Initial"
    let reason_bytes = b"Initial";
    pool_status.last_update_reason[..reason_bytes.len()].copy_from_slice(reason_bytes);
    
    emit!(PoolStatusEvent {
        pool: ctx.accounts.pool.key(),
        old_status: "None".to_string(),
        new_status: "Active".to_string(),
        authority: ctx.accounts.authority.key(),
        reason: "Initial".to_string(),
        spot_stress: 0,
        time_stress: 0,
        leverage_stress: 0,
        timestamp: current_time,
    });
    
    msg!("Initialized pool status as Active");
    
    Ok(())
}

/// Update pool status
fn execute_update_pool_status<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    new_status: PoolOperationalStatus,
    reason: String,
    current_time: i64,
) -> Result<()> {
    let pool_status = &mut ctx.accounts.pool_status.unwrap();
    let old_status = pool_status.status.clone();
    
    // Update status
    pool_status.status = new_status.clone();
    pool_status.last_update = current_time;
    pool_status.last_update_authority = ctx.accounts.authority.key();
    
    // Store reason (truncate if too long)
    let reason_bytes = reason.as_bytes();
    let copy_len = reason_bytes.len().min(128);
    pool_status.last_update_reason[..copy_len].copy_from_slice(&reason_bytes[..copy_len]);
    
    // Calculate stress levels if market data available
    if let (Some(market_manager), Some(oracle)) = (&ctx.accounts.market_manager, &ctx.accounts.oracle) {
        let (spot_stress, time_stress, leverage_stress) = calculate_market_stress(
            market_manager,
            &oracle.load()?,
        )?;
        
        pool_status.spot_stress = spot_stress;
        pool_status.time_stress = time_stress;
        pool_status.leverage_stress = leverage_stress;
    }
    
    emit!(PoolStatusEvent {
        pool: ctx.accounts.pool.key(),
        old_status: format!("{:?}", old_status),
        new_status: format!("{:?}", new_status),
        authority: ctx.accounts.authority.key(),
        reason: reason.clone(),
        spot_stress: pool_status.spot_stress,
        time_stress: pool_status.time_stress,
        leverage_stress: pool_status.leverage_stress,
        timestamp: current_time,
    });
    
    msg!("Updated pool status to {:?}: {}", new_status, reason);
    
    Ok(())
}

/// Apply rebase operation
fn execute_apply_rebase<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
    rebase_type: RebaseType,
    proof: ConservationProof,
    current_time: i64,
) -> Result<()> {
    // Verify conservation law
    verify_conservation(&proof)?;
    
    match rebase_type {
        RebaseType::UpdateIndices { 
            growth_lending_0, 
            growth_lending_1, 
            growth_funding_long, 
            growth_funding_short 
        } => {
            // Update rebase indices
            let rebase_state = &mut ctx.accounts.rebase_state;
            
            // Apply growth factors
            rebase_state.lending_index_0 = safe::mul_u128(
                rebase_state.lending_index_0,
                growth_lending_0,
            )? >> 64;
            
            rebase_state.lending_index_1 = safe::mul_u128(
                rebase_state.lending_index_1,
                growth_lending_1,
            )? >> 64;
            
            rebase_state.funding_index_long = safe::mul_u128(
                rebase_state.funding_index_long,
                growth_funding_long,
            )? >> 64;
            
            rebase_state.funding_index_short = safe::mul_u128(
                rebase_state.funding_index_short,
                growth_funding_short,
            )? >> 64;
            
            rebase_state.last_update = current_time;
            
            emit!(RebaseEvent {
                pool: ctx.accounts.pool.key(),
                rebase_type: "UpdateIndices".to_string(),
                growth_factors: vec![
                    growth_lending_0,
                    growth_lending_1,
                    growth_funding_long,
                    growth_funding_short,
                ],
                new_weights: vec![],
                conservation_sum: proof.conservation_sum,
                authority: ctx.accounts.authority.key(),
                timestamp: current_time,
            });
        }
        
        RebaseType::ApplyWeightRebase { new_weights, rebase_factors } => {
            // Update market weights and apply rebase
            let market = &mut ctx.accounts.market;
            
            // Apply rebase factors to scalars
            market.S = safe::mul_u128(market.S, rebase_factors[0])? >> 64;
            market.T = safe::mul_u128(market.T, rebase_factors[1])? >> 64;
            market.L = safe::mul_u128(market.L, rebase_factors[2])? >> 64;
            
            // Update weights
            market.w_s = new_weights[0];
            market.w_t = new_weights[1];
            market.w_l = new_weights[2];
            market.w_tau = new_weights[3];
            
            emit!(RebaseEvent {
                pool: ctx.accounts.pool.key(),
                rebase_type: "ApplyWeightRebase".to_string(),
                growth_factors: rebase_factors.to_vec(),
                new_weights: new_weights.iter().map(|&w| w as u128).collect(),
                conservation_sum: proof.conservation_sum,
                authority: ctx.accounts.authority.key(),
                timestamp: current_time,
            });
        }
    }
    
    // Update pool status
    if let Some(pool_status) = &ctx.accounts.pool_status {
        let mut status = pool_status.load_mut()?;
        status.total_rebases += 1;
    }
    
    msg!("Applied rebase with conservation sum: {}", proof.conservation_sum);
    
    Ok(())
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate authority for maintenance operations
fn validate_maintenance_authority<'info>(
    operation: &MaintenanceOperation,
    ctx: &Context<'_, '_, 'info, 'info, MaintenanceAccounts<'info>>,
) -> Result<()> {
    match operation {
        // Cleanup operations require registered keeper
        MaintenanceOperation::CleanupTickArray { .. } |
        MaintenanceOperation::BatchCleanupArrays { .. } => {
            if let Some(registry) = &ctx.accounts.keeper_registry {
                require!(
                    is_registered_keeper(&ctx.accounts.authority.key(), registry)?,
                    FeelsProtocolError::NotAuthorizedKeeper
                );
            }
        }
        
        // Registry management requires protocol authority
        MaintenanceOperation::InitializeRegistry { .. } |
        MaintenanceOperation::AddKeeper { .. } |
        MaintenanceOperation::RemoveKeeper { .. } => {
            require!(
                ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
                FeelsProtocolError::Unauthorized
            );
        }
        
        // Pool status updates require keeper or admin
        MaintenanceOperation::InitializePoolStatus |
        MaintenanceOperation::UpdatePoolStatus { .. } => {
            let is_admin = ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority;
            let is_keeper = if let Some(registry) = &ctx.accounts.keeper_registry {
                is_registered_keeper(&ctx.accounts.authority.key(), registry)?
            } else {
                false
            };
            
            require!(
                is_admin || is_keeper,
                FeelsProtocolError::Unauthorized
            );
        }
        
        // Rebase operations have specific requirements
        MaintenanceOperation::ApplyRebase { rebase_type, .. } => {
            match rebase_type {
                RebaseType::UpdateIndices { .. } => {
                    // Index updates allowed by data providers
                    if let Some(data_source) = &ctx.accounts.market_data_source {
                        let source = data_source.load()?;
                        require!(
                            ctx.accounts.authority.key() == source.primary_provider ||
                            ctx.accounts.authority.key() == source.secondary_provider,
                            FeelsProtocolError::Unauthorized
                        );
                    }
                }
                RebaseType::ApplyWeightRebase { .. } => {
                    // Weight rebase requires admin
                    require!(
                        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
                        FeelsProtocolError::Unauthorized
                    );
                }
            }
        }
    }
    
    Ok(())
}

/// Check if pubkey is a registered keeper
fn is_registered_keeper(pubkey: &Pubkey, registry: &Account<KeeperRegistry>) -> Result<bool> {
    for i in 0..MAX_KEEPERS {
        if registry.keepers[i] == *pubkey && registry.keeper_active[i] {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Calculate market stress indicators
fn calculate_market_stress(
    market: &Account<Market>,
    oracle: &UnifiedOracle,
) -> Result<(u64, u64, u64)> {
    // Simplified stress calculation
    // In production, these would involve complex market metrics
    
    let spot_stress = if market.sqrt_price > oracle.get_safe_twap_a() {
        ((market.sqrt_price - oracle.get_safe_twap_a()) * 10000) / oracle.get_safe_twap_a()
    } else {
        0
    } as u64;
    
    let time_stress = 0; // Would calculate based on duration metrics
    let leverage_stress = 0; // Would calculate based on leverage imbalance
    
    Ok((spot_stress, time_stress, leverage_stress))
}

// ============================================================================
// Accounts Structure
// ============================================================================

#[derive(Accounts)]
#[instruction(operation: MaintenanceOperation)]
pub struct MaintenanceAccounts<'info> {
    // Authority performing the operation
    #[account(mut)]
    pub authority: Signer<'info>,
    
    // Pool reference
    /// CHECK: Pool key for PDA derivation
    pub pool: UncheckedAccount<'info>,
    
    // Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    // Market account (unified)
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    // Registry and status
    #[account(mut)]
    pub keeper_registry: Option<Account<'info, KeeperRegistry>>,
    
    #[account(mut)]
    pub pool_status: Option<Account<'info, PoolStatus>>,
    
    // For cleanup operations
    #[account(mut)]
    pub tick_array: Option<Account<'info, TickArray>>,
    
    /// CHECK: Cleaner receives rent
    #[account(mut)]
    pub cleaner: Option<UncheckedAccount<'info>>,
    
    /// CHECK: Protocol treasury
    #[account(mut)]
    pub protocol_treasury: Option<UncheckedAccount<'info>>,
    
    // For rebase operations
    #[account(mut)]
    pub rebase_state: Option<Account<'info, RebaseState>>,
    
    pub market_data_source: Option<AccountLoader<'info, MarketDataSource>>,
    
    pub oracle: Option<AccountLoader<'info, UnifiedOracle>>,
    
    // System program
    pub system_program: Program<'info, System>,
}

// ============================================================================
// Helper Functions
// ============================================================================

mod safe {
    use super::*;
    
    pub fn mul_u128(a: u128, b: u128) -> Result<u128> {
        a.checked_mul(b).ok_or(FeelsProtocolError::MathOverflow.into())
    }
}