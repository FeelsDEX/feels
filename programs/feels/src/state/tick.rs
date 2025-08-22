/// Manages liquidity distribution across price ranges using space-efficient tick arrays.
/// Groups 60 ticks together to minimize account lookups during swaps. Each tick tracks
/// liquidity changes and fee accumulation. Zero-copy design optimizes gas usage for
/// high-frequency operations. Critical for concentrated liquidity AMM performance.
use anchor_lang::prelude::*;
use crate::state::{Pool, PoolError};

// ============================================================================
// Constants (moved from utils to break circular dependency)
// ============================================================================

pub const TICK_ARRAY_SIZE: usize = 32;
const _TICK_ARRAY_SIZE_BITS: u32 = 5; // log2(32) - unused for now

// ============================================================================
// Tick Data Structures
// ============================================================================

/// Individual tick data within a tick array
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct Tick {
    // Liquidity tracking
    pub liquidity_net: i128,            // Net liquidity change when crossed
    pub liquidity_gross: u128,          // Total liquidity referencing this tick

    // Fee tracking (outside the tick) - using 0/1 naming
    pub fee_growth_outside_0: [u64; 4], // Fee growth outside (token 0) - u256 as 4 u64s
    pub fee_growth_outside_1: [u64; 4], // Fee growth outside (token 1) - u256 as 4 u64s

    // Tick metadata
    pub initialized: u8,                // Whether this tick is initialized (0 = false, 1 = true)
    pub _padding: [u8; 7],              // Explicit padding for 8-byte alignment
}

impl Tick {
    // Business logic methods moved to logic/tick_operations.rs
}

/// Tick array account containing multiple ticks for efficiency
/// Uses zero_copy for optimal Solana performance
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TickArray {
    pub pool: Pubkey,                   // Associated pool
    pub start_tick_index: i32,          // First tick in this array
    pub ticks: [Tick; TICK_ARRAY_SIZE], // Array of tick data
    pub initialized_tick_count: u8,     // Number of initialized ticks
}

impl TickArray {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        4 + // start_tick_index
        (16 + 16 + 32 + 32 + 1 + 7) * TICK_ARRAY_SIZE + // ticks array (104 bytes per tick * 32)
        1; // initialized_tick_count
        // Total: 8 + 32 + 4 + (104 * 32) + 1 = 3373 bytes

    // Business logic methods moved to logic/tick_operations.rs
}
// ============================================================================
// Tick Array Router System
// ============================================================================

/// Maximum number of tick arrays that can be pre-registered in a router
pub const MAX_ROUTER_ARRAYS: usize = 8;

/// Tick array router for efficient access without remaining_accounts
/// This structure enables Valence-compatible operations by pre-registering
/// commonly used tick arrays
#[account]
pub struct TickArrayRouter {
    /// The pool this router is associated with
    pub pool: Pubkey,
    
    /// Pre-registered tick array accounts (up to 8 for Valence compatibility)
    pub tick_arrays: [Pubkey; MAX_ROUTER_ARRAYS],
    
    /// Start tick index for each array (i32::MIN indicates unused slot)
    pub start_indices: [i32; MAX_ROUTER_ARRAYS],
    
    /// Bitmap indicating which slots are active
    pub active_bitmap: u8,
    
    /// Last update slot for cache invalidation
    pub last_update_slot: u64,
    
    /// Authority who can update the router
    pub authority: Pubkey,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl TickArrayRouter {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        32 * MAX_ROUTER_ARRAYS + // tick_arrays
        4 * MAX_ROUTER_ARRAYS + // start_indices
        1 + // active_bitmap
        8 + // last_update_slot
        32 + // authority
        64; // reserved
    
    /// Check if a tick array is registered in this router
    pub fn contains_array(&self, start_tick: i32) -> Option<usize> {
        (0..MAX_ROUTER_ARRAYS).find(|&i| self.is_slot_active(i) && self.start_indices[i] == start_tick)
    }
    
    /// Check if a slot is active
    pub fn is_slot_active(&self, index: usize) -> bool {
        if index >= MAX_ROUTER_ARRAYS {
            return false;
        }
        (self.active_bitmap & (1 << index)) != 0
    }
    
    /// Register a new tick array in the router
    pub fn register_array(
        &mut self,
        tick_array: Pubkey,
        start_tick: i32,
    ) -> Result<usize> {
        // Check if already registered
        if let Some(index) = self.contains_array(start_tick) {
            return Ok(index);
        }
        
        // Find first available slot
        for i in 0..MAX_ROUTER_ARRAYS {
            if !self.is_slot_active(i) {
                self.tick_arrays[i] = tick_array;
                self.start_indices[i] = start_tick;
                self.active_bitmap |= 1 << i;
                return Ok(i);
            }
        }
        
        Err(PoolError::TransientUpdatesFull.into())
    }
    
    /// Unregister a tick array from the router
    pub fn unregister_array(&mut self, start_tick: i32) -> Result<()> {
        if let Some(index) = self.contains_array(start_tick) {
            self.tick_arrays[index] = Pubkey::default();
            self.start_indices[index] = i32::MIN;
            self.active_bitmap &= !(1 << index);
            Ok(())
        } else {
            Err(PoolError::TickNotFound.into())
        }
    }
    
    /// Get the optimal set of tick arrays for a price range
    pub fn get_arrays_for_range(
        &self,
        tick_lower: i32,
        tick_upper: i32,
        tick_spacing: i16,
    ) -> Vec<(usize, Pubkey)> {
        let mut arrays = Vec::new();
        
        // Calculate array boundaries
        let lower_array_start = (tick_lower / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
            * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        let upper_array_start = (tick_upper / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
            * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        
        // Find all arrays in range
        for i in 0..MAX_ROUTER_ARRAYS {
            if !self.is_slot_active(i) {
                continue;
            }
            
            let start = self.start_indices[i];
            if start >= lower_array_start && start <= upper_array_start {
                arrays.push((i, self.tick_arrays[i]));
            }
        }
        
        arrays.sort_by_key(|(i, _)| self.start_indices[*i]);
        arrays
    }
}

/// Router configuration for automatic array selection
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RouterConfig {
    /// Number of arrays to pre-load around current price
    pub arrays_around_current: u8,
    
    /// Update frequency in slots
    pub update_frequency: u64,
    
    /// Whether to auto-update on significant price moves
    pub auto_update_enabled: bool,
    
    /// Price move threshold for auto-update (in ticks)
    pub price_move_threshold: i32,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            arrays_around_current: 3, // Load 3 arrays on each side
            update_frequency: 100,    // Update every 100 slots (~1 minute)
            auto_update_enabled: true,
            price_move_threshold: 100, // Update if price moves 100 ticks
        }
    }
}

/// Initialize a tick array router for a pool
pub fn initialize_router(
    ctx: Context<InitializeRouter>,
    config: RouterConfig,
) -> Result<()> {
    let router = &mut ctx.accounts.router;
    let pool = &ctx.accounts.pool.load()?;
    
    router.pool = ctx.accounts.pool.key();
    router.tick_arrays = [Pubkey::default(); MAX_ROUTER_ARRAYS];
    router.start_indices = [i32::MIN; MAX_ROUTER_ARRAYS];
    router.active_bitmap = 0;
    router.last_update_slot = Clock::get()?.slot;
    router.authority = ctx.accounts.authority.key();
    router._reserved = [0; 64];
    
    // Pre-load arrays around current price
    let current_tick = pool.current_tick;
    let tick_spacing = pool.tick_spacing;
    let current_array_start = (current_tick / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
        * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    
    // Register current array and surrounding arrays
    let arrays_to_load = config.arrays_around_current as i32;
    for i in -arrays_to_load..=arrays_to_load {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        
        // Derive tick array PDA
        let (tick_array_pda, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                ctx.accounts.pool.key().as_ref(),
                &start_tick.to_le_bytes(),
            ],
            ctx.program_id,
        );
        
        // V120 Fix: The PDA derivation above ensures the address is owned by our program
        // Additional validation happens in TickArrayManager::is_initialized which
        // checks the pool's tick bitmap to ensure only valid tick arrays are registered
        
        // Register if it exists (check bitmap)
        if crate::logic::tick_array::TickArrayManager::is_initialized(pool, start_tick) {
            router.register_array(tick_array_pda, start_tick)?;
        }
    }
    
    Ok(())
}

/// Update router with new tick arrays based on current price
pub fn update_router_arrays(
    ctx: Context<UpdateRouter>,
) -> Result<()> {
    let router_key = ctx.accounts.router.key();
    let router = &mut ctx.accounts.router;
    let pool = &ctx.accounts.pool.load()?;
    let clock = Clock::get()?;
    
    // V121 Fix: Validate that caller has authority to update router
    require!(
        ctx.accounts.authority.key() == router.authority,
        PoolError::InvalidAuthority
    );
    
    // Only update if enough time has passed
    require!(
        clock.slot >= router.last_update_slot + 100, // Min 100 slots between updates
        PoolError::InvalidOperation
    );
    
    // V135 Fix: Build new bitmap atomically to avoid race condition
    // Create a temporary copy of the router state
    let mut new_bitmap = 0u64;
    let mut new_arrays = [TickArrayEntry::default(); MAX_TICK_ARRAYS];
    let mut new_count = 0usize;
    
    // Re-populate based on current price
    let current_tick = pool.current_tick;
    let tick_spacing = pool.tick_spacing;
    let current_array_start = (current_tick / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
        * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    
    // Load 3 arrays on each side of current price
    for i in -3..=3 {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        
        // Derive tick array PDA
        let (tick_array_pda, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                ctx.accounts.pool.key().as_ref(),
                &start_tick.to_le_bytes(),
            ],
            ctx.program_id,
        );
        
        // V120 Fix: The PDA derivation above ensures the address is owned by our program
        // Additional validation happens in TickArrayManager::is_initialized
        
        if crate::logic::tick_array::TickArrayManager::is_initialized(pool, start_tick) {
            // Add to temporary arrays
            if new_count < MAX_TICK_ARRAYS {
                new_arrays[new_count] = TickArrayEntry {
                    address: tick_array_pda,
                    start_tick_index: start_tick,
                };
                new_bitmap |= 1u64 << new_count;
                new_count += 1;
            }
        }
    }
    
    // Atomically update the router state
    router.active_bitmap = new_bitmap;
    router.tick_arrays = new_arrays;
    
    router.last_update_slot = clock.slot;
    
    emit!(RouterUpdatedEvent {
        pool: ctx.accounts.pool.key(),
        router: router_key,
        arrays_loaded: router.active_bitmap.count_ones() as u8,
        current_tick: pool.current_tick,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Account Context Structs
// ============================================================================

#[derive(Accounts)]
pub struct InitializeRouter<'info> {
    #[account(
        init,
        payer = authority,
        space = TickArrayRouter::SIZE,
        seeds = [b"router", pool.key().as_ref()],
        bump
    )]
    pub router: Account<'info, TickArrayRouter>,
    
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRouter<'info> {
    #[account(
        mut,
        seeds = [b"router", pool.key().as_ref()],
        bump
    )]
    pub router: Account<'info, TickArrayRouter>,
    
    pub pool: AccountLoader<'info, Pool>,
    
    pub authority: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct RouterUpdatedEvent {
    pub pool: Pubkey,
    pub router: Pubkey,
    pub arrays_loaded: u8,
    pub current_tick: i32,
    pub timestamp: i64,
}
