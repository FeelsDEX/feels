/// Data structures for tick-based liquidity management in concentrated liquidity AMM.
/// Defines TickArray and TickArrayRouter accounts with their size calculations and
/// basic state query methods. Business logic for tick operations is in logic/tick.rs.
/// Zero-copy design optimizes gas usage for high-frequency operations.
/// Also includes 3D tick encoding for the unified order model.

use anchor_lang::prelude::*;
use crate::constant::{MAX_ROUTER_ARRAYS, TICK_ARRAY_SIZE};
use crate::utils::bitmap::{u8_bitmap, bit_encoding};

// ============================================================================
// Tick Data Structures
// ============================================================================

/// Individual tick data within a tick array
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct Tick {
    // Liquidity tracking
    pub liquidity_net: i128,   // Net liquidity change when crossed
    pub liquidity_gross: u128, // Total liquidity referencing this tick

    // Fee tracking (outside the tick) - using 0/1 naming
    pub fee_growth_outside_0: [u64; 4], // Fee growth outside (token 0) - u256 as 4 u64s
    pub fee_growth_outside_1: [u64; 4], // Fee growth outside (token 1) - u256 as 4 u64s

    // Tick metadata
    pub initialized: u8, // Whether this tick is initialized (0 = false, 1 = true)
    pub _padding: [u8; 7], // Explicit padding for 8-byte alignment
}

impl Tick {
    // Business logic methods moved to logic/tick_operations.rs
}

/// Tick array account containing multiple ticks for efficiency
/// Uses zero_copy for optimal Solana performance
///
/// TODO: Future optimization - implement compressed tick storage:
/// - Store only active tick range in account (e.g., current_tick ± 1000)
/// - Archive inactive ticks to merkle tree with proof-based access
/// - Lazy load tick data as price moves through ranges
/// - This would reduce account size from ~10KB to ~1KB per array
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TickArray {
    pub market: Pubkey,                 // Associated market
    pub pool: Pubkey,                   // Associated pool (alias for market)
    pub start_tick_index: i32,          // First tick in this array
    pub ticks: [Tick; TICK_ARRAY_SIZE], // Array of tick data
    pub initialized_tick_count: u8,     // Number of initialized ticks
}

impl TickArray {
    // Size constants for each section of the TickArray struct
    const DISCRIMINATOR_SIZE: usize = 8;
    const MARKET_SIZE: usize = 32;
    const START_TICK_INDEX_SIZE: usize = 4;
    const INITIALIZED_TICK_COUNT_SIZE: usize = 1;

    // Individual Tick struct size breakdown
    const TICK_LIQUIDITY_NET_SIZE: usize = 16; // i128
    const TICK_LIQUIDITY_GROSS_SIZE: usize = 16; // u128
    const TICK_FEE_GROWTH_OUTSIDE_0_SIZE: usize = 32; // [u64; 4]
    const TICK_FEE_GROWTH_OUTSIDE_1_SIZE: usize = 32; // [u64; 4]
    const TICK_INITIALIZED_SIZE: usize = 1; // u8
    const TICK_PADDING_SIZE: usize = 7; // [u8; 7]

    const SINGLE_TICK_SIZE: usize = Self::TICK_LIQUIDITY_NET_SIZE
        + Self::TICK_LIQUIDITY_GROSS_SIZE
        + Self::TICK_FEE_GROWTH_OUTSIDE_0_SIZE
        + Self::TICK_FEE_GROWTH_OUTSIDE_1_SIZE
        + Self::TICK_INITIALIZED_SIZE
        + Self::TICK_PADDING_SIZE; // Total: 104 bytes per tick

    const TICKS_ARRAY_SIZE: usize = Self::SINGLE_TICK_SIZE * TICK_ARRAY_SIZE; // 104 * 32 = 3328 bytes

    pub const SIZE: usize = Self::DISCRIMINATOR_SIZE
        + Self::MARKET_SIZE
        + Self::START_TICK_INDEX_SIZE
        + Self::TICKS_ARRAY_SIZE
        + Self::INITIALIZED_TICK_COUNT_SIZE; // Total: 3373 bytes
}
// ============================================================================
// Tick Array Router System
// ============================================================================

/// Tick array router for efficient access without remaining_accounts
/// This structure enables Valence-compatible operations by pre-registering
/// commonly used tick arrays
#[account]
pub struct TickArrayRouter {
    /// The market this router is associated with
    pub market: Pubkey,

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
    // Size constants for each section of the TickArrayRouter struct
    const DISCRIMINATOR_SIZE: usize = 8;
    const MARKET_MANAGER_SIZE: usize = 32;
    const TICK_ARRAYS_SIZE: usize = 32 * MAX_ROUTER_ARRAYS; // 32 * 8 = 256 bytes
    const START_INDICES_SIZE: usize = 4 * MAX_ROUTER_ARRAYS; // 4 * 8 = 32 bytes
    const ACTIVE_BITMAP_SIZE: usize = 1;
    const LAST_UPDATE_SLOT_SIZE: usize = 8;
    const AUTHORITY_SIZE: usize = 32;
    const RESERVED_SIZE: usize = 64;

    pub const SIZE: usize = Self::DISCRIMINATOR_SIZE
        + Self::MARKET_MANAGER_SIZE
        + Self::TICK_ARRAYS_SIZE
        + Self::START_INDICES_SIZE
        + Self::ACTIVE_BITMAP_SIZE
        + Self::LAST_UPDATE_SLOT_SIZE
        + Self::AUTHORITY_SIZE
        + Self::RESERVED_SIZE; // Total: 433 bytes

    /// Check if a tick array is registered in this router
    pub fn contains_array(&self, start_tick: i32) -> Option<usize> {
        (0..MAX_ROUTER_ARRAYS)
            .find(|&i| self.is_slot_active(i) && self.start_indices[i] == start_tick)
    }

    /// Check if a slot is active
    pub fn is_slot_active(&self, index: usize) -> bool {
        if index >= MAX_ROUTER_ARRAYS {
            return false;
        }
        u8_bitmap::is_bit_set(self.active_bitmap, index).unwrap_or(false)
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

// ============================================================================
// Account Context Structs
// ============================================================================

#[derive(Accounts)]
pub struct InitializeRouter<'info> {
    #[account(
        init,
        payer = authority,
        space = TickArrayRouter::SIZE,
        seeds = [b"router", market_manager.key().as_ref()],
        bump
    )]
    pub router: Account<'info, TickArrayRouter>,

    #[account(mut)]
    pub market_manager: AccountLoader<'info, super::MarketManager>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRouter<'info> {
    #[account(
        mut,
        seeds = [b"router", market_manager.key().as_ref()],
        bump
    )]
    pub router: Account<'info, TickArrayRouter>,

    pub market_manager: AccountLoader<'info, super::MarketManager>,

    pub authority: Signer<'info>,
}

// ============================================================================
// 3D Tick Encoding for Unified Order Model
// ============================================================================

/// 3D tick representation for the unified order model
/// Encodes position across three dimensions: rate, duration, and leverage
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct Tick3D {
    /// Rate dimension - price/interest rate tick
    pub rate_tick: i32,
    
    /// Duration dimension - time commitment tick
    pub duration_tick: i16,
    
    /// Leverage dimension - risk level tick
    pub leverage_tick: i16,
}

impl Tick3D {
    // Bit allocation for encoding
    pub const RATE_BITS: u8 = 20;      // Primary dimension
    pub const DURATION_BITS: u8 = 6;   // Supports Duration enum values
    pub const LEVERAGE_BITS: u8 = 6;   // 64 discrete leverage levels
    
    /// Encode 3D tick into single i32
    pub fn encode(&self) -> i32 {
        let mut packed = 0u64;
        
        // Pack rate tick (primary bits)
        bit_encoding::pack_bits(&mut packed, self.rate_tick as u64, 0, Self::RATE_BITS).unwrap();
        
        // Pack duration tick (middle bits)
        bit_encoding::pack_bits(&mut packed, self.duration_tick as u64, Self::RATE_BITS, Self::DURATION_BITS).unwrap();
        
        // Pack leverage tick (high bits) 
        bit_encoding::pack_bits(&mut packed, self.leverage_tick as u64, Self::RATE_BITS + Self::DURATION_BITS, Self::LEVERAGE_BITS).unwrap();
        
        packed as i32
    }
    
    /// Decode i32 into 3D tick components
    pub fn decode(encoded: i32) -> Self {
        let encoded_u64 = encoded as u64;
        
        let rate_tick = bit_encoding::extract_bits(encoded_u64, 0, Self::RATE_BITS) as i32;
        let duration_tick = bit_encoding::extract_bits(encoded_u64, Self::RATE_BITS, Self::DURATION_BITS) as i16;
        let leverage_tick = bit_encoding::extract_bits(encoded_u64, Self::RATE_BITS + Self::DURATION_BITS, Self::LEVERAGE_BITS) as i16;
        
        Self {
            rate_tick,
            duration_tick,
            leverage_tick,
        }
    }
    
    /// Calculate distance between two 3D ticks
    pub fn distance(&self, other: &Tick3D) -> u64 {
        let rate_diff = (self.rate_tick - other.rate_tick).abs() as u64;
        let duration_diff = (self.duration_tick - other.duration_tick).abs() as u64;
        let leverage_diff = (self.leverage_tick - other.leverage_tick).abs() as u64;
        
        // Simple Manhattan distance for now
        // Could use weighted distance based on dimension importance
        rate_diff + duration_diff * 100 + leverage_diff * 50
    }
    
    /// Check if this tick is within a 3D range
    pub fn in_range(&self, lower: &Tick3D, upper: &Tick3D) -> bool {
        self.rate_tick >= lower.rate_tick && 
        self.rate_tick <= upper.rate_tick &&
        self.duration_tick >= lower.duration_tick &&
        self.duration_tick <= upper.duration_tick &&
        self.leverage_tick >= lower.leverage_tick &&
        self.leverage_tick <= upper.leverage_tick
    }
}
