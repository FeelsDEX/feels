/// Fee system for the protocol including both static and dynamic fee configurations.
/// Phase 1 uses static fee tiers while Phase 2 adds dynamic fees based on market conditions
/// including volatility and trading volume to optimize liquidity provider returns.
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Fee configuration account for each pool
/// 
/// This account stores ALL fee parameters for a specific pool,
/// serving as the single source of truth for fee calculations.
/// This consolidates static rates, protocol shares, and dynamic fee logic.
#[account]
pub struct FeeConfig {
    /// Reference to the pool this fee configuration belongs to
    pub pool: Pubkey,
    
    /// Base fee rate for the pool (in basis points)
    /// e.g., 30 = 0.30%
    pub base_rate: u16,
    
    /// Protocol's share of the fees (in basis points)
    /// e.g., 2500 = 25% of fees go to protocol
    pub protocol_share: u16,
    
    /// LP's share of the fees (in basis points)
    /// Calculated as base_rate - protocol_fee
    pub liquidity_share: u16,
    
    /// Tick spacing for this fee tier
    pub tick_spacing: i16,
    
    /// Dynamic fee configuration
    pub dynamic_config: DynamicFeeConfig,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Authority that can update fee parameters
    pub update_authority: Pubkey,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 32],
}

impl FeeConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        2 +  // base_rate
        2 +  // protocol_share
        2 +  // liquidity_share
        2 +  // tick_spacing
        std::mem::size_of::<DynamicFeeConfig>() + // dynamic_config
        8 +  // last_update
        32 + // update_authority
        32;  // _reserved

    /// Create a fee configuration for a pool with the given fee rate
    pub fn create_for_pool(fee_rate: u16) -> Result<(u16, u16, u16)> {
        use crate::utils::VALID_FEE_TIERS;
        use crate::state::FeelsProtocolError;
        
        // Find the matching fee tier
        if !VALID_FEE_TIERS.contains(&fee_rate) {
            return Err(FeelsProtocolError::InvalidFeeRate.into());
        }
        
        // Get default protocol fee rate and tick spacing for the fee tier
        let protocol_fee_rate = 2500; // 25% default
        let tick_spacing = match fee_rate {
            1 => 1,     // 0.01%
            5 => 10,    // 0.05%
            30 => 60,   // 0.30%
            100 => 200, // 1.00%
            _ => return Err(FeelsProtocolError::InvalidFeeRate.into()),
        };
        
        Ok((fee_rate, protocol_fee_rate, tick_spacing))
    }
    
    /// Initialize a new fee configuration
    pub fn initialize(
        &mut self,
        pool: Pubkey,
        base_rate: u16,
        protocol_share: u16,
        tick_spacing: i16,
        update_authority: Pubkey,
    ) -> Result<()> {
        use crate::state::FeelsProtocolError;
        
        // Validate protocol share
        require!(
            protocol_share <= 10000,
            FeelsProtocolError::InvalidProtocolFeeRate
        );
        
        // Calculate liquidity share
        let liquidity_share = base_rate.saturating_sub(
            base_rate.saturating_mul(protocol_share) / 10000
        );
        
        self.pool = pool;
        self.base_rate = base_rate;
        self.protocol_share = protocol_share;
        self.liquidity_share = liquidity_share;
        self.tick_spacing = tick_spacing;
        self.dynamic_config = DynamicFeeConfig::default();
        self.last_update = Clock::get()?.unix_timestamp;
        self.update_authority = update_authority;
        
        Ok(())
    }
    
    /// Get the current fee rate (base or dynamic)
    pub fn get_fee_rate(&self, volatility_bps: Option<u64>, volume_24h: Option<u128>) -> u16 {
        if let (Some(vol), Some(volume)) = (volatility_bps, volume_24h) {
            self.dynamic_config.calculate_fee(vol, volume)
        } else {
            self.base_rate
        }
    }
    
    /// Calculate protocol fee amount
    pub fn calculate_protocol_fee(&self, total_fee: u64) -> u64 {
        (total_fee as u128)
            .saturating_mul(self.protocol_share as u128)
            .saturating_div(10000) as u64
    }
    
    /// Calculate liquidity provider fee amount
    pub fn calculate_lp_fee(&self, total_fee: u64) -> u64 {
        total_fee.saturating_sub(self.calculate_protocol_fee(total_fee))
    }
}

/// Dynamic fee configuration that adjusts based on market conditions
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
pub struct DynamicFeeConfig {
    /// Coefficient for volatility adjustment (6 decimals)
    pub volatility_coefficient: u64,
    /// Volume threshold for discounts
    pub volume_discount_threshold: u128,
    /// Base fee rate in basis points
    pub base_fee: u16,
    /// Minimum allowed fee
    pub min_fee: u16,
    /// Maximum allowed fee
    pub max_fee: u16,
    /// Minimum fee multiplier (x10000)
    pub min_multiplier: u16,
    /// Maximum fee multiplier (x10000)
    pub max_multiplier: u16,
    /// Padding for alignment
    pub _padding: [u8; 6],
}

impl DynamicFeeConfig {
    /// Calculate dynamic fee based on market conditions
    pub fn calculate_fee(&self, volatility_bps: u64, volume_24h: u128) -> u16 {
        // Start with base fee
        let mut fee = self.base_fee as u64;

        // Apply volatility adjustment
        if volatility_bps > 500 {
            // High volatility: increase fee by 50%
            fee = fee.saturating_mul(150) / 100;
        } else if volatility_bps > 300 {
            // Medium volatility: increase fee by 20%
            fee = fee.saturating_mul(120) / 100;
        }

        // Apply volume discount
        if volume_24h > self.volume_discount_threshold {
            // High volume: 10% discount
            fee = fee.saturating_mul(90) / 100;
        }

        // Clamp to min/max
        fee.clamp(self.min_fee as u64, self.max_fee as u64) as u16
    }
}
