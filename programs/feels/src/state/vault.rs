/// Position vault system for automated liquidity management enabling protocol-owned
/// liquidity strategies and user deposit aggregation with multiple share classes.
/// 
/// The vault accepts FeelsSOL and Feels token deposits from users and manages them
/// alongside protocol-owned liquidity. Users receive position vault shares that
/// represent different combinations of leverage, duration, and exposure preferences.
use anchor_lang::prelude::*;
use crate::state::FeelsProtocolError;

// ============================================================================
// Share Type System
// ============================================================================

/// Position vault share types representing different user preferences
/// Each share type is a combination of leverage, duration, and exposure
#[derive(Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum ShareType {
    // FeelsSOL exposure shares
    FeelsSolSenior,           // Low leverage, no duration lock
    FeelsSolSeniorLocked,     // Low leverage, 1 month lock
    FeelsSolJunior,           // High leverage, no duration lock
    FeelsSolJuniorLocked,     // High leverage, 1 month lock
    
    // Feels token exposure shares
    FeelsSenior,              // Low leverage, no duration lock
    FeelsSeniorLocked,        // Low leverage, 1 month lock
    FeelsJunior,              // High leverage, no duration lock
    FeelsJuniorLocked,        // High leverage, 1 month lock
}

impl ShareType {
    /// Get leverage multiplier for this share type (scaled by 1e6)
    pub fn leverage(&self) -> u64 {
        match self {
            // Senior shares have 1.5x leverage
            ShareType::FeelsSolSenior | ShareType::FeelsSolSeniorLocked |
            ShareType::FeelsSenior | ShareType::FeelsSeniorLocked => 1_500_000,
            
            // Junior shares have 3x leverage
            ShareType::FeelsSolJunior | ShareType::FeelsSolJuniorLocked |
            ShareType::FeelsJunior | ShareType::FeelsJuniorLocked => 3_000_000,
        }
    }
    
    /// Check if this share type has a duration lock
    pub fn has_lock(&self) -> bool {
        matches!(self, 
            ShareType::FeelsSolSeniorLocked | ShareType::FeelsSolJuniorLocked |
            ShareType::FeelsSeniorLocked | ShareType::FeelsJuniorLocked
        )
    }
    
    /// Get lock duration in slots (1 month = ~3,024,000 slots at 2 slots/sec)
    pub fn lock_duration(&self) -> u64 {
        if self.has_lock() {
            3_024_000 // 30 days * 24 hours * 60 minutes * 60 seconds * 2 slots/sec
        } else {
            0
        }
    }
    
    /// Check if this share type is for FeelsSOL exposure
    pub fn is_feelssol(&self) -> bool {
        matches!(self,
            ShareType::FeelsSolSenior | ShareType::FeelsSolSeniorLocked |
            ShareType::FeelsSolJunior | ShareType::FeelsSolJuniorLocked
        )
    }
    
    /// Get protection level (inverse of risk)
    pub fn protection_level(&self) -> u64 {
        match self {
            // Senior shares have higher protection
            ShareType::FeelsSolSenior | ShareType::FeelsSolSeniorLocked |
            ShareType::FeelsSenior | ShareType::FeelsSeniorLocked => 700_000, // 70%
            
            // Junior shares have lower protection
            ShareType::FeelsSolJunior | ShareType::FeelsSolJuniorLocked |
            ShareType::FeelsJunior | ShareType::FeelsJuniorLocked => 450_000, // 45%
        }
    }
}

/// User's position in the vault
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct VaultPosition {
    /// Share type held
    pub share_type: ShareType,
    /// Number of shares
    pub shares: u128,
    /// Initial deposit amount
    pub deposited_amount: u64,
    /// Deposit timestamp
    pub deposit_timestamp: i64,
    /// Lock expiry (0 if no lock)
    pub lock_expiry: i64,
    /// Last claim timestamp for rewards
    pub last_reward_claim: i64,
}

/// Share class accounting within the vault
#[derive(Clone, Debug, Default, AnchorSerialize, AnchorDeserialize)]
pub struct ShareClass {
    /// Total shares outstanding
    pub total_shares: u128,
    /// Total underlying value (FeelsSOL or Feels)
    pub total_value: u128,
    /// Accumulated fees per share (Q128.128)
    pub accumulated_fees_per_share: [u64; 4],
    /// Last update timestamp
    pub last_update: i64,
}

// ============================================================================
// Position Vault Account
// ============================================================================

/// Position vault for automated liquidity management
#[account]
pub struct PositionVault {
    /// Associated pool
    pub pool: Pubkey,
    
    /// Vault authority
    pub authority: Pubkey,
    
    /// Token accounts
    pub vault_feelssol: Pubkey,        // Vault's FeelsSOL account
    pub vault_feels: Pubkey,           // Vault's Feels token account
    
    /// Protocol-owned liquidity
    pub protocol_owned_liquidity: u128,
    
    /// Share classes (8 total: 2 leverage × 2 duration × 2 exposure)
    pub feelssol_senior: ShareClass,
    pub feelssol_senior_locked: ShareClass,
    pub feelssol_junior: ShareClass,
    pub feelssol_junior_locked: ShareClass,
    pub feels_senior: ShareClass,
    pub feels_senior_locked: ShareClass,
    pub feels_junior: ShareClass,
    pub feels_junior_locked: ShareClass,
    
    /// Total liquidity deployed to pool
    pub total_deployed_liquidity: u128,
    
    /// Rebalance configuration
    pub rebalance_config: RebalanceConfig,
    
    /// Last rebalance slot
    pub last_rebalance_slot: u64,
    
    /// Performance metrics
    pub total_fees_earned: u128,
    pub performance_fee_rate: u16,     // Basis points
    pub management_fee_rate: u16,      // Basis points
    
    /// Active positions (limited for account size)
    pub active_positions: [Option<ManagedPosition>; 10],
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

impl PositionVault {
    pub const SIZE: usize = 8 + // discriminator
        32 + 32 + // pool, authority
        32 + 32 + // token accounts
        16 + // protocol liquidity
        (16 + 16 + 32 + 8) * 8 + // 8 share classes
        16 + // total deployed
        20 + // rebalance config
        8 + // last rebalance
        16 + 2 + 2 + // performance metrics
        (1 + 96) * 10 + // active positions
        128; // reserved
        
    /// Get share class by type
    pub fn get_share_class(&self, share_type: ShareType) -> &ShareClass {
        match share_type {
            ShareType::FeelsSolSenior => &self.feelssol_senior,
            ShareType::FeelsSolSeniorLocked => &self.feelssol_senior_locked,
            ShareType::FeelsSolJunior => &self.feelssol_junior,
            ShareType::FeelsSolJuniorLocked => &self.feelssol_junior_locked,
            ShareType::FeelsSenior => &self.feels_senior,
            ShareType::FeelsSeniorLocked => &self.feels_senior_locked,
            ShareType::FeelsJunior => &self.feels_junior,
            ShareType::FeelsJuniorLocked => &self.feels_junior_locked,
        }
    }
    
    /// Get mutable share class by type
    pub fn get_share_class_mut(&mut self, share_type: ShareType) -> &mut ShareClass {
        match share_type {
            ShareType::FeelsSolSenior => &mut self.feelssol_senior,
            ShareType::FeelsSolSeniorLocked => &mut self.feelssol_senior_locked,
            ShareType::FeelsSolJunior => &mut self.feelssol_junior,
            ShareType::FeelsSolJuniorLocked => &mut self.feelssol_junior_locked,
            ShareType::FeelsSenior => &mut self.feels_senior,
            ShareType::FeelsSeniorLocked => &mut self.feels_senior_locked,
            ShareType::FeelsJunior => &mut self.feels_junior,
            ShareType::FeelsJuniorLocked => &mut self.feels_junior_locked,
        }
    }
    
    /// Calculate total vault value across all share classes
    pub fn total_value(&self) -> u128 {
        self.feelssol_senior.total_value +
        self.feelssol_senior_locked.total_value +
        self.feelssol_junior.total_value +
        self.feelssol_junior_locked.total_value +
        self.feels_senior.total_value +
        self.feels_senior_locked.total_value +
        self.feels_junior.total_value +
        self.feels_junior_locked.total_value
    }
    
    /// Calculate shares to mint for a deposit
    pub fn calculate_shares_to_mint(
        &self,
        share_type: ShareType,
        deposit_amount: u64,
    ) -> Result<u128> {
        let share_class = self.get_share_class(share_type);
        
        if share_class.total_shares == 0 {
            // First deposit, 1:1 ratio
            Ok(deposit_amount as u128)
        } else {
            // Proportional to existing shares
            let shares = (deposit_amount as u128)
                .checked_mul(share_class.total_shares)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(share_class.total_value)
                .ok_or(FeelsProtocolError::DivisionByZero)?;
            Ok(shares)
        }
    }
    
    /// Calculate redemption value for shares
    pub fn calculate_redemption_value(
        &self,
        share_type: ShareType,
        shares: u128,
    ) -> Result<u64> {
        let share_class = self.get_share_class(share_type);
        
        if share_class.total_shares == 0 {
            return Ok(0);
        }
        
        let value = shares
            .checked_mul(share_class.total_value)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(share_class.total_shares)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
            
        Ok(value as u64)
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Managed position within the PositionVault
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct ManagedPosition {
    /// Position NFT mint
    pub position_mint: Pubkey,
    /// Lower tick of the position
    pub tick_lower: i32,
    /// Upper tick of the position
    pub tick_upper: i32,
    /// Liquidity amount
    pub liquidity: u128,
    /// Share type this position supports
    pub share_type: ShareType,
    /// Last rebalance slot
    pub last_rebalance: u64,
}

/// Rebalance configuration for automated management
#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct RebalanceConfig {
    /// Price deviation threshold to trigger rebalance (basis points)
    pub price_deviation_threshold: u16,
    /// Time-based rebalance frequency (slots)
    pub rebalance_frequency: u64,
    /// Width of positions as percentage of current price (basis points)
    pub position_width: u16,
    /// Whether to use just-in-time liquidity
    pub enable_jit: bool,
}

/// User's share token account
/// This is a separate account that tracks a user's shares in the vault
#[account]
pub struct VaultShareAccount {
    /// Vault this account belongs to
    pub vault: Pubkey,
    /// Owner of the shares
    pub owner: Pubkey,
    /// Share positions (user can hold multiple types)
    pub positions: Vec<VaultPosition>,
    /// Last interaction timestamp
    pub last_interaction: i64,
    /// Total unclaimed rewards
    pub unclaimed_rewards: u64,
    /// Reserved
    pub _reserved: [u8; 64],
}

impl VaultShareAccount {
    pub const INITIAL_SIZE: usize = 8 + // discriminator
        32 + 32 + // vault, owner
        4 + // vec length
        8 + 8 + // timestamps and rewards
        64; // reserved
        
    /// Get position for a specific share type
    pub fn get_position(&self, share_type: ShareType) -> Option<&VaultPosition> {
        self.positions.iter().find(|p| p.share_type == share_type)
    }
    
    /// Get mutable position for a specific share type
    pub fn get_position_mut(&mut self, share_type: ShareType) -> Option<&mut VaultPosition> {
        self.positions.iter_mut().find(|p| p.share_type == share_type)
    }
    
    /// Add or update position
    pub fn update_position(
        &mut self,
        share_type: ShareType,
        shares: u128,
        deposit_amount: u64,
        current_time: i64,
    ) -> Result<()> {
        if let Some(position) = self.get_position_mut(share_type) {
            // Update existing position
            position.shares = position.shares
                .checked_add(shares)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            position.deposited_amount = position.deposited_amount
                .checked_add(deposit_amount)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            // Create new position
            let lock_expiry = if share_type.has_lock() {
                current_time + (share_type.lock_duration() as i64 / 2) // Convert slots to seconds
            } else {
                0
            };
            
            self.positions.push(VaultPosition {
                share_type,
                shares,
                deposited_amount: deposit_amount,
                deposit_timestamp: current_time,
                lock_expiry,
                last_reward_claim: current_time,
            });
        }
        
        self.last_interaction = current_time;
        Ok(())
    }
    
    /// Check if withdrawal is allowed
    pub fn can_withdraw(&self, share_type: ShareType, current_time: i64) -> bool {
        if let Some(position) = self.get_position(share_type) {
            position.lock_expiry == 0 || current_time >= position.lock_expiry
        } else {
            false
        }
    }
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct VaultDeposit {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub share_type: ShareType,
    pub deposit_amount: u64,
    pub shares_minted: u128,
    pub timestamp: i64,
}

#[event]
pub struct VaultWithdrawal {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub share_type: ShareType,
    pub shares_burned: u128,
    pub withdrawal_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct VaultRebalance {
    pub vault: Pubkey,
    pub old_positions: u8,
    pub new_positions: u8,
    pub total_liquidity: u128,
    pub timestamp: i64,
}