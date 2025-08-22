/// Core pool state account implementing concentrated liquidity with Uniswap V3-style mechanics.
/// Stores price, liquidity, fee data, and tick bitmap for efficient position management.
/// Designed with 512 bytes of reserved space for future Phase 2/3 upgrades including
/// leverage parameters, enhanced oracles, and three-dimensional trading capabilities.
use anchor_lang::prelude::*;

// ============================================================================
// Phase 2 Constants (Three-Dimensional System)
// ============================================================================

// Three-dimensional system constants (Phase 2 preparation)
pub const RATE_BITS: u8 = 20;
pub const DURATION_BITS: u8 = 6;
pub const LEVERAGE_BITS: u8 = 6;

// ============================================================================
// Phase 2+ Types (Three-Dimensional System)
// ============================================================================

/// Duration types for Phase 2+ three-dimensional system
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum Duration {
    Flash = 0,      // 1 block
    Swap = 1,       // Immediate (spot)
    Weekly = 2,     // 7 days
    Monthly = 3,    // 28 days
    Quarterly = 4,  // 90 days
    Annual = 5,     // 365 days
}

impl Duration {
    pub const COUNT: usize = 6;
    
    pub fn to_blocks(&self) -> u64 {
        match self {
            Duration::Flash => 1,
            Duration::Swap => 0,
            Duration::Weekly => 7 * 24 * 60 * 5,     // 5 blocks/min
            Duration::Monthly => 28 * 24 * 60 * 5,
            Duration::Quarterly => 90 * 24 * 60 * 5,
            Duration::Annual => 365 * 24 * 60 * 5,
        }
    }
}

/// Three-dimensional tick structure for Phase 2+
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct Tick3D {
    pub rate_tick: i32,
    pub duration_tick: i16,
    pub leverage_tick: i16,
}

impl Tick3D {
    /// Encode 3D tick into single i32 for efficient storage
    pub fn encode(&self) -> Result<i32> {
        // V119 Fix: Add overflow checks for bit shifting operations
        // Validate that values fit within their allocated bit ranges
        
        // Rate uses primary bits (highest precision needed)
        if self.rate_tick.abs() >= (1 << RATE_BITS) {
            return Err(PoolError::InvalidTickRange.into());
        }
        let rate_masked = self.rate_tick & ((1 << RATE_BITS) - 1);
        
        // Duration uses 6 bits (supports Duration enum)
        if self.duration_tick.abs() >= (1 << DURATION_BITS) {
            return Err(PoolError::InvalidTickRange.into());
        }
        let duration_shifted = (self.duration_tick as i32)
            .checked_shl(RATE_BITS as u32)
            .ok_or(PoolError::MathOverflow)?;
        
        // Leverage uses 6 bits (64 discrete levels of continuous leverage)
        if self.leverage_tick.abs() >= (1 << LEVERAGE_BITS) {
            return Err(PoolError::InvalidTickRange.into());
        }
        let leverage_shifted = (self.leverage_tick as i32)
            .checked_shl((RATE_BITS + DURATION_BITS) as u32)
            .ok_or(PoolError::MathOverflow)?;
        
        // Combine with overflow protection
        let result = rate_masked | duration_shifted | leverage_shifted;
        Ok(result)
    }
    
    /// Decode i32 back into 3D tick structure
    pub fn decode(encoded: i32) -> Self {
        let rate_tick = encoded & ((1 << RATE_BITS) - 1);
        let duration_tick = ((encoded >> RATE_BITS) & ((1 << DURATION_BITS) - 1)) as i16;
        let leverage_tick = ((encoded >> (RATE_BITS + DURATION_BITS)) & ((1 << LEVERAGE_BITS) - 1)) as i16;
        
        Self { rate_tick, duration_tick, leverage_tick }
    }
}

// ============================================================================
// Phase 1 Core Structures
// ============================================================================

/// Core Pool account for concentrated liquidity AMM (Phase 1)
/// Matches specification exactly with proper u256 handling
#[account(zero_copy)]
#[repr(C, packed)]
pub struct Pool {
    // Version control
    pub version: u8,                    // Set to 1 for Phase 1
    pub _padding: [u8; 7],              // Explicit padding for alignment
    
    // Token configuration
    pub token_a_mint: Pubkey,           // Any token mint
    pub token_b_mint: Pubkey,           // Always FeelsSOL mint
    pub token_a_vault: Pubkey,          // Token A vault PDA
    pub token_b_vault: Pubkey,          // FeelsSOL vault PDA
    
    // Fee configuration
    pub fee_rate: u16,                  // Fee tier in basis points (1, 5, 30, 100)
    pub protocol_fee_rate: u16,         // Protocol's share of fees
    
    // Price and liquidity state
    pub current_tick: i32,              // Current price tick
    pub current_sqrt_price: u128,       // Square root of price (Q64.96)
    pub liquidity: u128,                // Total active liquidity
    
    // Tick bitmap for efficient searching (1024-bit bitmap)
    pub tick_array_bitmap: [u64; 16],   
    pub tick_spacing: i16,              // Minimum tick spacing
    pub _padding2: [u8; 6],             // Explicit padding for alignment
    
    // Fee tracking (using [u64; 4] to represent u256)
    pub fee_growth_global_0: [u64; 4],  // Cumulative fees (token 0)
    pub fee_growth_global_1: [u64; 4],  // Cumulative fees (token 1) 
    pub protocol_fees_0: u64,           // Uncollected protocol fees
    pub protocol_fees_1: u64,           // Uncollected protocol fees
    
    // Pool metadata
    pub authority: Pubkey,              // Pool authority
    pub creation_timestamp: i64,        // Creation time
    pub last_update_slot: u64,          // Last update slot
    
    // Statistics
    pub total_volume_0: u128,           // Cumulative volume
    pub total_volume_1: u128,           // Cumulative volume
    
    // Future upgrade space
    pub _reserved: [u8; 512],           // Reserved for Phase 2+
}

impl Pool {
    pub const SIZE: usize = 8 +         // Discriminator
        1 +                             // version
        32 * 4 +                        // token configuration (128)
        2 + 2 +                         // fee rates (4)
        4 + 16 + 16 +                   // price and liquidity (36)
        128 + 2 +                       // tick bitmap (130)
        32 + 32 + 8 + 8 +               // fee tracking (80)
        32 + 8 + 8 +                    // metadata (48)
        16 + 16 +                       // statistics (32)
        512;                            // reserved
        // Total: 8 + 1 + 128 + 4 + 36 + 130 + 80 + 48 + 32 + 512 = 979
        // Business logic methods moved to logic/pool_operations.rs
}

// ============================================================================
// FeelsSOL Wrapper
// ============================================================================

/// FeelsSOL wrapper for universal base pair
#[account]
pub struct FeelsSOL {
    pub underlying_mint: Pubkey,        // JitoSOL or other LST
    pub feels_mint: Pubkey,             // FeelsSOL Token-2022 mint
    pub total_wrapped: u128,            // Total LST wrapped
    pub virtual_reserves: u128,         // Virtual balance for AMM
    pub yield_accumulator: u128,        // Accumulated staking yield
    pub last_update_slot: u64,          // Last yield update
    pub authority: Pubkey,              // Protocol authority
}

impl FeelsSOL {
    pub const SIZE: usize = 8 + // discriminator
        32 * 3 + // underlying_mint, feels_mint, authority
        16 * 3 + // wrapped, reserves, yield
        8; // last_update_slot
}

// ============================================================================
// Emergency Controls
// ============================================================================

/// Circuit breaker status for emergency controls
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct CircuitBreakerStatus {
    pub swaps_paused: bool,
    pub deposits_paused: bool,
    pub withdrawals_paused: bool,
    pub fee_collection_paused: bool,
    pub guardian: Option<Pubkey>,
    pub pause_expiry: Option<i64>,
}

// ============================================================================
// Oracle Infrastructure
// ============================================================================

/// Simple observation buffer for TWAP calculations (Phase 1)
#[account]
pub struct ObservationState {
    pub pool: Pubkey,
    pub observations: [Observation; 100],   // Fixed size for Phase 1
    pub observation_index: u16,             // Current position in ring buffer
    pub cardinality: u16,                   // Active observations
    pub last_update_timestamp: i64,
}

impl ObservationState {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        (8 + 16 + 16 + 1) * 100 + // observations array
        2 + 2 + 8; // index, cardinality, timestamp
}

#[derive(Default, Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub struct Observation {
    pub timestamp: i64,
    pub sqrt_price_x96: u128,
    pub cumulative_tick: i128,
    pub initialized: bool,
}


// ============================================================================
// Transient Update System
// ============================================================================

/// Maximum number of tick updates per batch for gas optimization
pub const MAX_TICK_UPDATES: usize = 20;

/// Transient tick updates for gas optimization (Phase 1)
/// Fixed-size array prevents dynamic allocation issues and provides predictable rent costs
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TransientTickUpdates {
    pub pool: Pubkey,
    pub slot: u64,
    pub updates: [TickUpdate; MAX_TICK_UPDATES],
    pub update_count: u8,          // Number of active updates (0-20)
    pub finalized: u8,             // 0 = false, 1 = true
    pub created_at: i64,           // Timestamp for cleanup management
    pub gas_budget_remaining: u32, // For rate limiting
    pub _reserved: [u8; 64],       // Future extensibility
}

impl TransientTickUpdates {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        8 +  // slot
        (32 + 4 + 16 + 32 + 32 + 8 + 1) * MAX_TICK_UPDATES + // updates array
        1 +  // update_count
        1 +  // finalized
        8 +  // created_at
        4 +  // gas_budget_remaining
        64;  // _reserved
        // Total: ~2,500 bytes - well under account limits
    
    /// Initialize a new TransientTickUpdates account
    pub fn initialize(&mut self, pool: Pubkey, slot: u64, timestamp: i64) {
        self.pool = pool;
        self.slot = slot;
        self.update_count = 0;
        self.finalized = 0;
        self.created_at = timestamp;
        self.gas_budget_remaining = 100_000; // Conservative gas budget
        self._reserved = [0u8; 64];
        
        // Initialize all updates to default
        self.updates = core::array::from_fn(|_| TickUpdate::default());
    }
    
    /// Add a tick update to the batch (if space available)
    pub fn add_update(&mut self, update: TickUpdate) -> Result<()> {
        require!(
            self.finalized == 0,
            crate::state::PoolError::InvalidOperation
        );
        
        require!(
            (self.update_count as usize) < MAX_TICK_UPDATES,
            crate::state::PoolError::TransientUpdatesFull
        );
        
        self.updates[self.update_count as usize] = update;
        self.update_count += 1;
        
        Ok(())
    }
    
    /// Remove an update from the batch (compact array)
    pub fn remove_update(&mut self, index: u8) -> Result<()> {
        require!(
            index < self.update_count,
            crate::state::PoolError::InvalidTickIndex
        );
        
        // Shift remaining updates down
        for i in (index as usize)..((self.update_count - 1) as usize) {
            self.updates[i] = self.updates[i + 1];
        }
        
        // Clear the last update and decrement count
        self.updates[(self.update_count - 1) as usize] = TickUpdate::default();
        self.update_count -= 1;
        
        Ok(())
    }
    
    /// Mark all updates as finalized
    pub fn finalize(&mut self) {
        self.finalized = 1;
    }
    
    /// Check if the batch is full
    pub fn is_full(&self) -> bool {
        (self.update_count as usize) >= MAX_TICK_UPDATES
    }
    
    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.update_count == 0
    }
    
    /// Get active updates slice
    pub fn get_active_updates(&self) -> &[TickUpdate] {
        &self.updates[0..(self.update_count as usize)]
    }
    
    /// Clear all updates and reset for reuse
    pub fn reset(&mut self, new_slot: u64, timestamp: i64) {
        self.slot = new_slot;
        self.update_count = 0;
        self.finalized = 0;
        self.created_at = timestamp;
        self.gas_budget_remaining = 100_000;
        
        // Clear all updates
        self.updates = core::array::from_fn(|_| TickUpdate::default());
    }
    
    /// Check if updates should be cleaned up (older than threshold)
    pub fn should_cleanup(&self, current_timestamp: i64, max_age_seconds: i64) -> bool {
        (current_timestamp - self.created_at) > max_age_seconds
    }
}

#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct TickUpdate {
    pub tick_array_pubkey: Pubkey,
    pub tick_index: i32,
    pub liquidity_net_delta: i128,
    pub fee_growth_outside_0: [u64; 4],
    pub fee_growth_outside_1: [u64; 4],
    pub slot: u64,              // When update was generated
    pub priority: u8,           // Urgency hint for keepers
    pub _padding: [u8; 7],      // Explicit padding for alignment
}