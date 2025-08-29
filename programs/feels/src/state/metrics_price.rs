/// Unified oracle system with manipulation-resistant TWAP calculations and volatility tracking
/// 
/// Comprehensive oracle implementation that provides:
/// - Time-weighted average price calculations with multiple windows
/// - Real-time volatility tracking for dynamic fee adjustments
/// - Maximum deviation protection against manipulation
/// - Update frequency limits to prevent spam
/// - Manipulation detection and recovery mechanisms
/// - Efficient storage with header/data separation (1024 observations)
///
/// TODO: Future optimization with compressed accounts:
/// - Keep only recent observations on-chain (e.g., last 100)
/// - Archive historical data to merkle tree
/// - Generate proofs for TWAP calculations
/// - Enable unlimited price history without rent costs

use anchor_lang::prelude::*;
use crate::state::FeelsProtocolError;
use crate::utils::TickMath;
use fixed::types::I64F64;

// ============================================================================
// Constants and Configuration
// ============================================================================

/// Maximum observations in the ring buffer
pub const MAX_OBSERVATIONS: u16 = 1024;

/// Minimum time between oracle updates (seconds)
pub const MIN_UPDATE_INTERVAL: i64 = 10;

/// Maximum allowed price deviation per update (basis points)
pub const MAX_PRICE_DEVIATION: u16 = 1000; // 10%

/// Stale oracle threshold (seconds)
pub const STALE_ORACLE_THRESHOLD: i64 = 300; // 5 minutes

/// Number of observations required for valid TWAP
pub const MIN_OBSERVATIONS_FOR_TWAP: u16 = 3;

/// TWAP window configurations (seconds, weight)
pub const TWAP_WINDOWS: [(u32, u16); 5] = [
    (300, 9500),    // 5 minutes - weight 95%
    (1800, 8500),   // 30 minutes - weight 85%
    (3600, 7500),   // 1 hour - weight 75%
    (14400, 6500),  // 4 hours - weight 65%
    (86400, 5000),  // 24 hours - weight 50%
];

// ============================================================================
// Oracle Account Structure
// ============================================================================

/// Unified oracle with manipulation protection and comprehensive metrics
#[account]
pub struct Oracle {
    /// Associated pool
    pub pool: Pubkey,
    
    /// Total number of observations (max 1024)
    pub observation_count: u16,
    
    /// Current index in ring buffer
    pub ring_index: u16,
    
    /// Last update timestamp
    pub last_update_timestamp: i64,
    
    /// Last update slot
    pub last_update_slot: u64,
    
    /// TWAP values for different windows (Q64.96 fixed point)
    pub twap_5min: u128,
    pub twap_30min: u128,
    pub twap_1hr: u128,
    pub twap_4hr: u128,
    pub twap_24hr: u128,
    
    /// Volatility metrics (basis points)
    pub volatility_5min: u16,
    pub volatility_30min: u16,
    pub volatility_1hr: u16,
    pub volatility_4hr: u16,
    pub volatility_24hr: u16,
    
    /// Manipulation detection
    pub manipulation_detected: bool,
    pub manipulation_count: u16,
    pub last_valid_price: u128,
    pub recovery_mode: bool,
    
    /// Oracle configuration
    pub config: OracleConfig,
    
    /// Oracle data account for full buffer storage
    pub data_account: Pubkey,
    
    /// Authority that can update oracle config
    pub authority: Pubkey,
    
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl Oracle {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        2 + 2 + // observation_count, ring_index
        8 + 8 + // timestamps
        16 * 5 + // TWAP values (5 windows)
        2 * 5 + // volatility values (5 windows)
        1 + 2 + 16 + 1 + // manipulation detection
        32 + // config
        32 + // data account
        32 + // authority
        32; // reserved

    /// Initialize oracle with pool
    pub fn initialize(
        &mut self,
        pool: Pubkey,
        data_account: Pubkey,
        authority: Pubkey,
        config: OracleConfig,
    ) -> Result<()> {
        self.pool = pool;
        self.data_account = data_account;
        self.authority = authority;
        self.config = config;
        self.observation_count = 0;
        self.ring_index = 0;
        self.last_update_timestamp = 0;
        self.last_update_slot = 0;
        self.manipulation_detected = false;
        self.manipulation_count = 0;
        self.recovery_mode = false;
        Ok(())
    }

    /// Add a new observation with manipulation protection
    pub fn add_observation(
        &mut self,
        sqrt_price: u128,
        tick: i32,
        timestamp: i64,
        oracle_data: &mut OracleData,
    ) -> Result<()> {
        // Validate timing
        require!(
            timestamp > self.last_update_timestamp,
            FeelsProtocolError::InvalidObservationTimestamp
        );
        
        let time_delta = timestamp - self.last_update_timestamp;
        require!(
            time_delta >= MIN_UPDATE_INTERVAL,
            FeelsProtocolError::OracleUpdateTooFrequent
        );

        // Check for price manipulation
        if self.observation_count > 0 && !self.recovery_mode {
            let price_change = calculate_price_change(self.last_valid_price, sqrt_price)?;
            if price_change > MAX_PRICE_DEVIATION {
                self.manipulation_detected = true;
                self.manipulation_count += 1;
                return Err(FeelsProtocolError::PriceManipulationDetected.into());
            }
        }

        // Store observation in data account
        let observation = PriceObservation {
            sqrt_price,
            tick,
            timestamp,
            slot: Clock::get()?.slot,
        };
        
        oracle_data.add_observation(self.ring_index, observation)?;

        // Update metadata
        self.last_update_timestamp = timestamp;
        self.last_update_slot = Clock::get()?.slot;
        self.last_valid_price = sqrt_price;
        
        // Update ring buffer index
        if self.observation_count < MAX_OBSERVATIONS {
            self.observation_count += 1;
        }
        self.ring_index = (self.ring_index + 1) % MAX_OBSERVATIONS;
        
        // Update TWAPs and volatility if we have enough observations
        if self.observation_count >= MIN_OBSERVATIONS_FOR_TWAP {
            self.update_twaps(oracle_data, timestamp)?;
            self.update_volatility(oracle_data)?;
        }
        
        // Clear manipulation flag after successful update
        if self.manipulation_detected && !self.recovery_mode {
            self.manipulation_detected = false;
        }
        
        Ok(())
    }

    /// Update TWAP values for all windows
    fn update_twaps(
        &mut self,
        oracle_data: &OracleData,
        current_time: i64,
    ) -> Result<()> {
        self.twap_5min = calculate_twap(oracle_data, self.observation_count, self.ring_index, current_time, 300)?;
        self.twap_30min = calculate_twap(oracle_data, self.observation_count, self.ring_index, current_time, 1800)?;
        self.twap_1hr = calculate_twap(oracle_data, self.observation_count, self.ring_index, current_time, 3600)?;
        self.twap_4hr = calculate_twap(oracle_data, self.observation_count, self.ring_index, current_time, 14400)?;
        self.twap_24hr = calculate_twap(oracle_data, self.observation_count, self.ring_index, current_time, 86400)?;
        Ok(())
    }

    /// Update volatility metrics
    fn update_volatility(
        &mut self,
        oracle_data: &OracleData,
    ) -> Result<()> {
        self.volatility_5min = calculate_volatility_safe(oracle_data, self.observation_count, self.ring_index, 300)?;
        self.volatility_30min = calculate_volatility_safe(oracle_data, self.observation_count, self.ring_index, 1800)?;
        self.volatility_1hr = calculate_volatility_safe(oracle_data, self.observation_count, self.ring_index, 3600)?;
        self.volatility_4hr = calculate_volatility_safe(oracle_data, self.observation_count, self.ring_index, 14400)?;
        self.volatility_24hr = calculate_volatility_safe(oracle_data, self.observation_count, self.ring_index, 86400)?;
        Ok(())
    }

    /// Check if oracle is stale
    pub fn is_stale(&self, current_time: i64) -> bool {
        current_time - self.last_update_timestamp > STALE_ORACLE_THRESHOLD
    }

    /// Get weighted TWAP across multiple windows
    pub fn get_weighted_twap(&self) -> u128 {
        let twaps = [
            self.twap_5min,
            self.twap_30min,
            self.twap_1hr,
            self.twap_4hr,
            self.twap_24hr,
        ];
        
        let mut weighted_sum: u128 = 0;
        let mut total_weight: u16 = 0;
        
        for (i, &twap) in twaps.iter().enumerate() {
            let weight = TWAP_WINDOWS[i].1;
            weighted_sum = weighted_sum.saturating_add(twap.saturating_mul(weight as u128));
            total_weight = total_weight.saturating_add(weight);
        }
        
        if total_weight > 0 {
            weighted_sum / total_weight as u128
        } else {
            self.last_valid_price
        }
    }

    /// Get weighted volatility across multiple windows
    pub fn get_weighted_volatility(&self) -> u16 {
        let volatilities = [
            self.volatility_5min,
            self.volatility_30min,
            self.volatility_1hr,
            self.volatility_4hr,
            self.volatility_24hr,
        ];
        
        let mut weighted_sum: u32 = 0;
        let mut total_weight: u16 = 0;
        
        for (i, &vol) in volatilities.iter().enumerate() {
            let weight = TWAP_WINDOWS[i].1;
            weighted_sum = weighted_sum.saturating_add((vol as u32).saturating_mul(weight as u32));
            total_weight = total_weight.saturating_add(weight);
        }
        
        if total_weight > 0 {
            (weighted_sum / total_weight as u32) as u16
        } else {
            0
        }
    }
}

// ============================================================================
// Oracle Data Storage
// ============================================================================

/// Separate account for storing observation buffer
#[account(zero_copy)]
pub struct OracleData {
    pub observations: [PriceObservation; MAX_OBSERVATIONS as usize],
}

impl OracleData {
    pub const SIZE: usize = 8 + // discriminator
        (32 * MAX_OBSERVATIONS as usize); // observations

    pub fn add_observation(
        &mut self,
        index: u16,
        observation: PriceObservation,
    ) -> Result<()> {
        require!(
            (index as usize) < MAX_OBSERVATIONS as usize,
            FeelsProtocolError::InvalidTickIndex
        );
        self.observations[index as usize] = observation;
        Ok(())
    }

    pub fn get_observation(&self, index: u16) -> Result<&PriceObservation> {
        require!(
            (index as usize) < MAX_OBSERVATIONS as usize,
            FeelsProtocolError::InvalidTickIndex
        );
        Ok(&self.observations[index as usize])
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Single price observation in the oracle
#[zero_copy]
#[derive(Default)]
pub struct PriceObservation {
    pub sqrt_price: u128,  // Q64.96 square root price
    pub tick: i32,         // The tick at this price
    pub timestamp: i64,    // Unix timestamp
    pub slot: u64,         // Solana slot number
}

/// Oracle configuration parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct OracleConfig {
    /// Whether oracle updates are permissioned
    pub permissioned: bool,
    /// Allowed price deviation per update (basis points)
    pub max_deviation: u16,
    /// Minimum update interval (seconds)
    pub min_interval: i64,
    /// Recovery mode threshold (consecutive manipulations)
    pub recovery_threshold: u16,
    /// Reserved for future use
    pub _reserved: [u8; 16],
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate price change in basis points
pub fn calculate_price_change(old_price: u128, new_price: u128) -> Result<u16> {
    if old_price == 0 {
        return Ok(10000); // 100% change if from 0
    }
    
    let change = if new_price > old_price {
        ((new_price - old_price) * 10000) / old_price
    } else {
        ((old_price - new_price) * 10000) / old_price
    };
    
    Ok(change.min(10000) as u16)
}

/// Calculate TWAP for a given window
pub fn calculate_twap(
    oracle_data: &OracleData,
    observation_count: u16,
    current_index: u16,
    current_time: i64,
    window_seconds: u32,
) -> Result<u128> {
    if observation_count == 0 {
        return Ok(0);
    }

    let mut weighted_sum: u256 = 0;
    let mut total_weight: u128 = 0;
    let window_start = current_time - window_seconds as i64;
    
    // Iterate through observations within the window
    for i in 0..observation_count.min(MAX_OBSERVATIONS) {
        let index = (current_index + MAX_OBSERVATIONS - i) % MAX_OBSERVATIONS;
        let obs = oracle_data.get_observation(index)?;
        
        if obs.timestamp < window_start {
            break; // Observations are ordered, so we can stop here
        }
        
        let time_weight = (obs.timestamp - window_start) as u128;
        weighted_sum += (obs.sqrt_price as u256) * (time_weight as u256);
        total_weight += time_weight;
    }
    
    if total_weight > 0 {
        Ok((weighted_sum / total_weight as u256) as u128)
    } else {
        // Return the most recent price if no observations in window
        let latest_index = (current_index + MAX_OBSERVATIONS - 1) % MAX_OBSERVATIONS;
        Ok(oracle_data.get_observation(latest_index)?.sqrt_price)
    }
}

/// Calculate volatility for a given window
pub fn calculate_volatility_safe(
    oracle_data: &OracleData,
    observation_count: u16,
    current_index: u16,
    window_seconds: u32,
) -> Result<u16> {
    if observation_count < 2 {
        return Ok(0);
    }

    let current_time = Clock::get()?.unix_timestamp;
    let window_start = current_time - window_seconds as i64;
    
    let mut returns: Vec<I64F64> = Vec::new();
    let mut prev_price: Option<u128> = None;
    
    // Collect price returns within the window
    for i in 0..observation_count.min(MAX_OBSERVATIONS) {
        let index = (current_index + MAX_OBSERVATIONS - i) % MAX_OBSERVATIONS;
        let obs = oracle_data.get_observation(index)?;
        
        if obs.timestamp < window_start {
            break;
        }
        
        if let Some(prev) = prev_price {
            if prev > 0 && obs.sqrt_price > 0 {
                // Calculate log return using fixed-point arithmetic
                let price_ratio = I64F64::from_num(obs.sqrt_price) / I64F64::from_num(prev);
                if price_ratio > I64F64::ZERO {
                    let log_return = ln_approximation(price_ratio)?;
                    returns.push(log_return);
                }
            }
        }
        prev_price = Some(obs.sqrt_price);
    }
    
    if returns.len() < 2 {
        return Ok(0);
    }
    
    // Calculate standard deviation of returns
    let mean = returns.iter().sum::<I64F64>() / I64F64::from_num(returns.len());
    let variance = returns.iter()
        .map(|r| {
            let diff = *r - mean;
            diff * diff
        })
        .sum::<I64F64>() / I64F64::from_num(returns.len());
    
    // Convert to basis points (multiply by 10000)
    let std_dev = sqrt_approximation(variance)?;
    let volatility_bps = (std_dev * I64F64::from_num(10000)).to_num::<u16>();
    
    Ok(volatility_bps.min(10000)) // Cap at 100%
}

/// Natural logarithm approximation for fixed-point
fn ln_approximation(x: I64F64) -> Result<I64F64> {
    require!(x > I64F64::ZERO, FeelsProtocolError::LogarithmUndefined);
    
    // Taylor series approximation around 1
    // ln(x) ≈ (x-1) - (x-1)²/2 + (x-1)³/3 - ...
    let x_minus_1 = x - I64F64::ONE;
    let x_minus_1_sq = x_minus_1 * x_minus_1;
    let x_minus_1_cu = x_minus_1_sq * x_minus_1;
    
    let result = x_minus_1 
        - x_minus_1_sq / I64F64::from_num(2)
        + x_minus_1_cu / I64F64::from_num(3);
    
    Ok(result)
}

/// Square root approximation for fixed-point
fn sqrt_approximation(x: I64F64) -> Result<I64F64> {
    if x == I64F64::ZERO {
        return Ok(I64F64::ZERO);
    }
    
    // Newton's method for square root
    let mut result = x;
    for _ in 0..10 { // 10 iterations is usually enough
        result = (result + x / result) / I64F64::from_num(2);
    }
    
    Ok(result)
}

// For 256-bit arithmetic
type u256 = u128; // Simplified for example, would use proper U256 in production