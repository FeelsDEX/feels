/// Simplified Pool state that merges ancillary accounts for reduced complexity.
/// This reduces the number of accounts per instruction while maintaining hot/cold data separation.
use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, duration::Duration};
use crate::state::reentrancy::ReentrancyStatus;
use crate::state::leverage::LeverageParameters;
use crate::utils::U256Wrapper;

// ============================================================================
// Simplified Pool Structure
// ============================================================================

/// Simplified Pool account with merged ancillary data
#[account(zero_copy)]
#[repr(C, packed)]
pub struct PoolSimplified {
    pub _discriminator: [u8; 8], // Account discriminator
    
    // ========== Core Token Configuration ==========
    pub token_a_mint: Pubkey,      // Any token mint
    pub token_b_mint: Pubkey,      // Always FeelsSOL mint
    pub token_a_vault: Pubkey,     // Token A vault PDA
    pub token_b_vault: Pubkey,     // FeelsSOL vault PDA
    
    // ========== Fee Configuration ==========
    pub fee_config: Pubkey,        // Reference to FeeConfig account
    pub fee_rate: u16,             // IMMUTABLE: Only for PDA derivation
    pub _fee_padding: [u8; 6],     // Alignment
    
    // ========== Rate and Liquidity State (HOT) ==========
    pub current_tick: i32,         // Current rate tick
    pub current_sqrt_rate: u128,   // Square root of rate (Q64.96)
    pub liquidity: u128,           // Total active liquidity
    
    // ========== Tick Management (HOT) ==========
    pub tick_array_bitmap: [u64; 16], // 1024-bit bitmap for tick arrays
    pub tick_spacing: i16,            // Minimum tick spacing
    pub _tick_padding: [u8; 6],       // Alignment
    
    // ========== Fee Tracking (HOT) ==========
    pub fee_growth_global_a: [u64; 4], // Cumulative fees token A (u256)
    pub fee_growth_global_b: [u64; 4], // Cumulative fees token B (u256)
    pub protocol_fees_a: u64,          // Uncollected protocol fees A
    pub protocol_fees_b: u64,          // Uncollected protocol fees B
    
    // ========== Pool Metadata ==========
    pub authority: Pubkey,         // Pool authority
    pub creation_timestamp: i64,   // Creation time
    pub last_update_slot: u64,     // Last update slot
    
    // ========== Security Features ==========
    pub reentrancy_status: u8,     // Reentrancy protection state
    pub _security_padding: [u8; 7], // Alignment
    
    // ========== Oracle Integration ==========
    pub oracle: Pubkey,            // Oracle account (Pubkey::default() if none)
    
    // ========== Leverage System ==========
    pub leverage_params: LeverageParameters,  // Hot path params
    
    // ========== Position Vault ==========
    pub position_vault: Pubkey,    // Position vault (Pubkey::default() if none)
    
    // ========== Merged Hook Configuration (was PoolHooks) ==========
    pub hook_registry: Pubkey,     // Hook registry account (Pubkey::default() if none)
    pub hooks_enabled: bool,       // Hook enable flag
    pub _hook_padding: [u8; 7],    // Alignment
    
    // ========== Merged Rebase Configuration (was PoolRebase) ==========
    pub rebase_accumulator: Pubkey,    // Rebase accumulator account
    pub last_redenomination: i64,      // Last redenomination timestamp
    pub redenomination_threshold: u64, // Redenomination threshold
    pub last_rebase_timestamp: i64,    // Last rebase timestamp
    pub rebase_epoch_duration: i64,    // Rebase epoch duration (seconds)
    
    // ========== Reference to Metrics (COLD) ==========
    pub pool_metrics: Pubkey,      // Reference to consolidated PoolMetrics account
    
    // ========== Creator for Governance ==========
    pub pool_creator: Pubkey,      // Original pool creator (for oracle updates)
    
    // ========== Reserved Space ==========
    pub _reserved: [u8; 64],       // Reserved for future use
}

impl PoolSimplified {
    pub const SIZE: usize = 8 +    // discriminator
        32 * 4 +                   // token configuration (4 pubkeys)
        32 + 2 + 6 +              // fee configuration
        4 + 16 + 16 +             // rate and liquidity
        128 + 2 + 6 +             // tick management
        32 + 32 + 8 + 8 +         // fee tracking
        32 + 8 + 8 +              // metadata
        1 + 7 +                   // security
        32 +                      // oracle
        32 +                      // leverage params
        32 +                      // position vault
        32 + 1 + 7 +              // hook configuration (merged)
        32 + 8 + 8 + 8 + 8 +      // rebase configuration (merged)
        32 +                      // pool_metrics reference
        32 +                      // pool_creator
        64;                       // reserved
    
    // ========================================================================
    // Core Pool Functions
    // ========================================================================
    
    /// Get pool seeds for PDA derivation
    pub fn seeds(token_a: &Pubkey, token_b: &Pubkey, fee_rate: u16) -> Vec<Vec<u8>> {
        let (token_a_sorted, token_b_sorted) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        
        vec![
            b"pool".to_vec(),
            token_a_sorted.to_bytes().to_vec(),
            token_b_sorted.to_bytes().to_vec(),
            fee_rate.to_le_bytes().to_vec(),
        ]
    }
    
    /// Verify pool authority
    pub fn verify_authority(&self, signer: &Pubkey) -> Result<()> {
        require!(
            self.authority == *signer,
            FeelsProtocolError::InvalidAuthority
        );
        Ok(())
    }
    
    /// Verify pool creator (for oracle updates)
    pub fn verify_creator(&self, signer: &Pubkey) -> Result<()> {
        require!(
            self.pool_creator == *signer,
            FeelsProtocolError::InvalidAuthority
        );
        Ok(())
    }
    
    /// Get vault PDAs for this pool
    pub fn get_vault_pdas(&self, pool_key: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8, Pubkey, u8) {
        let (vault_a, bump_a) = Pubkey::find_program_address(
            &[b"token_vault", pool_key.as_ref(), self.token_a_mint.as_ref()],
            program_id,
        );
        
        let (vault_b, bump_b) = Pubkey::find_program_address(
            &[b"token_vault", pool_key.as_ref(), self.token_b_mint.as_ref()],
            program_id,
        );
        
        (vault_a, bump_a, vault_b, bump_b)
    }
    
    // ========================================================================
    // Hook Functions (merged from PoolHooks)
    // ========================================================================
    
    /// Check if hook registry is configured
    pub fn has_hook_registry(&self) -> bool {
        self.hook_registry != Pubkey::default()
    }
    
    /// Update hook registry
    pub fn set_hook_registry(&mut self, registry: Pubkey) -> Result<()> {
        self.hook_registry = registry;
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }
    
    /// Enable or disable hooks
    pub fn set_hooks_enabled(&mut self, enabled: bool) -> Result<()> {
        self.hooks_enabled = enabled;
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }
    
    // ========================================================================
    // Rebase Functions (merged from PoolRebase)
    // ========================================================================
    
    /// Check if rebase accumulator is configured
    pub fn has_rebase_accumulator(&self) -> bool {
        self.rebase_accumulator != Pubkey::default()
    }
    
    /// Update rebase accumulator
    pub fn set_rebase_accumulator(&mut self, accumulator: Pubkey) -> Result<()> {
        self.rebase_accumulator = accumulator;
        Ok(())
    }
    
    /// Check if redenomination is needed
    pub fn needs_redenomination(&self, current_timestamp: i64, threshold_reached: bool) -> bool {
        let time_elapsed = current_timestamp.saturating_sub(self.last_redenomination);
        let min_interval = 86400; // 24 hours minimum between redenominations
        
        time_elapsed >= min_interval && threshold_reached
    }
    
    /// Update redenomination timestamp
    pub fn update_redenomination(&mut self) -> Result<()> {
        self.last_redenomination = Clock::get()?.unix_timestamp;
        Ok(())
    }
    
    /// Check if rebase epoch has passed
    pub fn is_rebase_due(&self, current_timestamp: i64) -> bool {
        let time_elapsed = current_timestamp.saturating_sub(self.last_rebase_timestamp);
        time_elapsed >= self.rebase_epoch_duration
    }
    
    /// Update rebase timestamp
    pub fn update_rebase_timestamp(&mut self) -> Result<()> {
        self.last_rebase_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
    
    // ========================================================================
    // Tick Array Management
    // ========================================================================
    
    /// Check if a tick array is initialized from bitmap
    pub fn is_tick_array_initialized(&self, tick_array_start: i32) -> bool {
        // Calculate bitmap index
        let array_index = (tick_array_start / (self.tick_spacing as i32 * TICK_ARRAY_SIZE as i32)) as i64;
        let bitmap_index = ((array_index >> 6) & 15) as usize;
        let bit_position = (array_index & 63) as u64;
        
        if bitmap_index >= 16 {
            return false;
        }
        
        let mask = 1u64 << bit_position;
        (self.tick_array_bitmap[bitmap_index] & mask) != 0
    }
    
    /// Update tick array bitmap
    pub fn update_tick_array_bitmap(&mut self, tick_array_start: i32, initialized: bool) -> Result<()> {
        let array_index = (tick_array_start / (self.tick_spacing as i32 * TICK_ARRAY_SIZE as i32)) as i64;
        let bitmap_index = ((array_index >> 6) & 15) as usize;
        let bit_position = (array_index & 63) as u64;
        
        require!(
            bitmap_index < 16,
            FeelsProtocolError::InvalidTickArrayIndex
        );
        
        let mask = 1u64 << bit_position;
        
        if initialized {
            self.tick_array_bitmap[bitmap_index] |= mask;
        } else {
            self.tick_array_bitmap[bitmap_index] &= !mask;
        }
        
        Ok(())
    }
}

// ============================================================================
// Migration Helper
// ============================================================================

/// Helper to migrate from old structure to simplified
pub fn migrate_to_simplified(
    old_pool: &crate::state::Pool,
    old_hooks: Option<&crate::state::PoolHooks>,
    old_rebase: Option<&crate::state::PoolRebase>,
) -> PoolSimplified {
    let mut simplified = PoolSimplified::default();
    
    // Copy core fields
    simplified._discriminator = old_pool._discriminator;
    simplified.token_a_mint = old_pool.token_a_mint;
    simplified.token_b_mint = old_pool.token_b_mint;
    simplified.token_a_vault = old_pool.token_a_vault;
    simplified.token_b_vault = old_pool.token_b_vault;
    simplified.fee_config = old_pool.fee_config;
    simplified.fee_rate = old_pool.fee_rate;
    simplified.current_tick = old_pool.current_tick;
    simplified.current_sqrt_rate = old_pool.current_sqrt_rate;
    simplified.liquidity = old_pool.liquidity;
    simplified.tick_array_bitmap = old_pool.tick_array_bitmap;
    simplified.tick_spacing = old_pool.tick_spacing;
    simplified.fee_growth_global_a = old_pool.fee_growth_global_a;
    simplified.fee_growth_global_b = old_pool.fee_growth_global_b;
    simplified.protocol_fees_a = old_pool.protocol_fees_a;
    simplified.protocol_fees_b = old_pool.protocol_fees_b;
    simplified.authority = old_pool.authority;
    simplified.creation_timestamp = old_pool.creation_timestamp;
    simplified.last_update_slot = old_pool.last_update_slot;
    simplified.reentrancy_status = old_pool.reentrancy_status;
    simplified.oracle = old_pool.oracle;
    simplified.leverage_params = old_pool.leverage_params;
    simplified.position_vault = old_pool.position_vault;
    simplified.pool_metrics = old_pool.pool_metrics;
    simplified.pool_creator = old_pool.authority; // Use authority as creator initially
    
    // Merge hook configuration if available
    if let Some(hooks) = old_hooks {
        simplified.hook_registry = hooks.hook_registry;
        simplified.hooks_enabled = hooks.hooks_enabled;
    }
    
    // Merge rebase configuration if available
    if let Some(rebase) = old_rebase {
        simplified.rebase_accumulator = rebase.rebase_accumulator;
        simplified.last_redenomination = rebase.last_redenomination;
        simplified.redenomination_threshold = rebase.redenomination_threshold;
        simplified.last_rebase_timestamp = rebase.last_rebase_timestamp;
        simplified.rebase_epoch_duration = rebase.rebase_epoch_duration;
    }
    
    simplified
}

// ============================================================================
// Benefits of Simplification
// ============================================================================

// 1. Reduced Accounts: From 4 accounts (Pool + PoolHooks + PoolRebase + PoolMetrics)
//    to 2 accounts (PoolSimplified + PoolMetricsConsolidated)
//
// 2. Lower Transaction Costs: Fewer accounts = lower transaction fees
//
// 3. Simpler Client Integration: Less accounts to pass in instructions
//
// 4. Maintained Performance: Hot data still in main Pool, cold data in metrics
//
// 5. Easier Initialization: Single pool creation instead of multiple accounts