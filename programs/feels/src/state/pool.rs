/// Unified Pool state for the Feels Protocol
/// All features integrated directly without phase separation
use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, duration::Duration};
use crate::state::reentrancy::ReentrancyStatus;
use crate::state::leverage::{LeverageParameters, RiskProfile, LeverageStatistics};
use crate::state::fee::DynamicFeeConfig;
use crate::state::metrics_volume::VolumeTracker;
use crate::state::tick::Tick3D;
use crate::utils::U256Wrapper;

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
    pub fee_config: Pubkey,        // Reference to FeeConfig account
    pub fee_rate: u16,             // Current fee rate (basis points)
    pub protocol_fee_rate: u16,    // Protocol fee share (basis points)
    pub liquidity_fee_rate: u16,   // LP fee share
    pub _fee_padding: [u8; 2],     // Alignment
    
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
    
    // ========== Statistics ==========
    pub total_volume_a: u128,      // Cumulative volume token A
    pub total_volume_b: u128,      // Cumulative volume token B
    
    // ========== Security Features ==========
    pub reentrancy_status: u8,     // Reentrancy protection state
    pub _security_padding: [u8; 7], // Alignment
    
    // ========== Oracle Integration ==========
    pub oracle: Pubkey,            // Oracle account (Pubkey::default() if none)
    
    // ========== Leverage System ==========
    pub leverage_params: LeverageParameters,
    pub leverage_stats: LeverageStatistics,
    
    // ========== Dynamic Fees ==========
    pub dynamic_fee_config: DynamicFeeConfig,
    
    // ========== Volume Tracking ==========
    pub volume_tracker: VolumeTracker,
    
    // ========== Position Vault ==========
    pub position_vault: Pubkey,    // Position vault (Pubkey::default() if none)
    
    // ========== Redenomination ==========
    pub last_redenomination: i64,
    pub redenomination_threshold: u64,
    
    // ========== Hook Integration ==========
    pub hook_registry: Pubkey,     // Hook registry account (Pubkey::default() if none)
    pub valence_session: Pubkey,   // Valence hook session (Pubkey::default() if none)
    
    // ========== Reserved Space ==========
    pub _reserved: [u8; 224],      // Reserved for future use (reduced by 32 for hook_registry)
    
    // TODO: When implementing zero-copy optimization, most of these fields
    // would move to compressed accounts. The Pool would store only:
    // - Essential hot state (current_tick, liquidity, sqrt_rate)
    // - References to compressed account trees
    // - Frequently accessed configuration
}

impl Pool {
    pub const SIZE: usize = 8 +    // discriminator
        32 * 4 +                   // token configuration (4 pubkeys)
        32 + 2 + 2 + 2 + 2 +      // fee configuration
        4 + 16 + 16 +             // rate and liquidity
        128 + 2 + 6 +             // tick management
        32 + 32 + 8 + 8 +         // fee tracking
        32 + 8 + 8 +              // metadata
        16 + 16 +                 // statistics
        1 + 7 +                   // security
        32 +                      // oracle
        32 + 32 +                 // leverage (params + stats sizes)
        64 +                      // dynamic fees
        64 +                      // volume tracker
        32 +                      // position vault
        8 + 8 +                   // redenomination
        32 + 32 +                 // hook_registry + valence
        224;                      // reserved
    
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
            FeelsProtocolError::UnauthorizedPoolAuthority
        );
        Ok(())
    }
    
    /// Get vault PDAs for this pool
    pub fn get_vault_pdas(&self, program_id: &Pubkey) -> (Pubkey, u8, Pubkey, u8) {
        let (vault_a, bump_a) = Pubkey::find_program_address(
            &[b"token_vault", self.key().as_ref(), self.token_a_mint.as_ref()],
            program_id,
        );
        
        let (vault_b, bump_b) = Pubkey::find_program_address(
            &[b"token_vault", self.key().as_ref(), self.token_b_mint.as_ref()],
            program_id,
        );
        
        (vault_a, bump_a, vault_b, bump_b)
    }
    
    /// Calculate current 3D tick position
    pub fn get_current_tick_3d(&self) -> Result<Tick3D> {
        Ok(Tick3D::new(
            self.current_tick,
            Duration::Swap.to_tick(),
            0, // Default 1x leverage
        )?)
    }
    
    // ========================================================================
    // Security Functions
    // ========================================================================
    
    /// Get reentrancy status
    pub fn get_reentrancy_status(&self) -> Result<ReentrancyStatus> {
        ReentrancyStatus::try_from(self.reentrancy_status)
            .map_err(|_| FeelsProtocolError::InvalidValue.into())
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
    
    /// Check if valence session is active
    pub fn has_valence_session(&self) -> bool {
        self.valence_session != Pubkey::default()
    }
    
    /// Check if hook registry is configured
    pub fn has_hook_registry(&self) -> bool {
        self.hook_registry != Pubkey::default()
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
            .checked_mul(U256Wrapper::from_u64(1u64 << 32))?
            .checked_div(U256Wrapper::from_u128(self.liquidity))?;
        
        if is_token_a {
            let current = U256Wrapper::from_u64_array(self.fee_growth_global_a);
            let new = current.checked_add(fee_growth)?;
            self.fee_growth_global_a = new.to_u64_array();
        } else {
            let current = U256Wrapper::from_u64_array(self.fee_growth_global_b);
            let new = current.checked_add(fee_growth)?;
            self.fee_growth_global_b = new.to_u64_array();
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Leverage Statistics
    // ========================================================================
    
    /// Update leverage statistics when adding liquidity
    pub fn update_leverage_stats_add(
        &mut self,
        liquidity: u128,
        leverage: u64,
        effective_liquidity: u128,
    ) -> Result<()> {
        self.leverage_stats.total_base_liquidity = self.leverage_stats
            .total_base_liquidity
            .checked_add(liquidity)
            .ok_or(FeelsProtocolError::MathOverflow)?;
            
        self.leverage_stats.total_leveraged_liquidity = self.leverage_stats
            .total_leveraged_liquidity
            .checked_add(effective_liquidity)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        if leverage > RiskProfile::LEVERAGE_SCALE {
            self.leverage_stats.leveraged_position_count = self.leverage_stats
                .leveraged_position_count
                .checked_add(1)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }
        
        self.leverage_stats.last_update = Clock::get()?.unix_timestamp;
        Ok(())
    }
    
    /// Update leverage statistics when removing liquidity
    pub fn update_leverage_stats_remove(
        &mut self,
        liquidity: u128,
        leverage: u64,
        effective_liquidity: u128,
    ) -> Result<()> {
        self.leverage_stats.total_base_liquidity = self.leverage_stats
            .total_base_liquidity
            .checked_sub(liquidity)
            .ok_or(FeelsProtocolError::MathUnderflow)?;
            
        self.leverage_stats.total_leveraged_liquidity = self.leverage_stats
            .total_leveraged_liquidity
            .checked_sub(effective_liquidity)
            .ok_or(FeelsProtocolError::MathUnderflow)?;
        
        if leverage > RiskProfile::LEVERAGE_SCALE && 
           self.leverage_stats.leveraged_position_count > 0 {
            self.leverage_stats.leveraged_position_count -= 1;
        }
        
        self.leverage_stats.last_update = Clock::get()?.unix_timestamp;
        Ok(())
    }
    
    // ========================================================================
    // Volume Tracking
    // ========================================================================
    
    /// Update trading volume
    pub fn update_volume(&mut self, amount_a: u64, amount_b: u64) -> Result<()> {
        self.total_volume_a = self.total_volume_a
            .checked_add(amount_a as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
            
        self.total_volume_b = self.total_volume_b
            .checked_add(amount_b as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
            
        let current_timestamp = Clock::get()?.unix_timestamp;
        self.volume_tracker.update_volume(amount_a, amount_b, current_timestamp)?;
        
        Ok(())
    }
    
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
        
        require!(word_pos < 16, FeelsProtocolError::InvalidTick);
        
        if initialized {
            self.tick_array_bitmap[word_pos] |= 1u64 << bit_pos;
        } else {
            self.tick_array_bitmap[word_pos] &= !(1u64 << bit_pos);
        }
        
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

impl AccountDeserialize for Pool {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        let pool: &Pool = bytemuck::try_from_bytes(buf)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        Ok(*pool)
    }
    
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize(buf)
    }
}

impl anchor_lang::Owner for Pool {
    fn owner() -> Pubkey {
        crate::ID
    }
}

/// Helper trait for Pool key operations
pub trait PoolKey {
    fn key(&self) -> Pubkey;
}

impl<'info> PoolKey for AccountLoader<'info, Pool> {
    fn key(&self) -> Pubkey {
        AccountLoader::key(self)
    }
}