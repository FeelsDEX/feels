/// Unified lending metrics tracking both flash loans and utilization rates.
/// Monitors all lending activity including flash loans, borrows, and utilization patterns
/// to provide comprehensive risk assessment and dynamic fee adjustments.
/// Combines short-term signals (flash loans) with long-term metrics (utilization).

use anchor_lang::prelude::*;
use crate::state::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Number of observations in the time-weighted buffer
pub const OBSERVATION_BUFFER_SIZE: usize = 144; // 144 x 5 min = 12 hours

/// Utilization rate precision (10000 = 100%)
pub const UTILIZATION_PRECISION: u16 = 10000;

/// Target utilization rate for optimal interest rates
pub const TARGET_UTILIZATION: u16 = 8000; // 80%

// ============================================================================
// Lending Metrics Account
// ============================================================================

/// Comprehensive lending metrics for both flash loans and utilization
#[account]
pub struct LendingMetrics {
    /// Associated pool
    pub pool: Pubkey,
    
    // ========== Utilization Metrics ==========
    
    /// Total amount available for borrowing
    pub total_supplied: u128,
    
    /// Total amount currently borrowed
    pub total_borrowed: u128,
    
    /// Current utilization rate (basis points)
    pub utilization_rate: u16,
    
    /// Time-weighted average utilization rates
    pub utilization_1h: u16,
    pub utilization_4h: u16,
    pub utilization_24h: u16,
    
    /// Interest rate metrics (basis points per year)
    pub current_borrow_rate: u16,
    pub current_supply_rate: u16,
    pub avg_borrow_rate_24h: u16,
    
    // ========== Flash Loan Metrics ==========
    
    /// Cumulative flash loan volume
    pub flash_volume_total: u128,
    pub flash_count_total: u64,
    
    /// Recent flash loan activity
    pub flash_volume_5m: u64,
    pub flash_volume_1h: u64,
    pub flash_count_5m: u32,
    pub flash_count_1h: u32,
    
    /// Flash loan burst detection
    pub flash_burst_detected: bool,
    pub flash_burst_magnitude: u64,
    pub flash_burst_start: i64,
    
    // ========== Combined Risk Metrics ==========
    
    /// Overall lending stress indicator (0-10000 basis points)
    pub lending_stress_score: u16,
    
    /// Utilization velocity (rate of change)
    pub utilization_velocity: i16, // Can be negative
    
    /// Time-weighted observations buffer
    pub last_observation_slot: u64,
    pub last_observation_time: i64,
    pub observation_index: u16,
    
    /// Configuration
    pub min_update_interval: i64, // Minimum seconds between updates
    pub flash_burst_threshold: u64, // Volume threshold for burst detection
    
    pub _reserved: [u8; 64],
}

impl LendingMetrics {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        16 + 16 + 2 + // utilization basic
        2 + 2 + 2 + // utilization TWA
        2 + 2 + 2 + // interest rates
        16 + 8 + // flash totals
        8 + 8 + 4 + 4 + // flash recent
        1 + 8 + 8 + // flash burst
        2 + 2 + // combined metrics
        8 + 8 + 2 + // observation tracking
        8 + 8 + // configuration
        64; // reserved
    
    /// Initialize new lending metrics
    pub fn initialize(
        &mut self, 
        pool: Pubkey,
        min_update_interval: i64,
        flash_burst_threshold: u64,
    ) -> Result<()> {
        self.pool = pool;
        self.min_update_interval = min_update_interval;
        self.flash_burst_threshold = flash_burst_threshold;
        self.utilization_rate = 0;
        self.lending_stress_score = 0;
        Ok(())
    }
    
    // ========================================================================
    // Utilization Updates
    // ========================================================================
    
    /// Update supply amount (deposit or withdraw)
    pub fn update_supply(
        &mut self,
        amount_delta: i128, // Positive for deposit, negative for withdraw
        current_slot: u64,
        current_time: i64,
    ) -> Result<()> {
        // Update total supplied
        if amount_delta >= 0 {
            self.total_supplied = self.total_supplied
                .checked_add(amount_delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            self.total_supplied = self.total_supplied
                .checked_sub((-amount_delta) as u128)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
        }
        
        self.update_utilization_metrics(current_slot, current_time)?;
        Ok(())
    }
    
    /// Update borrow amount (borrow or repay)
    pub fn update_borrow(
        &mut self,
        amount_delta: i128, // Positive for borrow, negative for repay
        current_slot: u64,
        current_time: i64,
    ) -> Result<()> {
        // Update total borrowed
        if amount_delta >= 0 {
            self.total_borrowed = self.total_borrowed
                .checked_add(amount_delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            self.total_borrowed = self.total_borrowed
                .checked_sub((-amount_delta) as u128)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
        }
        
        self.update_utilization_metrics(current_slot, current_time)?;
        Ok(())
    }
    
    /// Calculate and update utilization metrics
    fn update_utilization_metrics(
        &mut self,
        current_slot: u64,
        current_time: i64,
    ) -> Result<()> {
        // Calculate current utilization
        if self.total_supplied == 0 {
            self.utilization_rate = 0;
        } else {
            self.utilization_rate = ((self.total_borrowed as u128 * UTILIZATION_PRECISION as u128) 
                / self.total_supplied as u128)
                .min(UTILIZATION_PRECISION as u128) as u16;
        }
        
        // Calculate velocity if we have a previous observation
        if self.last_observation_time > 0 {
            let time_delta = current_time.saturating_sub(self.last_observation_time).max(1);
            let old_util = self.utilization_1h; // Use 1h as baseline
            let util_change = self.utilization_rate as i32 - old_util as i32;
            self.utilization_velocity = (util_change / time_delta.max(60) as i32) as i16;
        }
        
        // Update time-weighted averages (simplified - in production would use buffer)
        self.utilization_1h = self.exponential_average(self.utilization_1h, self.utilization_rate, 12);
        self.utilization_4h = self.exponential_average(self.utilization_4h, self.utilization_rate, 48);
        self.utilization_24h = self.exponential_average(self.utilization_24h, self.utilization_rate, 288);
        
        // Update interest rates based on utilization
        self.update_interest_rates()?;
        
        // Update combined stress score
        self.update_stress_score()?;
        
        // Update observation tracking
        self.last_observation_slot = current_slot;
        self.last_observation_time = current_time;
        
        Ok(())
    }
    
    /// Update interest rates based on utilization curve
    fn update_interest_rates(&mut self) -> Result<()> {
        // Simple interest rate model
        // Below target: gradual increase
        // Above target: steep increase
        
        let base_rate = 200u16; // 2% base rate
        
        if self.utilization_rate <= TARGET_UTILIZATION {
            // Linear increase up to target
            let utilization_ratio = (self.utilization_rate as u32 * 10000) / TARGET_UTILIZATION as u32;
            self.current_borrow_rate = base_rate + ((utilization_ratio * 800) / 10000) as u16; // +8% at target
        } else {
            // Exponential increase above target
            let excess_utilization = self.utilization_rate - TARGET_UTILIZATION;
            let excess_ratio = (excess_utilization as u32 * 10000) / (UTILIZATION_PRECISION - TARGET_UTILIZATION) as u32;
            self.current_borrow_rate = base_rate + 800 + ((excess_ratio * 4000) / 10000) as u16; // +40% at 100%
        }
        
        // Supply rate = borrow rate * utilization * (1 - protocol fee)
        let protocol_fee = 1000u16; // 10%
        self.current_supply_rate = ((self.current_borrow_rate as u32 * self.utilization_rate as u32 * (10000 - protocol_fee) as u32) 
            / (10000 * 10000)) as u16;
        
        // Update 24h average
        self.avg_borrow_rate_24h = self.exponential_average(self.avg_borrow_rate_24h, self.current_borrow_rate, 288);
        
        Ok(())
    }
    
    // ========================================================================
    // Flash Loan Updates
    // ========================================================================
    
    /// Record a flash loan event
    pub fn record_flash_loan(
        &mut self,
        volume: u64,
        _current_slot: u64,
        current_time: i64,
    ) -> Result<()> {
        // Check minimum update interval
        if current_time.saturating_sub(self.last_observation_time) < self.min_update_interval {
            return Ok(());
        }
        
        // Update cumulative metrics
        self.flash_volume_total = self.flash_volume_total
            .checked_add(volume as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        self.flash_count_total = self.flash_count_total
            .checked_add(1)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        // Update recent metrics (simplified - TODO: in production would use time-weighted buffer)
        self.flash_volume_5m = self.exponential_average_u64(self.flash_volume_5m, volume, 1);
        self.flash_volume_1h = self.exponential_average_u64(self.flash_volume_1h, volume, 12);
        self.flash_count_5m = (self.flash_count_5m + 1).min(100);
        self.flash_count_1h = (self.flash_count_1h + 1).min(1000);
        
        // Detect flash loan burst
        self.detect_flash_burst(volume, current_time)?;
        
        // Update combined stress score
        self.update_stress_score()?;
        
        Ok(())
    }
    
    /// Detect flash loan volume bursts
    fn detect_flash_burst(
        &mut self,
        current_volume: u64,
        current_time: i64,
    ) -> Result<()> {
        let volume_spike = current_volume > self.flash_burst_threshold ||
                          self.flash_volume_5m > self.flash_volume_1h.saturating_mul(3);
        
        if volume_spike && !self.flash_burst_detected {
            self.flash_burst_detected = true;
            self.flash_burst_start = current_time;
            self.flash_burst_magnitude = current_volume.max(self.flash_volume_5m);
        } else if !volume_spike && self.flash_burst_detected &&
                  current_time - self.flash_burst_start > 300 { // 5 min cooldown
            self.flash_burst_detected = false;
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Combined Metrics
    // ========================================================================
    
    /// Update the combined lending stress score
    fn update_stress_score(&mut self) -> Result<()> {
        let mut stress = 0u32;
        
        // Utilization stress (0-4000 based on distance from target)
        if self.utilization_rate > TARGET_UTILIZATION {
            let excess = self.utilization_rate - TARGET_UTILIZATION;
            stress += (excess as u32 * 4000) / (UTILIZATION_PRECISION - TARGET_UTILIZATION) as u32;
        }
        
        // Velocity stress (0-2000 based on rate of change)
        if self.utilization_velocity > 0 {
            stress += (self.utilization_velocity as u32).min(2000);
        }
        
        // Flash loan stress (0-2000)
        if self.flash_burst_detected {
            stress += 1000;
        }
        if self.flash_volume_5m > 0 {
            let flash_ratio = (self.flash_volume_5m as u128 * 1000) / self.total_supplied.max(1);
            stress += flash_ratio.min(1000) as u32;
        }
        
        // Interest rate stress (0-2000 based on rates)
        if self.current_borrow_rate > 1000 { // >10% APR
            stress += ((self.current_borrow_rate - 1000) as u32).min(2000);
        }
        
        self.lending_stress_score = stress.min(10000) as u16;
        Ok(())
    }
    
    // ========================================================================
    // Helper Functions
    // ========================================================================
    
    /// Exponential moving average helper
    fn exponential_average(&self, old_value: u16, new_value: u16, periods: u32) -> u16 {
        let alpha = 10000u32 / periods.max(1);
        let weighted_new = (new_value as u32 * alpha) / 10000;
        let weighted_old = (old_value as u32 * (10000 - alpha)) / 10000;
        (weighted_new + weighted_old) as u16
    }
    
    /// Exponential moving average for u64
    fn exponential_average_u64(&self, old_value: u64, new_value: u64, periods: u32) -> u64 {
        let alpha = 10000u32 / periods.max(1);
        let weighted_new = (new_value as u128 * alpha as u128) / 10000;
        let weighted_old = (old_value as u128 * (10000 - alpha) as u128) / 10000;
        (weighted_new + weighted_old) as u64
    }
    
    /// Get risk-adjusted fee multiplier based on lending stress
    pub fn get_fee_multiplier(&self) -> u16 {
        // Base multiplier 10000 (1x)
        // Max multiplier 20000 (2x) at max stress
        10000 + ((self.lending_stress_score as u32 * 10000) / 10000) as u16
    }
    
    /// Check if lending is in a healthy state
    pub fn is_healthy(&self) -> bool {
        self.lending_stress_score < 5000 && // Below 50% stress
        self.utilization_rate < 9500 && // Below 95% utilization
        !self.flash_burst_detected
    }
    
    /// Check if pool has high activity
    pub fn is_high_activity(&self) -> bool {
        // High activity if:
        // - High utilization rate (> 70%)
        // - High flash loan volume
        // - High stress score
        self.utilization_rate > 7000 ||
        self.flash_volume_1h > self.total_supplied.saturating_div(240) as u64 || // Estimate 24h from 1h > 10% of supply
        self.lending_stress_score > 7500 // > 75% stress
    }
}