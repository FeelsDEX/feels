/// Volatility tracking system for multi-timeframe market volatility analysis.
/// Uses high-frequency price observations to calculate volatility across different time windows
/// for dynamic fee adjustments and risk management. Includes spike detection and vol-of-vol
/// metrics to enhance trading experience during volatile market conditions.

use anchor_lang::prelude::*;

// ============================================================================
// Volatility Data Structures
// ============================================================================

#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct VolatilityObservation {
    pub timestamp: i64,
    pub log_return_squared: u32,  // Log return squared (basis points squared)
    pub price: u64,               // Compressed price for efficiency
}

// ============================================================================
// Volatility Tracker Account
// ============================================================================

#[account]
pub struct VolatilityTracker {
    pub pool: Pubkey,
    
    // High-frequency circular buffer (2-minute intervals for 6 hours = 180 observations)
    pub observations: [VolatilityObservation; 180],
    pub observation_index: u8,
    pub observation_count: u16,
    
    // Multi-timeframe volatility metrics
    pub volatility_5m: u32,       // 5-minute volatility (basis points squared)
    pub volatility_15m: u32,      // 15-minute volatility
    pub volatility_1h: u32,       // 1-hour volatility
    pub volatility_24h: u32,      // 24-hour volatility
    
    // Vol-of-vol tracking
    pub last_volatility_5m: u32,
    pub volatility_change_rate: i16,  // Rate of change in volatility (bps/minute)
    
    // Spike detection
    pub spike_detected: bool,
    pub spike_start_time: i64,
    pub spike_magnitude: u32,
    
    // Basic tracking
    pub last_price: u128,
    pub last_update: i64,
    pub decay_factor: u16,         // Lambda * 10000 (e.g., 9900 = 0.99 for fast decay)
    pub manipulation_threshold: u16,
    
    pub _reserved: [u8; 32],
}

impl VolatilityTracker {
    pub const SIZE: usize = 8 + 32 + // discriminator + pool
        (180 * std::mem::size_of::<VolatilityObservation>()) + // observations
        1 + 2 + // indices
        4 * 4 + // volatility metrics
        4 + 2 + // vol-of-vol
        1 + 8 + 4 + // spike detection
        16 + 8 + 2 + 2 + // basic tracking
        32; // reserved
    
    // Total size: ~3.2KB for 6 hours of granular data
    
    /// Get weighted volatility across multiple timeframes
    pub fn get_composite_volatility(&self) -> u32 {
        // Weight recent volatility more heavily
        let weighted = 
            self.volatility_5m as u64 * 40 +   // 40% weight
            self.volatility_15m as u64 * 30 +  // 30% weight
            self.volatility_1h as u64 * 20 +   // 20% weight
            self.volatility_24h as u64 * 10;   // 10% weight
            
        (weighted / 100) as u32
    }
    
    /// Check if we should update (minimum 10 seconds between updates)
    pub fn should_update(&self, current_time: i64) -> bool {
        current_time.saturating_sub(self.last_update) >= 10
    }
}