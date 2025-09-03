/// Duration types for the 3D liquidity model representing time commitments.
/// Defines time dimensions from instant flash loans to annual commitments that work
/// with leverage and rate dimensions to create a unified position model.
/// Duration affects fee calculations and position behavior throughout the protocol.

use anchor_lang::prelude::*;
use crate::utils::bitmap::u8_bitmap;

// ============================================================================
// Duration Types
// ============================================================================

/// Duration types for the 3D liquidity model (Rate × Duration × Leverage)
/// Represents time commitments from flash loans to annual terms
#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duration {
    Flash = 0,      // 1 block duration (flash loans)
    Swap = 1,       // Immediate execution (spot trading)
    Weekly = 2,     // 7 day commitment
    Monthly = 3,    // 28 day commitment  
    Quarterly = 4,  // 90 day commitment
    Annual = 5,     // 365 day commitment
}

impl Default for Duration {
    fn default() -> Self {
        Duration::Swap // Default to immediate execution
    }
}

impl Duration {
    pub const COUNT: usize = 6;
    
    /// Convert duration to number of slots
    pub fn to_slots(&self) -> u64 {
        match self {
            Duration::Flash => 1,
            Duration::Swap => 0, // Immediate
            Duration::Weekly => 7 * 24 * 60 * 60 * 2,      // ~2 slots/second on Solana
            Duration::Monthly => 28 * 24 * 60 * 60 * 2,    
            Duration::Quarterly => 90 * 24 * 60 * 60 * 2,
            Duration::Annual => 365 * 24 * 60 * 60 * 2,
        }
    }
    
    /// Get duration weight for fee calculations (longer duration = lower fees)
    pub fn fee_multiplier(&self) -> u16 {
        match self {
            Duration::Flash => 300,     // 3x fees for flash loans
            Duration::Swap => 100,      // Base fee rate
            Duration::Weekly => 90,     // 10% discount
            Duration::Monthly => 80,    // 20% discount
            Duration::Quarterly => 70,  // 30% discount
            Duration::Annual => 50,     // 50% discount
        }
    }
    
    /// Get protection priority (shorter duration = higher priority in redenomination)
    pub fn protection_priority(&self) -> u16 {
        match self {
            Duration::Flash => 100,
            Duration::Swap => 90,
            Duration::Weekly => 70,
            Duration::Monthly => 50,
            Duration::Quarterly => 30,
            Duration::Annual => 10,
        }
    }
    
    /// Check if position has matured
    pub fn is_matured(&self, position_slot: u64, current_slot: u64) -> bool {
        if *self == Duration::Swap {
            return true; // Swap positions are always "mature" (no lock)
        }
        
        let maturity_slot = position_slot.saturating_add(self.to_slots());
        current_slot >= maturity_slot
    }
    
    /// Convert to tick for 3D encoding
    pub fn to_tick(&self) -> i16 {
        *self as i16
    }
    
    /// Convert to u8 for encoding
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
    
    /// Convert duration to blocks
    pub fn to_blocks(&self) -> u64 {
        self.to_slots()
    }
    
    /// Convert to time factor for physics calculations using Q64 fixed-point
    pub fn to_time_factor(&self) -> Result<u128> {
        use crate::constant::Q64;
        
        let days = match self {
            Duration::Flash => 1,       // 1 day minimum for flash loans
            Duration::Swap => 1,        // Immediate execution treated as 1 day
            Duration::Weekly => 7,
            Duration::Monthly => 28,
            Duration::Quarterly => 90,
            Duration::Annual => 365,
        };
        
        // Time factor = 1 / (1 + days/365) in Q64 format
        // This represents the time decay factor for the physics model
        // To avoid overflow, just return Q64 scaled by the inverse
        let factor = (Q64 as u128).saturating_mul(365) / (365 + days as u128);
        
        Ok(factor)
    }
}

/// Duration configuration for pools
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct DurationConfig {
    /// Allowed durations bitmask (bit i set = Duration variant i allowed)
    pub allowed_durations: u8,
    
    /// Minimum duration for new positions
    pub min_duration: Duration,
    
    /// Maximum duration for new positions  
    pub max_duration: Duration,
}

impl Default for DurationConfig {
    fn default() -> Self {
        Self {
            // Allow all durations by default
            allowed_durations: 0b00111111, 
            min_duration: Duration::Swap,
            max_duration: Duration::Annual,
        }
    }
}

impl DurationConfig {
    /// Check if a duration is allowed in this pool
    pub fn is_duration_allowed(&self, duration: Duration) -> bool {
        u8_bitmap::is_bit_set(self.allowed_durations, duration as usize).unwrap_or(false)
    }
}