/// Fees policy state for enforcing fee constraints on-chain.
/// Manages minimum fees, pool status, and rebate eligibility.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::constant::*;

// ============================================================================
// Fees Policy State
// ============================================================================

/// Protocol-wide fees policy configuration
#[account(zero_copy)]
#[derive(Debug)]
#[repr(C, packed)]
pub struct FeesPolicy {
    /// Authority that can update policy
    pub authority: Pubkey,
    
    /// Minimum base fee in basis points
    pub min_base_fee_bps: u64,
    
    /// Maximum base fee in basis points
    pub max_base_fee_bps: u64,
    
    /// Fee increase cap per update (basis points)
    pub max_fee_increase_bps: u64,
    
    /// Fee decrease cap per update (basis points)
    pub max_fee_decrease_bps: u64,
    
    /// Minimum time between fee updates (seconds)
    pub min_update_interval: i64,
    
    /// Spot price deviation threshold for pool disable (basis points)
    pub spot_disable_threshold_bps: u64,
    
    /// Time utilization threshold for pool disable (basis points)
    pub time_disable_threshold_bps: u64,
    
    /// Leverage imbalance threshold for pool disable (basis points)
    pub leverage_disable_threshold_bps: u64,
    
    /// Number of consecutive high stress periods before disable
    pub consecutive_stress_periods_for_disable: u8,
    
    /// Cool-down period after re-enabling (seconds)
    pub reenable_cooldown: i64,
    
    /// Maximum staleness before fallback mode (seconds)
    pub max_commitment_staleness: i64,
    
    /// Fallback fee in basis points
    pub fallback_fee_bps: u64,
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

impl Default for FeesPolicy {
    fn default() -> Self {
        Self {
            authority: Pubkey::default(),
            min_base_fee_bps: MIN_FEE_BPS,
            max_base_fee_bps: MAX_FEE_BPS,
            max_fee_increase_bps: 500,  // 5% max increase per update
            max_fee_decrease_bps: 300,   // 3% max decrease per update  
            min_update_interval: 300,    // 5 minutes
            spot_disable_threshold_bps: 9500,    // 95% deviation
            time_disable_threshold_bps: 9500,    // 95% utilization
            leverage_disable_threshold_bps: 9000, // 90% imbalance
            consecutive_stress_periods_for_disable: 3,
            reenable_cooldown: 3600,    // 1 hour
            max_commitment_staleness: 1800,  // 30 minutes
            fallback_fee_bps: 100,      // 1% fallback fee
            _reserved: [0u8; 128],
        }
    }
}

impl FeesPolicy {
    /// Size calculation for account allocation
    pub const SIZE: usize = 8 +   // discriminator
        32 +                       // authority
        8 * 7 +                    // fee parameters
        1 +                        // consecutive periods
        8 * 2 +                    // cooldown and staleness
        8 +                        // fallback fee
        128;                       // reserved

    /// Validate fee update against policy constraints
    pub fn validate_fee_update(
        &self,
        current_fee: u64,
        new_fee: u64,
        last_update_ts: i64,
        current_ts: i64,
    ) -> Result<()> {
        // Check update interval
        let time_since_update = current_ts - last_update_ts;
        require!(
            time_since_update >= self.min_update_interval,
            FeelsProtocolError::UpdateTooFrequent
        );
        
        // Check absolute bounds
        require!(
            new_fee >= self.min_base_fee_bps,
            FeelsProtocolError::FeeBelowMinimum
        );
        require!(
            new_fee <= self.max_base_fee_bps,
            FeelsProtocolError::FeeAboveMaximum
        );
        
        // Check rate of change limits
        if new_fee > current_fee {
            let increase = new_fee - current_fee;
            let max_increase = (current_fee * self.max_fee_increase_bps) / BPS_DENOMINATOR;
            require!(
                increase <= max_increase,
                FeelsProtocolError::FeeIncreaseTooLarge
            );
        } else if new_fee < current_fee {
            let decrease = current_fee - new_fee;
            let max_decrease = (current_fee * self.max_fee_decrease_bps) / BPS_DENOMINATOR;
            require!(
                decrease <= max_decrease,
                FeelsProtocolError::FeeDecreaseTooLarge
            );
        }
        
        Ok(())
    }
    
    /// Check if pool should be disabled based on stress levels
    pub fn should_disable_pool(
        &self,
        spot_stress: u64,
        time_stress: u64,
        leverage_stress: u64,
    ) -> bool {
        spot_stress > self.spot_disable_threshold_bps ||
        time_stress > self.time_disable_threshold_bps ||
        leverage_stress > self.leverage_disable_threshold_bps
    }
    
    /// Get appropriate fee based on commitment freshness
    pub fn get_active_fee(
        &self,
        base_fee: u64,
        commitment_ts: i64,
        current_ts: i64,
    ) -> u64 {
        let staleness = current_ts - commitment_ts;
        if staleness > self.max_commitment_staleness {
            self.fallback_fee_bps
        } else {
            base_fee
        }
    }
}

// ============================================================================
// Pool Status Tracking
// ============================================================================

/// Pool-specific status for fee enforcement
#[account(zero_copy)]
#[derive(Debug)]
#[repr(C, packed)]
pub struct PoolStatus {
    /// Pool this status belongs to
    pub pool: Pubkey,
    
    /// Current operational status (0=Normal, 1=Warning, 2=Disabled, 3=Cooldown)
    pub status: u8,
    
    /// Last fee update timestamp
    pub last_fee_update_ts: i64,
    
    /// Current base fee (cached from field commitment)
    pub current_base_fee_bps: u64,
    
    /// Consecutive high stress periods
    pub consecutive_stress_periods: u8,
    
    /// Last stress measurement timestamp
    pub last_stress_check_ts: i64,
    
    /// Pool disabled timestamp (0 if not disabled)
    pub disabled_at_ts: i64,
    
    /// Pool re-enabled timestamp (0 if never re-enabled)
    pub reenabled_at_ts: i64,
    
    /// Total time disabled (cumulative seconds)
    pub total_disabled_time: u64,
    
    /// Number of times disabled
    pub disable_count: u32,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

/// Pool operational status enum - kept for documentation purposes
/// Note: The actual status field uses u8 values directly
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum PoolOperationalStatus {
    /// Normal operation
    Normal = 0,
    /// High stress warning
    Warning = 1,
    /// Disabled due to extreme conditions
    Disabled = 2,
    /// In cooldown after re-enabling
    Cooldown = 3,
}

impl Default for PoolStatus {
    fn default() -> Self {
        Self {
            pool: Pubkey::default(),
            status: 0, // Normal
            last_fee_update_ts: 0,
            current_base_fee_bps: MIN_FEE_BPS,
            consecutive_stress_periods: 0,
            last_stress_check_ts: 0,
            disabled_at_ts: 0,
            reenabled_at_ts: 0,
            total_disabled_time: 0,
            disable_count: 0,
            _reserved: [0u8; 64],
        }
    }
}

impl PoolStatus {
    /// Size calculation for account allocation
    pub const SIZE: usize = 8 +   // discriminator
        32 +                       // pool
        1 +                        // status enum
        8 * 2 +                    // timestamps
        8 +                        // current fee
        1 +                        // consecutive periods
        8 * 3 +                    // more timestamps
        8 +                        // total disabled time
        4 +                        // disable count
        64;                        // reserved

    /// Update stress tracking
    pub fn update_stress_tracking(
        &mut self,
        policy: &FeesPolicy,
        is_high_stress: bool,
        current_ts: i64,
    ) -> Result<()> {
        self.last_stress_check_ts = current_ts;
        
        if is_high_stress {
            self.consecutive_stress_periods = self.consecutive_stress_periods.saturating_add(1);
            
            // Check if we should disable
            if self.consecutive_stress_periods >= policy.consecutive_stress_periods_for_disable 
                && self.status == 0 { // Normal
                self.status = 2; // Disabled
                self.disabled_at_ts = current_ts;
                self.disable_count = self.disable_count.saturating_add(1);
            } else if self.consecutive_stress_periods > 1 
                && self.status == 0 { // Normal
                self.status = 1; // Warning
            }
        } else {
            // Reset consecutive count on normal stress
            self.consecutive_stress_periods = 0;
            
            // Handle status transitions
            match self.status {
                1 => { // Warning
                    self.status = 0; // Normal
                }
                2 => { // Disabled
                    // Stay disabled until explicitly re-enabled
                }
                3 => { // Cooldown
                    // Check if cooldown period has passed
                    if current_ts - self.reenabled_at_ts >= policy.reenable_cooldown {
                        self.status = 0; // Normal
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Re-enable a disabled pool
    pub fn reenable(&mut self, current_ts: i64) -> Result<()> {
        require!(
            self.status == 2, // Disabled
            FeelsProtocolError::InvalidPoolStatus
        );
        
        // Update disabled time tracking
        if self.disabled_at_ts > 0 {
            let disabled_duration = (current_ts - self.disabled_at_ts) as u64;
            self.total_disabled_time = self.total_disabled_time.saturating_add(disabled_duration);
        }
        
        self.status = 3; // Cooldown
        self.reenabled_at_ts = current_ts;
        self.consecutive_stress_periods = 0;
        
        Ok(())
    }
    
    /// Check if pool can accept new orders
    pub fn can_accept_orders(&self) -> bool {
        matches!(self.status, 0 | 1) // Normal | Warning
    }
    
    /// Get effective fee multiplier based on status
    pub fn get_fee_multiplier(&self) -> u64 {
        match self.status {
            0 => BPS_DENOMINATOR,      // Normal
            1 => 15000,                // Warning - 1.5x multiplier
            2 => 0,                    // Disabled - No trading allowed
            3 => 12000,                // Cooldown - 1.2x multiplier
            _ => BPS_DENOMINATOR,      // Default to normal
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate stress level from components
pub fn calculate_combined_stress(
    spot_stress: u64,
    time_stress: u64,
    leverage_stress: u64,
    weights: (u32, u32, u32),
) -> u64 {
    let (w_s, w_t, w_l) = weights;
    let total_weight = (w_s + w_t + w_l) as u64;
    
    if total_weight == 0 {
        return 0;
    }
    
    let weighted_stress = (spot_stress * w_s as u64 + 
                          time_stress * w_t as u64 + 
                          leverage_stress * w_l as u64) / total_weight;
    
    weighted_stress.min(BPS_DENOMINATOR)
}

/// Check if fee should trigger rebate
pub fn qualifies_for_rebate(
    base_fee: u64,
    actual_fee: u64,
    pool_status: &PoolStatus,
) -> bool {
    // Rebates only available in normal operation
    if pool_status.status != 0 { // Normal
        return false;
    }
    
    // Must pay more than base fee to qualify
    actual_fee > base_fee
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_validation() {
        let policy = FeesPolicy::default();
        
        // Valid update
        assert!(policy.validate_fee_update(100, 105, 0, 300).is_ok());
        
        // Too frequent
        assert!(policy.validate_fee_update(100, 105, 0, 299).is_err());
        
        // Too large increase
        assert!(policy.validate_fee_update(100, 200, 0, 300).is_err());
        
        // Below minimum
        assert!(policy.validate_fee_update(100, 0, 0, 300).is_err());
    }

    #[test]
    fn test_pool_status_transitions() {
        let policy = FeesPolicy::default();
        let mut status = PoolStatus::default();
        
        // First high stress - goes to warning
        status.update_stress_tracking(&policy, true, 100).unwrap();
        assert_eq!(status.consecutive_stress_periods, 1);
        assert_eq!(status.status, 0); // Normal
        
        // Second high stress - warning
        status.update_stress_tracking(&policy, true, 200).unwrap();
        assert_eq!(status.consecutive_stress_periods, 2);
        assert_eq!(status.status, 1); // Warning
        
        // Third high stress - disabled
        status.update_stress_tracking(&policy, true, 300).unwrap();
        assert_eq!(status.consecutive_stress_periods, 3);
        assert_eq!(status.status, 2); // Disabled
        assert_eq!(status.disabled_at_ts, 300);
        
        // Normal stress doesn't auto-reenable
        status.update_stress_tracking(&policy, false, 400).unwrap();
        assert_eq!(status.status, 2); // Disabled
        assert_eq!(status.consecutive_stress_periods, 0);
    }

    #[test]
    fn test_combined_stress() {
        let stress = calculate_combined_stress(
            5000,  // 50% spot stress
            3000,  // 30% time stress
            2000,  // 20% leverage stress
            (5000, 3000, 2000),  // weights
        );
        
        // (5000*5000 + 3000*3000 + 2000*2000) / 10000 = 3800
        assert_eq!(stress, 3800);
    }
}