/// Time-weighted average utilities providing generic TWAP/TWAV calculations.
/// Implements a flexible observation buffer system that can track various metrics
/// over configurable time windows. Used by oracle systems for price TWAP and
/// flash loan volume tracking for volatility detection and dynamic fee calculations.

use anchor_lang::prelude::*;
use crate::state::FeelsProtocolError;

// ============================================================================
// Time-Weighted Observation Trait
// ============================================================================

/// Generic trait for time-weighted observations (TWAP/TWAV)
pub trait TimeWeightedObservation: Clone + Copy + Default {
    fn timestamp(&self) -> i64;
    fn cumulative_value(&self) -> u128;
}

// ============================================================================
// Time-Weighted Average Buffer
// ============================================================================

/// Generic buffer for time-weighted averages
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TimeWeightedAverageBuffer<T: TimeWeightedObservation, const N: usize> {
    pub observations: [T; N],
    pub observation_index: u8,
    pub observation_cardinality: u8,
    pub last_observation_time: i64,
}

impl<T: TimeWeightedObservation, const N: usize> TimeWeightedAverageBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            observations: [T::default(); N],
            observation_index: 0,
            observation_cardinality: 0,
            last_observation_time: 0,
        }
    }
    
    /// Get time-weighted average over a window
    pub fn get_twa(&self, window_seconds: u32, current_time: i64) -> Option<u128> {
        let window_start = current_time - window_seconds as i64;
        
        if self.observation_cardinality == 0 {
            return None;
        }
        
        // Find observations that bracket the window
        let (start_obs, end_obs) = match self.find_observations_for_window(window_start, current_time) {
            Some(obs) => obs,
            None => return None,
        };
        
        // Calculate time-weighted average
        let value_delta = end_obs.cumulative_value()
            .saturating_sub(start_obs.cumulative_value());
        let time_delta = end_obs.timestamp()
            .saturating_sub(start_obs.timestamp());
            
        if time_delta > 0 {
            Some(value_delta / time_delta as u128)
        } else {
            None
        }
    }
    
    /// Add new observation
    pub fn record_observation(&mut self, observation: T) -> Result<()> {
        // Ensure chronological order
        crate::utils::ErrorHandling::validate_timestamp_ordering(
            observation.timestamp(),
            self.last_observation_time,
        )?;
        
        // Update index and cardinality
        self.observation_index = (self.observation_index + 1) % N as u8;
        if self.observation_cardinality < N as u8 {
            self.observation_cardinality += 1;
        }
        
        // Store observation
        self.observations[self.observation_index as usize] = observation;
        self.last_observation_time = observation.timestamp();
        
        Ok(())
    }
    
    /// Find observations that bracket a time window
    fn find_observations_for_window(
        &self,
        window_start: i64,
        window_end: i64,
    ) -> Option<(T, T)> {
        if self.observation_cardinality == 0 {
            return None;
        }
        
        // Binary search for start observation
        let start_obs = self.find_observation_at_or_before(window_start)?;
        
        // Binary search for end observation
        let end_obs = self.find_observation_at_or_before(window_end)?;
        
        Some((start_obs, end_obs))
    }
    
    /// Binary search for observation at or before timestamp
    fn find_observation_at_or_before(&self, timestamp: i64) -> Option<T> {
        let mut best_obs = None;
        let mut best_time = i64::MIN;
        
        // Linear search for now (can optimize to binary search)
        for i in 0..self.observation_cardinality as usize {
            let obs = self.observations[i];
            let obs_time = obs.timestamp();
            
            if obs_time <= timestamp && obs_time > best_time {
                best_obs = Some(obs);
                best_time = obs_time;
            }
        }
        
        best_obs
    }
    
    /// Get latest observation
    pub fn latest(&self) -> Option<T> {
        if self.observation_cardinality == 0 {
            return None;
        }
        Some(self.observations[self.observation_index as usize])
    }
    
    /// Check if buffer needs update (minimum interval)
    pub fn should_update(&self, current_time: i64, min_interval: i64) -> bool {
        current_time.saturating_sub(self.last_observation_time) >= min_interval
    }
}

/// Price observation for TWAP
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct PriceObservation {
    pub timestamp: i64,
    pub cumulative_price: u128,
    pub sqrt_price: u128,
}

impl TimeWeightedObservation for PriceObservation {
    fn timestamp(&self) -> i64 { self.timestamp }
    fn cumulative_value(&self) -> u128 { self.cumulative_price }
}

/// Volume observation for TWAV (flash loans)
#[derive(Clone, Copy, Default, AnchorSerialize, AnchorDeserialize)]
pub struct FlashVolumeObservation {
    pub timestamp: i64,
    pub cumulative_volume: u128,
    pub volume_rate: u64,  // Volume per second
}

impl TimeWeightedObservation for FlashVolumeObservation {
    fn timestamp(&self) -> i64 { self.timestamp }
    fn cumulative_value(&self) -> u128 { self.cumulative_volume }
}

/// Helper to calculate time-weighted metrics
pub struct TimeWeightedMetrics;

impl TimeWeightedMetrics {
    /// Calculate rate of change (derivative) from TWA values
    pub fn calculate_rate_of_change(
        current_twa: u128,
        previous_twa: u128,
        time_delta: i64,
    ) -> Result<i64> {
        if time_delta == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }
        
        let value_delta = if current_twa >= previous_twa {
            (current_twa - previous_twa) as i64
        } else {
            -((previous_twa - current_twa) as i64)
        };
        
        Ok(value_delta / time_delta)
    }
    
    /// Detect spikes in time-weighted values
    pub fn detect_spike(
        current_value: u128,
        average_value: u128,
        spike_threshold_multiplier: u64,
    ) -> bool {
        current_value > average_value * spike_threshold_multiplier as u128 / 10000
    }
    
    /// Calculate acceleration (second derivative)
    pub fn calculate_acceleration(
        current_rate: i64,
        previous_rate: i64,
        time_delta: i64,
    ) -> Result<i64> {
        if time_delta == 0 {
            return Err(FeelsProtocolError::DivisionByZero.into());
        }
        
        Ok((current_rate - previous_rate) / time_delta)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_weighted_average() {
        let mut buffer: TimeWeightedAverageBuffer<PriceObservation, 10> = 
            TimeWeightedAverageBuffer::new();
        
        // Add observations
        let obs1 = PriceObservation {
            timestamp: 1000,
            cumulative_price: 1000000,
            sqrt_price: 1000,
        };
        buffer.record_observation(obs1).unwrap();
        
        let obs2 = PriceObservation {
            timestamp: 2000,
            cumulative_price: 2000000,
            sqrt_price: 1500,
        };
        buffer.record_observation(obs2).unwrap();
        
        // Calculate TWA
        let twa = buffer.get_twa(1000, 2000).unwrap();
        assert_eq!(twa, 1000); // (2000000 - 1000000) / 1000
    }
}