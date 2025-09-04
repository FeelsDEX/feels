//! # Volatility Oracle
//! 
//! Tracks volatility across multiple timeframes for risk management.
//! Based on log return squared calculations.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::math::safe_math::{safe_add_u128, safe_div_u128, safe_mul_u128};

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Maximum volatility observations
pub const MAX_VOLATILITY_OBSERVATIONS: usize = 24;

/// Volatility scaling factor (10^6)
pub const VOLATILITY_SCALE: u64 = 1_000_000;

/// Volatility observation
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct VolatilityObservation {
    /// Log return squared (scaled by VOLATILITY_SCALE)
    pub log_return_squared: u64,
    /// Timestamp of observation
    pub timestamp: i64,
}

/// Volatility tracking across multiple timeframes
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct VolatilityOracle {
    /// Current volatility (basis points)
    pub current_volatility_bps: u64,
    /// 24-hour volatility (basis points)
    pub volatility_24h_bps: u64,
    /// 7-day volatility (basis points)
    pub volatility_7d_bps: u64,
    
    /// Circular buffer of observations
    pub observations: [VolatilityObservation; MAX_VOLATILITY_OBSERVATIONS],
    /// Current write position
    pub observation_index: u8,
    /// Total observations recorded
    pub observation_count: u8,
    /// Last update timestamp
    pub last_update: i64,
    /// Last price for log return calculation
    pub last_price: u128,
}

impl VolatilityOracle {
    /// Create new volatility oracle
    pub fn new() -> Self {
        Self {
            current_volatility_bps: 0,
            volatility_24h_bps: 0,
            volatility_7d_bps: 0,
            observations: [VolatilityObservation::default(); MAX_VOLATILITY_OBSERVATIONS],
            observation_index: 0,
            observation_count: 0,
            last_update: 0,
            last_price: 0,
        }
    }
    
    /// Update volatility with new price
    pub fn update_volatility(
        &mut self,
        price: u128,
        timestamp: i64,
    ) -> CoreResult<()> {
        if timestamp <= self.last_update {
            return Err(FeelsCoreError::StaleData);
        }
        
        // Calculate log return if we have previous price
        if self.last_price > 0 && price > 0 {
            let log_return_squared = calculate_log_return_squared(self.last_price, price)?;
            
            // Store observation
            let observation = VolatilityObservation {
                log_return_squared,
                timestamp,
            };
            
            self.observations[self.observation_index as usize] = observation;
            self.observation_index = (self.observation_index + 1) % MAX_VOLATILITY_OBSERVATIONS as u8;
            if self.observation_count < MAX_VOLATILITY_OBSERVATIONS as u8 {
                self.observation_count += 1;
            }
            
            // Update current volatility
            self.current_volatility_bps = log_return_to_bps(log_return_squared)?;
        }
        
        // Update aggregated volatilities
        self.update_timeframe_volatilities(timestamp)?;
        
        self.last_price = price;
        self.last_update = timestamp;
        
        Ok(())
    }
    
    /// Update volatility for different timeframes
    fn update_timeframe_volatilities(&mut self, current_time: i64) -> CoreResult<()> {
        // 24-hour volatility
        self.volatility_24h_bps = self.calculate_timeframe_volatility(86400, current_time)?;
        
        // 7-day volatility
        self.volatility_7d_bps = self.calculate_timeframe_volatility(604800, current_time)?;
        
        Ok(())
    }
    
    /// Calculate volatility for specific timeframe
    fn calculate_timeframe_volatility(
        &self,
        window_seconds: i64,
        current_time: i64,
    ) -> CoreResult<u64> {
        if self.observation_count == 0 {
            return Ok(0);
        }
        
        let window_start = current_time.saturating_sub(window_seconds);
        let mut sum_squared_returns = 0u128;
        let mut count = 0u32;
        
        // Iterate through observations
        for i in 0..self.observation_count {
            let idx = if i < self.observation_index {
                (self.observation_index - 1 - i) as usize
            } else {
                (MAX_VOLATILITY_OBSERVATIONS as u8 + self.observation_index - 1 - i) as usize
            };
            
            let obs = &self.observations[idx];
            
            if obs.timestamp >= window_start {
                sum_squared_returns = safe_add_u128(
                    sum_squared_returns,
                    obs.log_return_squared as u128
                )?;
                count += 1;
            }
        }
        
        if count == 0 {
            return Ok(0);
        }
        
        // Calculate average and convert to basis points
        let avg_squared = safe_div_u128(sum_squared_returns, count as u128)?;
        log_return_to_bps(avg_squared as u64)
    }
    
    /// Get risk scalers for each dimension
    pub fn get_risk_scalers(&self) -> (u64, u64, u64) {
        (
            self.volatility_24h_bps, // σ_price
            self.volatility_24h_bps / 2, // σ_rate (typically lower)
            self.volatility_24h_bps * 2, // σ_leverage (typically higher)
        )
    }
}

/// Calculate log return squared
fn calculate_log_return_squared(price_before: u128, price_after: u128) -> CoreResult<u64> {
    // Simplified calculation: (price_after - price_before)^2 / price_before^2 * SCALE
    let delta = if price_after > price_before {
        price_after - price_before
    } else {
        price_before - price_after
    };
    
    let delta_squared = safe_mul_u128(delta, delta)?;
    let price_squared = safe_mul_u128(price_before, price_before)?;
    
    // Scale and convert
    let scaled = safe_mul_u128(delta_squared, VOLATILITY_SCALE as u128)?;
    let result = safe_div_u128(scaled, price_squared)?;
    
    Ok(result as u64)
}

/// Convert log return to basis points
fn log_return_to_bps(log_return_squared: u64) -> CoreResult<u64> {
    // Approximate: sqrt(log_return^2) * 10000
    // For small values, we can use linear approximation
    Ok((log_return_squared * 10000) / VOLATILITY_SCALE)
}

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    /// Advanced volatility metrics
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VolatilityMetrics {
        /// Realized volatility (annualized)
        pub realized_vol: f64,
        /// GARCH volatility forecast
        pub garch_forecast: Option<f64>,
        /// Volatility of volatility
        pub vol_of_vol: f64,
        /// Skewness of returns
        pub skewness: f64,
        /// Kurtosis of returns
        pub kurtosis: f64,
    }
    
    /// Calculate advanced volatility metrics
    pub fn calculate_advanced_metrics(
        oracle: &VolatilityOracle,
        window_seconds: i64,
        current_time: i64,
    ) -> CoreResult<VolatilityMetrics> {
        // Placeholder for advanced calculations
        // In production, implement GARCH, statistical moments, etc.
        
        let realized_vol = oracle.volatility_24h_bps as f64 / 10000.0;
        
        Ok(VolatilityMetrics {
            realized_vol,
            garch_forecast: None,
            vol_of_vol: 0.0,
            skewness: 0.0,
            kurtosis: 3.0, // Normal distribution kurtosis
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_volatility_update() {
        let mut oracle = VolatilityOracle::new();
        
        // Initial price
        oracle.last_price = 100_000_000; // 100 in some unit
        oracle.last_update = 0;
        
        // Update with 1% increase
        oracle.update_volatility(101_000_000, 3600).unwrap();
        assert!(oracle.current_volatility_bps > 0);
        
        // Update with 2% decrease
        oracle.update_volatility(99_000_000, 7200).unwrap();
        assert!(oracle.observation_count == 2);
    }
}