/// Unified Pool state for the Feels Protocol
/// All features integrated directly without phase separation
use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, duration::Duration};
use crate::state::reentrancy::ReentrancyStatus;
use crate::state::leverage::{LeverageParameters, RiskProfile};
use crate::state::tick::Tick3D;
use crate::utils::U256Wrapper;
use crate::constant::TICK_ARRAY_SIZE;

// ============================================================================
// Unified Pool Structure
// ============================================================================

/// Unified Pool account with all features integrated
#[account(zero_copy)]
#[repr(C, packed)]
pub struct Pool {
    pub _discriminator: [u8; 8], // Account discriminator
    
    // ========== Core Token Configuration ==========
    pub token_a_mint: Pubkey,      // Any token mint
    pub token_b_mint: Pubkey,      // Always FeelsSOL mint
    pub token_a_vault: Pubkey,     // Token A vault PDA
    pub token_b_vault: Pubkey,     // FeelsSOL vault PDA
    
    // ========== Fee Configuration ==========
    pub fee_config: Pubkey,        // Reference to FeeConfig account (single source of truth)
    pub fee_rate: u16,             // IMMUTABLE: Only for PDA derivation compatibility
    pub _fee_padding: [u8; 6],     // Alignment
    
    // ========== Rate and Liquidity State ==========
    pub current_tick: i32,         // Current rate tick
    pub current_sqrt_rate: u128,   // Square root of rate (Q64.96)
    pub liquidity: u128,           // Total active liquidity
    
    // ========== Tick Management ==========
    pub tick_array_bitmap: [u64; 16], // 1024-bit bitmap for tick arrays
    pub tick_spacing: i16,            // Minimum tick spacing
    pub _tick_padding: [u8; 6],       // Alignment
    
    // ========== Fee Tracking ==========
    pub fee_growth_global_a: [u64; 4], // Cumulative fees token A (u256)
    pub fee_growth_global_b: [u64; 4], // Cumulative fees token B (u256)
    pub protocol_fees_a: u64,          // Uncollected protocol fees A
    pub protocol_fees_b: u64,          // Uncollected protocol fees B
    
    // ========== Pool Metadata ==========
    pub authority: Pubkey,         // Pool authority
    pub creation_timestamp: i64,   // Creation time
    pub last_update_slot: u64,     // Last update slot
    
    // ========== References to Separate Accounts ==========
    pub pool_metrics: Pubkey,      // Reference to PoolMetrics account
    pub pool_hooks: Pubkey,        // Reference to PoolHooks account
    pub pool_rebase: Pubkey,       // Reference to PoolRebase account
    pub market_field: Pubkey,      // Reference to MarketField account
    
    // ========== Security Features ==========
    pub reentrancy_status: u8,     // Reentrancy protection state
    pub _security_padding: [u8; 7], // Alignment
    
    // ========== Oracle Integration ==========
    pub oracle: Pubkey,            // Oracle account (Pubkey::default() if none)
    
    // ========== Leverage System ==========
    pub leverage_params: LeverageParameters,  // Hot path params stay
    // leverage_stats moved to PoolMetrics
    
    // Volume tracking moved to PoolMetrics
    
    // ========== Position Vault ==========
    pub position_vault: Pubkey,    // Position vault (Pubkey::default() if none)
    
    // Redenomination moved to PoolRebase
    
    // Hook integration moved to PoolHooks
    
    // Virtual rebasing moved to PoolRebase
    
    // ========== Reserved Space ==========
    pub _reserved: [u8; 64],       // Reserved for future use (reduced by 64)
    pub _reserved2: [u8; 64],      // Additional reserved space
    pub _reserved3: [u8; 32],      // More reserved space
    
    // TODO: When implementing zero-copy optimization, most of these fields
    // would move to compressed accounts. The Pool would store only:
    // - Essential hot state (current_tick, liquidity, sqrt_rate)
    // - References to compressed account trees
    // - Frequently accessed configuration
}

impl Pool {
    pub const SIZE: usize = 8 +    // discriminator
        32 * 4 +                   // token configuration (4 pubkeys)
        32 + 2 + 6 +              // fee configuration (removed duplicate padding)
        4 + 16 + 16 +             // rate and liquidity
        128 + 2 + 6 +             // tick management
        32 + 32 + 8 + 8 +         // fee tracking
        32 + 8 + 8 +              // metadata
        32 + 32 + 32 + 32 +       // references to separate accounts (pool_metrics, pool_hooks, pool_rebase, market_field)
        1 + 7 +                   // security
        32 +                      // oracle
        32 +                      // leverage params only (stats moved)
        32 +                      // position vault
        96 + 64 + 32;             // reserved (_reserved + _reserved2 + _reserved3)
    
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
    
    /// Calculate current 3D tick position
    pub fn get_current_tick_3d(&self) -> Result<Tick3D> {
        Ok(Tick3D {
            rate_tick: self.current_tick,
            duration_tick: Duration::Swap.to_tick(),
            leverage_tick: 0, // Default 1x leverage
        })
    }
    
    // ========================================================================
    // Security Functions
    // ========================================================================
    
    /// Get reentrancy status
    pub fn get_reentrancy_status(&self) -> Result<ReentrancyStatus> {
        ReentrancyStatus::try_from(self.reentrancy_status)
            .map_err(|_| FeelsProtocolError::InvalidAmount.into())
    }
    
    /// Set reentrancy status
    pub fn set_reentrancy_status(&mut self, status: ReentrancyStatus) -> Result<()> {
        self.reentrancy_status = status as u8;
        Ok(())
    }
    
    // ========================================================================
    // Feature Checks
    // ========================================================================
    
    /// Check if oracle is configured
    pub fn has_oracle(&self) -> bool {
        self.oracle != Pubkey::default()
    }
    
    /// Check if position vault is configured
    pub fn has_position_vault(&self) -> bool {
        self.position_vault != Pubkey::default()
    }
    
    /// Check if pool hooks are configured
    pub fn has_pool_hooks(&self) -> bool {
        self.pool_hooks != Pubkey::default()
    }
    
    /// Check if pool metrics are configured
    pub fn has_pool_metrics(&self) -> bool {
        self.pool_metrics != Pubkey::default()
    }
    
    /// Check if pool rebase is configured
    pub fn has_pool_rebase(&self) -> bool {
        self.pool_rebase != Pubkey::default()
    }
    
    /// Get maximum allowed leverage
    pub fn get_max_leverage(&self) -> Result<u64> {
        Ok(self.leverage_params.max_leverage)
    }
    
    // ========================================================================
    // Fee Management
    // ========================================================================
    
    /// Accumulate protocol fees
    pub fn accumulate_protocol_fees(&mut self, fee_amount: u64, is_token_a: bool) -> Result<()> {
        if is_token_a {
            self.protocol_fees_a = self.protocol_fees_a
                .checked_add(fee_amount)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            self.protocol_fees_b = self.protocol_fees_b
                .checked_add(fee_amount)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }
        Ok(())
    }
    
    /// Accumulate fee growth
    pub fn accumulate_fee_growth(&mut self, fee_amount: u64, is_token_a: bool) -> Result<()> {
        if self.liquidity == 0 {
            return Ok(());
        }
        
        let fee_growth = U256Wrapper::from_u64(fee_amount)
            .checked_mul(U256Wrapper::from_u64(1u64 << 32))
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(U256Wrapper::from_u128(self.liquidity))
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        if is_token_a {
            let current = U256Wrapper::from_u64_array(self.fee_growth_global_a);
            let new = current.checked_add(fee_growth)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            self.fee_growth_global_a = new.as_u64_array();
        } else {
            let current = U256Wrapper::from_u64_array(self.fee_growth_global_b);
            let new = current.checked_add(fee_growth)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            self.fee_growth_global_b = new.as_u64_array();
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Reference Account Helpers
    // ========================================================================
    
    /// Initialize reference accounts (called during pool creation)
    pub fn initialize_references(
        &mut self,
        pool_metrics: Pubkey,
        pool_hooks: Pubkey,
        pool_rebase: Pubkey,
    ) -> Result<()> {
        self.pool_metrics = pool_metrics;
        self.pool_hooks = pool_hooks;
        self.pool_rebase = pool_rebase;
        Ok(())
    }
    
    // Leverage statistics methods moved to PoolMetrics
    
    // Volume tracking methods moved to PoolMetrics
    
    // ========================================================================
    // Bitmap Operations
    // ========================================================================
    
    /// Check if a tick has liquidity
    pub fn is_tick_initialized(&self, tick: i32) -> bool {
        let word_pos = (tick / 64) as usize;
        let bit_pos = (tick % 64) as u8;
        
        if word_pos >= 16 {
            return false;
        }
        
        (self.tick_array_bitmap[word_pos] & (1u64 << bit_pos)) != 0
    }
    
    /// Set tick initialization state
    pub fn set_tick_initialized(&mut self, tick: i32, initialized: bool) -> Result<()> {
        let word_pos = (tick / 64) as usize;
        let bit_pos = (tick % 64) as u8;
        
        require!(word_pos < 16, FeelsProtocolError::InvalidTickIndex);
        
        if initialized {
            self.tick_array_bitmap[word_pos] |= 1u64 << bit_pos;
        } else {
            self.tick_array_bitmap[word_pos] &= !(1u64 << bit_pos);
        }
        
        Ok(())
    }
    
    /// Check if a specific tick array is initialized
    pub fn is_tick_array_initialized(&self, start_tick_index: i32) -> bool {
        // Calculate which bit in the bitmap represents this tick array
        let array_index = start_tick_index / (TICK_ARRAY_SIZE as i32 * self.tick_spacing as i32);
        if array_index < 0 || array_index >= 1024 {
            return false;
        }
        
        let word_pos = (array_index / 64) as usize;
        let bit_pos = (array_index % 64) as u8;
        
        if word_pos >= 16 {
            return false;
        }
        
        (self.tick_array_bitmap[word_pos] & (1u64 << bit_pos)) != 0
    }
    
    /// Update fee growth tracking
    pub fn update_fee_growth(&mut self, fee_growth_a: U256Wrapper, fee_growth_b: U256Wrapper) -> Result<()> {
        // Update global fee growth
        self.fee_growth_global_a = fee_growth_a.as_u64_array();
        self.fee_growth_global_b = fee_growth_b.as_u64_array();
        Ok(())
    }
    
    /// Check if phase 2 features are enabled
    pub fn is_phase2_enabled(&self) -> bool {
        // Phase 2 is enabled if leverage parameters allow leverage > 1x
        self.leverage_params.max_leverage > RiskProfile::LEVERAGE_SCALE
    }
    
    /// Update volume tracker
    /// Note: Volume tracking has been moved to PoolMetrics account
    /// This method is deprecated and will be removed
    pub fn update_volume_tracker(&mut self, _amount_a: u64, _amount_b: u64) -> Result<()> {
        // Volume tracking moved to PoolMetrics
        // This method kept for compatibility but does nothing
        Ok(())
    }
}

// ============================================================================
// Account Key Extension
// ============================================================================

impl AsRef<[u8]> for Pool {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Pool as *const u8,
                std::mem::size_of::<Pool>(),
            )
        }
    }
}

impl AccountSerialize for Pool {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.as_ref())?;
        Ok(())
    }
}

// Note: AccountDeserialize and Owner traits are automatically implemented by #[account(zero_copy)]

/// Helper trait for Pool key operations
pub trait PoolKey {
    fn key(&self) -> Pubkey;
}

impl<'info> PoolKey for AccountLoader<'info, Pool> {
    fn key(&self) -> Pubkey {
        self.to_account_info().key()
    }
}