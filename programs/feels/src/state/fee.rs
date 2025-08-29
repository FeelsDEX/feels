/// Fee system for the protocol including both static and dynamic fee configurations.
/// Phase 1 uses static fee tiers while Phase 2 adds dynamic fees based on market conditions
/// including volatility and trading volume to optimize liquidity provider returns.
use anchor_lang::prelude::*;

/// Fee configuration account for each pool
/// 
/// This account stores the fee parameters for a specific pool,
/// allowing for dynamic fee updates and modular fee management.
#[account]
pub struct FeeConfig {
    /// Reference to the pool this fee configuration belongs to
    pub pool: Pubkey,
    
    /// Static fee rate for the pool (in basis points)
    /// e.g., 30 = 0.30%
    pub base_rate: u16,
    
    /// Protocol's share of the fees (in basis points)
    /// e.g., 2500 = 25% of fees go to protocol
    pub protocol_share: u16,
    
    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}

impl FeeConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        2 +  // base_rate
        2 +  // protocol_share
        64;  // _reserved

    /// Create a fee configuration for a pool with the given fee rate
    pub fn create_for_pool(fee_rate: u16) -> Result<(u16, u16, u16)> {
        use crate::utils::VALID_FEE_TIERS;
        use crate::state::FeelsProtocolError;
        
        // Find the matching fee tier configuration
        let tier = VALID_FEE_TIERS
            .iter()
            .find(|tier| tier.fee_rate == fee_rate)
            .ok_or(FeelsProtocolError::InvalidFeeRate)?;
        
        Ok((tier.fee_rate, tier.protocol_fee_rate, tier.tick_spacing))
    }
}

/// Dynamic fee configuration that adjusts based on market conditions
#[derive(Default, Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DynamicFeeConfig {
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
    pub _padding: u16,
    /// Coefficient for volatility adjustment (6 decimals)
    pub volatility_coefficient: u64,
    /// Volume threshold for discounts
    pub volume_discount_threshold: u128,
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
