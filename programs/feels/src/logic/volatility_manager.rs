/// Advanced volatility analysis and calculation system for dynamic fee adjustments.
/// Combines price movement volatility with flash loan volume signals to detect market stress.
/// Uses fixed-point arithmetic for accurate log return calculations and includes
/// manipulation detection with recovery mechanisms for market stability.
///
/// TODO: Future optimization with compressed accounts:
/// - Store only recent volatility data on-chain (e.g., last hour)
/// - Archive historical volatility to compressed storage
/// - Use proofs for backtesting and analysis
/// - Enable unlimited volatility history without rent costs

use anchor_lang::prelude::*;
use fixed::types::I64F64;
use crate::state::{VolatilityTracker, FeelsProtocolError, DynamicFeeConfig, LendingMetrics};

pub struct VolatilityManager;

// Multi-tier volatility tracking for different time horizons
pub const VOLATILITY_WINDOWS: [(u32, u16); 4] = [
    (300, 9500),     // 5 minutes - weight 95%
    (900, 8500),     // 15 minutes - weight 85%  
    (3600, 7000),    // 1 hour - weight 70%
    (86400, 5000),   // 24 hours - weight 50%
];

pub const MIN_UPDATE_INTERVAL: i64 = 10;

#[derive(Clone, Debug)]
pub struct RiskMetrics {
    pub current_volatility: u64,
    pub volatility_trend: VolatilityTrend,
    pub volatility_acceleration: u32,
    pub time_since_spike: i64,
    pub spike_active: bool,
    pub stability_score: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VolatilityTrend {
    Increasing,
    Decreasing,
}

impl VolatilityManager {
    /// Calculate enhanced volatility using both price and flash loan volume signals
    pub fn calculate_enhanced_volatility(
        price_tracker: &VolatilityTracker,
        flash_twav: &FlashLoanTWAV,
        config: &DynamicFeeConfig,
    ) -> Result<u32> {
        // Get composite price volatility
        let price_volatility = price_tracker.get_composite_volatility();
        
        // Flash loan signal strength based on current activity
        let flash_signal = if flash_twav.burst_detected {
            20000 // 2x multiplier during bursts
        } else {
            // Compare 5-minute volume to hourly average
            let avg_5m_volume = flash_twav.volume_1h / 12;
            if avg_5m_volume > 0 {
                (flash_twav.volume_5m as u128 * 10000 / avg_5m_volume as u128)
                    .min(30000) as u32
            } else {
                10000
            }
        };
        
        // Both trackers showing spikes = very high confidence
        let correlation_bonus = if price_tracker.spike_detected && flash_twav.burst_detected {
            5000 // Additional 50% when both spike
        } else {
            0
        };
        
        // Combine signals with weights (assuming flash_volume_weight is in basis points)
        let enhanced_volatility = if config.volatility_coefficient > 0 {
            let price_weight = 10000u32.saturating_sub(config.volatility_coefficient as u32 / 100);
            let flash_weight = config.volatility_coefficient as u32 / 100;
            
            let base = (
                price_volatility as u64 * price_weight as u64 +
                price_volatility as u64 * flash_signal as u64 / 10000 * flash_weight as u64
            ) / 10000;
            
            // Apply correlation bonus
            ((base * (10000 + correlation_bonus) / 10000) as u32)
                .min(config.max_multiplier as u32 * 100)
        } else {
            price_volatility
        };
        
        Ok(enhanced_volatility)
    }
    
    pub fn update_volatility(
        tracker: &mut VolatilityTracker,
        current_price: u128,
        current_timestamp: i64,
    ) -> Result<()> {
        // Skip if this is the first observation
        if tracker.last_price == 0 {
            tracker.last_price = current_price;
            tracker.last_update = current_timestamp;
            return Ok(());
        }

        // Calculate time since last update
        let time_elapsed = current_timestamp.saturating_sub(tracker.last_update);
        
        // Update every 10-15 seconds during active trading
        if time_elapsed < MIN_UPDATE_INTERVAL {
            return Ok(());
        }

        // Calculate log return using the `fixed` crate
        let log_return = calculate_log_return_fixed(tracker.last_price, current_price)?;
        
        // Check for manipulation - using fixed-point result
        let price_change_pct = log_return.abs();
        if price_change_pct > tracker.manipulation_threshold as i32 {
            // Suspicious price movement - cap the impact
            return Err(FeelsProtocolError::PriceManipulationDetected.into());
        }

        // Store new observation in circular buffer
        if time_elapsed >= 120 { // New observation every 2 minutes
            tracker.observation_index = (tracker.observation_index + 1) % 180;
            tracker.observation_count = tracker.observation_count.saturating_add(1).min(180);
            
            let obs = &mut tracker.observations[tracker.observation_index as usize];
            obs.timestamp = current_timestamp;
            obs.log_return_squared = log_return.pow(2).min(u32::MAX) as u32;
            obs.price = compress_price(current_price);
        }
        
        // Calculate volatility for different timeframes
        tracker.last_volatility_5m = tracker.volatility_5m;
        tracker.volatility_5m = Self::calculate_window_volatility(&tracker.observations, 300, current_timestamp)?;
        tracker.volatility_15m = Self::calculate_window_volatility(&tracker.observations, 900, current_timestamp)?;
        tracker.volatility_1h = Self::calculate_window_volatility(&tracker.observations, 3600, current_timestamp)?;
        tracker.volatility_24h = Self::calculate_window_volatility(&tracker.observations, 86400, current_timestamp)?;
        
        // Calculate volatility change rate (basis points per minute)
        let vol_change = tracker.volatility_5m as i16 - tracker.last_volatility_5m as i16;
        tracker.volatility_change_rate = vol_change / 5; // Change per minute
        
        // Spike detection - if 5-minute volatility is 3x the hourly average
        if tracker.volatility_5m > tracker.volatility_1h * 3 && !tracker.spike_detected {
            tracker.spike_detected = true;
            tracker.spike_start_time = current_timestamp;
            tracker.spike_magnitude = tracker.volatility_5m;
        } else if tracker.volatility_5m < tracker.volatility_1h * 2 && tracker.spike_detected {
            // Spike has subsided
            tracker.spike_detected = false;
        }
        
        // Update state
        tracker.last_price = current_price;
        tracker.last_update = current_timestamp;
        
        Ok(())
    }
    
    fn calculate_window_volatility(
        observations: &[crate::state::VolatilityObservation],
        window_seconds: u32,
        current_time: i64,
    ) -> Result<u32> {
        let window_start = current_time - window_seconds as i64;
        let mut sum = 0u64;
        let mut count = 0u32;
        
        for obs in observations {
            if obs.timestamp >= window_start && obs.timestamp <= current_time {
                sum = sum.saturating_add(obs.log_return_squared as u64);
                count += 1;
            }
        }
        
        if count > 0 {
            Ok((sum / count as u64) as u32)
        } else {
            Ok(0)
        }
    }
    
    /// Calculate risk-adjusted parameters for PositionVault
    pub fn get_risk_metrics(tracker: &VolatilityTracker) -> RiskMetrics {
        RiskMetrics {
            current_volatility: tracker.get_composite_volatility() as u64,
            volatility_trend: if tracker.volatility_change_rate > 0 { 
                VolatilityTrend::Increasing 
            } else { 
                VolatilityTrend::Decreasing 
            },
            volatility_acceleration: tracker.volatility_change_rate.abs() as u32,
            time_since_spike: if tracker.spike_detected {
                Clock::get().unwrap().unix_timestamp - tracker.spike_start_time
            } else {
                i64::MAX
            },
            spike_active: tracker.spike_detected,
            stability_score: calculate_stability_score(tracker),
        }
    }

    pub fn calculate_volatility_multiplier(
        tracker: &VolatilityTracker,
        config: &DynamicFeeConfig,
    ) -> Result<u16> {
        // Use composite volatility for fee calculation
        let volatility = tracker.get_composite_volatility();
        let base_multiplier = 10000; // 1.0x
        
        // Higher multiplier during spikes
        let spike_multiplier = if tracker.spike_detected {
            15000 // 1.5x during spikes
        } else {
            10000
        };
        
        // Calculate additional fee based on volatility
        // volatility_fee = base_fee * (1 + coefficient * sqrt(volatility))
        let sqrt_volatility = integer_sqrt(volatility as u64)?;
        let additional_fee = (config.volatility_coefficient * sqrt_volatility) / 10000;
        
        // Apply bounds
        let multiplier = (base_multiplier + additional_fee) * spike_multiplier / 10000;
        let capped_multiplier = multiplier
            .min(config.max_multiplier as u64)
            .max(config.min_multiplier as u64);
            
        Ok(capped_multiplier as u16)
    }
}

/// Calculate log return using fixed-point arithmetic from the `fixed` crate
fn calculate_log_return_fixed(previous_price: u128, current_price: u128) -> Result<i32> {
    if previous_price == 0 || current_price == 0 {
        return Err(FeelsProtocolError::InvalidPrice.into());
    }
    
    // Convert to fixed-point with appropriate scaling
    // I64F64 gives us 64 bits of integer and 64 bits of fractional precision
    let scale = 1_000_000u128; // Scale to maintain precision for typical prices
    
    // Scale down large prices to fit in I64F64 range
    let (prev_scaled, curr_scaled) = if previous_price > u64::MAX as u128 {
        (
            (previous_price / scale) as u64,
            (current_price / scale) as u64,
        )
    } else {
        (previous_price as u64, current_price as u64)
    };
    
    let prev = I64F64::from_num(prev_scaled);
    let curr = I64F64::from_num(curr_scaled);
    
    // Calculate ratio
    let ratio = curr.checked_div(prev)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    // Check for extreme ratios before calculating log
    if ratio > I64F64::from_num(10) || ratio < I64F64::from_num(0.1) {
        return Err(FeelsProtocolError::ExtremePrice.into());
    }
    
    // Calculate natural logarithm
    let log_ratio = ratio.checked_ln()
        .ok_or(FeelsProtocolError::LogarithmUndefined)?;
    
    // Convert to basis points (multiply by 10,000)
    let log_return_bps = (log_ratio * I64F64::from_num(10_000))
        .to_num::<i32>();
    
    Ok(log_return_bps)
}

/// Compress price to u64 for storage efficiency
fn compress_price(price: u128) -> u64 {
    // Simple compression - store top 64 bits of precision
    (price >> 64) as u64
}

/// Integer square root for fee calculations
fn integer_sqrt(n: u64) -> Result<u64> {
    if n == 0 {
        return Ok(0);
    }
    
    let mut x = n;
    let mut y = (x + 1) / 2;
    
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    
    Ok(x)
}

/// Calculate stability score based on volatility metrics
fn calculate_stability_score(tracker: &VolatilityTracker) -> u32 {
    // Higher score = more stable
    let base_score = 10000u32;
    
    // Penalize if spike detected
    let spike_penalty = if tracker.spike_detected { 5000 } else { 0 };
    
    // Penalize based on volatility level
    let vol_penalty = tracker.get_composite_volatility().min(5000);
    
    // Penalize rapid changes
    let change_penalty = (tracker.volatility_change_rate.abs() as u32).min(3000);
    
    base_score.saturating_sub(spike_penalty + vol_penalty + change_penalty)
}

#[cfg(feature = "compute-budget")]
pub fn benchmark_log_calculation() -> Result<u64> {
    use solana_program::compute_budget::get_compute_units;
    let start = get_compute_units();
    let _ = calculate_log_return_fixed(1_000_000_000, 1_010_000_000)?;
    let end = get_compute_units();
    Ok(end - start)
}