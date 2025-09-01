/// Simple TWAP oracle for internal price tracking.
/// Provides time-weighted average prices for the field commitment strategy.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Number of price observations to store
pub const OBSERVATION_BUFFER_SIZE: usize = 24; // 24 hours at 1 per hour

/// Default TWAP window in seconds
pub const DEFAULT_TWAP_WINDOW: i64 = 3600; // 1 hour

/// Minimum time between observations (seconds)
pub const MIN_OBSERVATION_INTERVAL: i64 = 60; // 1 minute

// ============================================================================
// TWAP Oracle Account
// ============================================================================

/// Simple TWAP oracle tracking internal pool prices
#[account(zero_copy)]
#[derive(Default)]
#[repr(C, packed)]
pub struct TwapOracle {
    /// Pool this oracle belongs to
    pub pool: Pubkey,
    
    /// Circular buffer of price observations
    pub observations: [PriceObservation; OBSERVATION_BUFFER_SIZE],
    
    /// Current write index in circular buffer
    pub write_index: u8,
    
    /// Number of valid observations
    pub observation_count: u8,
    
    /// Minimum TWAP window (seconds)
    pub min_twap_window: i64,
    
    /// Maximum TWAP window (seconds)
    pub max_twap_window: i64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

/// Single price observation
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct PriceObservation {
    /// Square root price at observation (Q64)
    pub sqrt_price: u128,
    
    /// Timestamp of observation
    pub timestamp: i64,
    
    /// Cumulative volume at observation (for volume weighting)
    pub cumulative_volume: u128,
}

impl TwapOracle {
    /// Record a new price observation
    pub fn observe_price(
        &mut self,
        sqrt_price: u128,
        volume: u128,
        timestamp: i64,
    ) -> Result<()> {
        // Check minimum interval
        if self.last_update > 0 && timestamp - self.last_update < MIN_OBSERVATION_INTERVAL {
            return Ok(()); // Skip too frequent updates
        }
        
        // Get previous cumulative volume
        let prev_volume = if self.observation_count > 0 {
            let prev_idx = if self.write_index == 0 {
                (OBSERVATION_BUFFER_SIZE - 1) as u8
            } else {
                self.write_index - 1
            };
            self.observations[prev_idx as usize].cumulative_volume
        } else {
            0
        };
        
        // Store new observation
        let obs = &mut self.observations[self.write_index as usize];
        obs.sqrt_price = sqrt_price;
        obs.timestamp = timestamp;
        obs.cumulative_volume = prev_volume.saturating_add(volume);
        
        // Update indices
        self.write_index = ((self.write_index as usize + 1) % OBSERVATION_BUFFER_SIZE) as u8;
        if self.observation_count < OBSERVATION_BUFFER_SIZE as u8 {
            self.observation_count += 1;
        }
        
        self.last_update = timestamp;
        
        Ok(())
    }
    
    /// Get TWAP for specified window
    pub fn get_twap(&self, window: i64, current_time: i64) -> Result<u128> {
        // Validate window
        require!(
            window >= self.min_twap_window && window <= self.max_twap_window,
            FeelsProtocolError::InvalidParameter {
                param: "TWAP window".to_string(),
                reason: "Outside allowed range".to_string()
            }
        );
        
        require!(
            self.observation_count > 0,
            FeelsProtocolError::InsufficientData {
                reason: "No price observations".to_string()
            }
        );
        
        let window_start = current_time - window;
        
        // Find observations within window
        let mut sum_price_time = 0u128;
        let mut total_time = 0i64;
        let mut found_start = false;
        
        // Iterate through observations from newest to oldest
        for i in 0..self.observation_count {
            let idx = if self.write_index >= i + 1 {
                self.write_index - i - 1
            } else {
                OBSERVATION_BUFFER_SIZE as u8 + self.write_index - i - 1
            } as usize;
            
            let obs = &self.observations[idx];
            
            // Skip if before window
            if obs.timestamp < window_start {
                if found_start {
                    // Use this observation as the start point
                    let time_in_window = current_time - window_start;
                    let price = sqrt_price_to_price(obs.sqrt_price)?;
                    sum_price_time = sum_price_time.saturating_add(
                        price.saturating_mul(time_in_window as u128)
                    );
                    total_time += time_in_window;
                }
                break;
            }
            
            found_start = true;
            
            // Calculate time weight
            let next_time = if i == 0 {
                current_time
            } else {
                let next_idx = if idx == OBSERVATION_BUFFER_SIZE - 1 {
                    0
                } else {
                    idx + 1
                };
                self.observations[next_idx].timestamp
            };
            
            let time_weight = (next_time - obs.timestamp.max(window_start)) as u128;
            let price = sqrt_price_to_price(obs.sqrt_price)?;
            
            sum_price_time = sum_price_time.saturating_add(
                price.saturating_mul(time_weight)
            );
            total_time += time_weight as i64;
        }
        
        require!(
            total_time > 0,
            FeelsProtocolError::InsufficientData {
                reason: "No observations in TWAP window".to_string()
            }
        );
        
        // Calculate TWAP
        let twap = sum_price_time.checked_div(total_time as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(twap)
    }
    
    /// Get most recent price
    pub fn get_current_price(&self) -> Result<u128> {
        require!(
            self.observation_count > 0,
            FeelsProtocolError::InsufficientData {
                reason: "No price observations".to_string()
            }
        );
        
        let last_idx = if self.write_index == 0 {
            (self.observation_count - 1) as usize
        } else {
            (self.write_index - 1) as usize
        };
        
        sqrt_price_to_price(self.observations[last_idx].sqrt_price)
    }
}

/// Convert sqrt price to price
fn sqrt_price_to_price(sqrt_price: u128) -> Result<u128> {
    // price = (sqrt_price)^2 / 2^128
    sqrt_price
        .checked_mul(sqrt_price)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 128)
        .ok_or(FeelsProtocolError::MathOverflow)
}

// ============================================================================
// Initialize TWAP Oracle
// ============================================================================

/// Initialize TWAP oracle parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeTwapParams {
    /// Minimum TWAP window (seconds)
    pub min_twap_window: i64,
    
    /// Maximum TWAP window (seconds)
    pub max_twap_window: i64,
}

impl Default for InitializeTwapParams {
    fn default() -> Self {
        Self {
            min_twap_window: 300,      // 5 minutes
            max_twap_window: 86400,    // 24 hours
        }
    }
}

// ============================================================================
// Size Constants
// ============================================================================

impl TwapOracle {
    pub const SIZE: usize = 8 +    // discriminator
        32 +                        // pool pubkey
        (16 + 8 + 16) * OBSERVATION_BUFFER_SIZE + // observations
        1 + 1 +                     // indices
        8 + 8 + 8 +                // windows and timestamp
        64;                         // reserved
}