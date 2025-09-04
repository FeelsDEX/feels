//! # TWAP (Time-Weighted Average Price) Oracle
//! 
//! Implements time-weighted average price calculations with multiple time windows.
//! Based on the on-chain implementation but generalized for both on-chain and off-chain use.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::constants::Q64;
use crate::math::safe_math::{safe_add_u128, safe_mul_u128, safe_div_u128};

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Maximum number of observations to store
pub const MAX_OBSERVATIONS: usize = 24;

/// Default TWAP windows in seconds
pub const TWAP_WINDOW_1_HOUR: i64 = 3600;
pub const TWAP_WINDOW_5_MIN: i64 = 300;

/// Price observation at a point in time
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct PriceObservation {
    /// Square root of price (Q64 format)
    pub sqrt_price: u128,
    /// Cumulative volume at observation
    pub cumulative_volume_0: u128,
    pub cumulative_volume_1: u128,
    /// Timestamp of observation
    pub timestamp: i64,
}

/// TWAP oracle maintaining price history
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct TWAPOracle {
    /// Circular buffer of observations
    pub observations: [PriceObservation; MAX_OBSERVATIONS],
    /// Current write position in buffer
    pub observation_index: u8,
    /// Total number of observations recorded
    pub observation_count: u8,
    /// Last update timestamp
    pub last_update: i64,
}

impl TWAPOracle {
    /// Create new TWAP oracle
    pub fn new() -> Self {
        Self {
            observations: [PriceObservation::default(); MAX_OBSERVATIONS],
            observation_index: 0,
            observation_count: 0,
            last_update: 0,
        }
    }
    
    /// Record a new price observation
    pub fn observe_price(
        &mut self,
        sqrt_price: u128,
        cumulative_volume_0: u128,
        cumulative_volume_1: u128,
        timestamp: i64,
    ) -> CoreResult<()> {
        // Validate timestamp
        if timestamp <= self.last_update {
            return Err(FeelsCoreError::StaleData);
        }
        
        // Create new observation
        let observation = PriceObservation {
            sqrt_price,
            cumulative_volume_0,
            cumulative_volume_1,
            timestamp,
        };
        
        // Store in circular buffer
        self.observations[self.observation_index as usize] = observation;
        
        // Update indices
        self.observation_index = (self.observation_index + 1) % MAX_OBSERVATIONS as u8;
        if self.observation_count < MAX_OBSERVATIONS as u8 {
            self.observation_count += 1;
        }
        
        self.last_update = timestamp;
        
        Ok(())
    }
    
    /// Calculate TWAP for specified window
    pub fn get_twap(&self, window_seconds: i64, current_time: i64) -> CoreResult<u128> {
        if self.observation_count == 0 {
            return Err(FeelsCoreError::InsufficientLiquidity);
        }
        
        let window_start = current_time.saturating_sub(window_seconds);
        
        // Find observations within window
        let mut sum_price_time = 0u128;
        let mut total_time = 0i64;
        let mut prev_time = window_start;
        
        // Iterate through observations
        for i in 0..self.observation_count {
            let idx = if i < self.observation_index {
                (self.observation_index - 1 - i) as usize
            } else {
                (MAX_OBSERVATIONS as u8 + self.observation_index - 1 - i) as usize
            };
            
            let obs = &self.observations[idx];
            
            // Skip if before window
            if obs.timestamp < window_start {
                continue;
            }
            
            // Calculate time weight
            let time_weight = obs.timestamp.saturating_sub(prev_time.max(window_start));
            
            // Convert sqrt price to price
            let price = sqrt_price_to_price(obs.sqrt_price)?;
            
            // Accumulate weighted price
            let weighted_price = safe_mul_u128(price, time_weight as u128)?;
            sum_price_time = safe_add_u128(sum_price_time, weighted_price)?;
            
            total_time += time_weight;
            prev_time = obs.timestamp;
        }
        
        // Add weight for current time if needed
        if prev_time < current_time && self.observation_count > 0 {
            let latest = &self.observations[((self.observation_index as i32 - 1 + MAX_OBSERVATIONS as i32) 
                % MAX_OBSERVATIONS as i32) as usize];
            let time_weight = current_time.saturating_sub(prev_time);
            let price = sqrt_price_to_price(latest.sqrt_price)?;
            let weighted_price = safe_mul_u128(price, time_weight as u128)?;
            sum_price_time = safe_add_u128(sum_price_time, weighted_price)?;
            total_time += time_weight;
        }
        
        // Calculate TWAP
        if total_time == 0 {
            return Err(FeelsCoreError::InsufficientLiquidity);
        }
        
        safe_div_u128(sum_price_time, total_time as u128)
    }
    
    /// Get safe price (5-minute TWAP)
    pub fn get_safe_price(&self, current_time: i64) -> CoreResult<u128> {
        self.get_twap(TWAP_WINDOW_5_MIN, current_time)
    }
    
    /// Check if oracle has sufficient observations
    pub fn has_sufficient_observations(&self, min_observations: u8) -> bool {
        self.observation_count >= min_observations
    }
}

/// Convert sqrt price to price
fn sqrt_price_to_price(sqrt_price: u128) -> CoreResult<u128> {
    // price = (sqrt_price)^2 / Q64
    let squared = safe_mul_u128(sqrt_price, sqrt_price)?;
    safe_div_u128(squared, Q64)
}

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    /// Volume-weighted TWAP calculation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VolumeWeightedTWAP {
        pub price_twap: u128,
        pub volume_0: u128,
        pub volume_1: u128,
        pub confidence: f64,
        pub observations_used: u8,
    }
    
    /// Calculate volume-weighted TWAP with confidence metrics
    pub fn calculate_vwap(
        oracle: &TWAPOracle,
        window_seconds: i64,
        current_time: i64,
    ) -> CoreResult<VolumeWeightedTWAP> {
        if oracle.observation_count == 0 {
            return Err(FeelsCoreError::InsufficientLiquidity);
        }
        
        let window_start = current_time.saturating_sub(window_seconds);
        
        let mut sum_price_volume = 0u128;
        let mut total_volume = 0u128;
        let mut observations_used = 0u8;
        let mut prev_volume_0 = 0u128;
        let mut prev_volume_1 = 0u128;
        
        // Iterate through observations
        for i in 0..oracle.observation_count {
            let idx = if i < oracle.observation_index {
                (oracle.observation_index - 1 - i) as usize
            } else {
                (MAX_OBSERVATIONS as u8 + oracle.observation_index - 1 - i) as usize
            };
            
            let obs = &oracle.observations[idx];
            
            // Skip if before window
            if obs.timestamp < window_start {
                continue;
            }
            
            // Calculate volume delta
            let volume_delta_0 = obs.cumulative_volume_0.saturating_sub(prev_volume_0);
            let volume_delta_1 = obs.cumulative_volume_1.saturating_sub(prev_volume_1);
            let volume = volume_delta_0.saturating_add(volume_delta_1);
            
            if volume > 0 {
                let price = sqrt_price_to_price(obs.sqrt_price)?;
                let weighted = safe_mul_u128(price, volume)?;
                sum_price_volume = safe_add_u128(sum_price_volume, weighted)?;
                total_volume = safe_add_u128(total_volume, volume)?;
                observations_used += 1;
            }
            
            prev_volume_0 = obs.cumulative_volume_0;
            prev_volume_1 = obs.cumulative_volume_1;
        }
        
        if total_volume == 0 {
            return Err(FeelsCoreError::InsufficientLiquidity);
        }
        
        let vwap = safe_div_u128(sum_price_volume, total_volume)?;
        let confidence = (observations_used as f64) / (oracle.observation_count as f64);
        
        Ok(VolumeWeightedTWAP {
            price_twap: vwap,
            volume_0: prev_volume_0,
            volume_1: prev_volume_1,
            confidence,
            observations_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_twap_observation() {
        let mut oracle = TWAPOracle::new();
        
        // Add observations
        oracle.observe_price(100 * Q64, 1000, 1000, 100).unwrap();
        oracle.observe_price(105 * Q64, 2000, 2000, 200).unwrap();
        oracle.observe_price(110 * Q64, 3000, 3000, 300).unwrap();
        
        assert_eq!(oracle.observation_count, 3);
        assert_eq!(oracle.last_update, 300);
    }
    
    #[test]
    fn test_twap_calculation() {
        let mut oracle = TWAPOracle::new();
        
        // Add observations with increasing prices
        oracle.observe_price(100 * Q64, 1000, 1000, 0).unwrap();
        oracle.observe_price(110 * Q64, 2000, 2000, 60).unwrap();
        oracle.observe_price(120 * Q64, 3000, 3000, 120).unwrap();
        
        // Calculate TWAP for 2-minute window
        let twap = oracle.get_twap(120, 120).unwrap();
        
        // TWAP should be between min and max prices
        assert!(twap >= 10000 * Q64 && twap <= 14400 * Q64);
    }
}