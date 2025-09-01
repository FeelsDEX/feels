/// Virtual rebasing system with lazy evaluation for yield accrual and funding rates.
/// Implements the mathematical framework where all yield/funding is delivered via
/// multiplicative rebasing while preserving invariants.
use anchor_lang::prelude::*;
use crate::utils::math::{safe_mul_div_u128, safe_mul_div_u64};

// ============================================================================
// Core Rebase State
// ============================================================================

/// Global rebase accumulator tracking all multiplicative factors
#[account(zero_copy)]
#[derive(Default)]
pub struct RebaseAccumulator {
    /// Current global rebase index for token A (scaled by 2^64)
    pub index_a: u128,
    
    /// Current global rebase index for token B (scaled by 2^64)
    pub index_b: u128,
    
    /// Timestamp of last rebase update
    pub last_update: i64,
    
    /// Cumulative funding rate index for longs (scaled by 2^64)
    pub funding_index_long: u128,
    
    /// Cumulative funding rate index for shorts (scaled by 2^64)
    pub funding_index_short: u128,
    
    /// Current supply rate for token A (basis points per year)
    pub supply_rate_a: u64,
    
    /// Current supply rate for token B (basis points per year)
    pub supply_rate_b: u64,
    
    /// Current funding rate (positive = longs pay shorts, basis points per year)
    pub funding_rate: i64,
    
    /// Pool weights for invariant preservation
    pub weight_a: u32,
    pub weight_b: u32,
    pub weight_long: u32,
    pub weight_short: u32,
    
    /// Reserved for future use
    pub _padding: [u8; 32],
}

/// Position checkpoint for lazy evaluation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct RebaseCheckpoint {
    /// Rebase index for token A at position creation/update
    pub index_a_checkpoint: u128,
    
    /// Rebase index for token B at position creation/update
    pub index_b_checkpoint: u128,
    
    /// Funding index checkpoint for leveraged positions
    pub funding_index_checkpoint: u128,
    
    /// Timestamp of checkpoint
    pub checkpoint_timestamp: i64,
}

// ============================================================================
// Constants
// ============================================================================

/// Scale factor for rebase indices (2^64)
pub const REBASE_INDEX_SCALE: u128 = 1 << 64;

/// Seconds per year for rate calculations
pub const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

// ============================================================================
// Implementation
// ============================================================================

impl RebaseAccumulator {
    /// Initialize a new rebase accumulator
    pub fn initialize(&mut self, weight_a: u32, weight_b: u32, weight_long: u32, weight_short: u32) {
        self.index_a = REBASE_INDEX_SCALE;
        self.index_b = REBASE_INDEX_SCALE;
        self.funding_index_long = REBASE_INDEX_SCALE;
        self.funding_index_short = REBASE_INDEX_SCALE;
        self.last_update = Clock::get().unwrap().unix_timestamp;
        self.weight_a = weight_a;
        self.weight_b = weight_b;
        self.weight_long = weight_long;
        self.weight_short = weight_short;
    }
    
    /// Update rebase indices based on elapsed time and current rates
    pub fn update_indices(&mut self, current_timestamp: i64) -> Result<()> {
        let time_elapsed = current_timestamp.saturating_sub(self.last_update);
        if time_elapsed <= 0 {
            return Ok(());
        }
        
        // Update yield indices for tokens A and B
        if self.supply_rate_a > 0 {
            let growth_a = self.calculate_growth_factor(self.supply_rate_a, time_elapsed)?;
            self.index_a = safe_mul_div_u128(self.index_a, growth_a, REBASE_INDEX_SCALE)?;
        }
        
        if self.supply_rate_b > 0 {
            let growth_b = self.calculate_growth_factor(self.supply_rate_b, time_elapsed)?;
            self.index_b = safe_mul_div_u128(self.index_b, growth_b, REBASE_INDEX_SCALE)?;
        }
        
        // Update funding indices for leveraged positions
        if self.funding_rate != 0 {
            let (growth_long, growth_short) = self.calculate_funding_growth(time_elapsed)?;
            self.funding_index_long = safe_mul_div_u128(
                self.funding_index_long, 
                growth_long, 
                REBASE_INDEX_SCALE
            )?;
            self.funding_index_short = safe_mul_div_u128(
                self.funding_index_short, 
                growth_short, 
                REBASE_INDEX_SCALE
            )?;
        }
        
        self.last_update = current_timestamp;
        Ok(())
    }
    
    /// Calculate growth factor for a given rate and time period
    fn calculate_growth_factor(&self, rate_bps: u64, time_elapsed: i64) -> Result<u128> {
        // growth = 1 + (rate * time / seconds_per_year)
        // Using Taylor approximation for small values: e^x ≈ 1 + x
        let rate_fraction = safe_mul_div_u128(
            rate_bps as u128 * time_elapsed as u128,
            REBASE_INDEX_SCALE,
            (BPS_DENOMINATOR as u128) * (SECONDS_PER_YEAR as u128)
        )?;
        
        Ok(REBASE_INDEX_SCALE + rate_fraction)
    }
    
    /// Calculate funding growth factors maintaining invariant
    fn calculate_funding_growth(&self, time_elapsed: i64) -> Result<(u128, u128)> {
        let funding_fraction = safe_mul_div_u128(
            self.funding_rate.abs() as u128 * time_elapsed as u128,
            REBASE_INDEX_SCALE,
            (BPS_DENOMINATOR as u128) * (SECONDS_PER_YEAR as u128)
        )?;
        
        // Maintain invariant: g_long^w_long * g_short^w_short = 1
        // If funding_rate > 0: longs pay shorts
        // If funding_rate < 0: shorts pay longs
        
        let (growth_long, growth_short) = if self.funding_rate > 0 {
            // Longs pay: g_long < 1, g_short > 1
            let g_long = REBASE_INDEX_SCALE.saturating_sub(funding_fraction);
            // Calculate g_short to maintain invariant
            let g_short = self.calculate_invariant_preserving_factor(
                g_long,
                self.weight_long,
                self.weight_short
            )?;
            (g_long, g_short)
        } else {
            // Shorts pay: g_short < 1, g_long > 1
            let g_short = REBASE_INDEX_SCALE.saturating_sub(funding_fraction);
            let g_long = self.calculate_invariant_preserving_factor(
                g_short,
                self.weight_short,
                self.weight_long
            )?;
            (g_long, g_short)
        };
        
        Ok((growth_long, growth_short))
    }
    
    /// Calculate the factor needed to preserve invariant
    /// Given g1 and weights w1, w2, find g2 such that g1^w1 * g2^w2 = 1
    fn calculate_invariant_preserving_factor(
        &self,
        g1: u128,
        w1: u32,
        w2: u32,
    ) -> Result<u128> {
        // g2 = (1/g1)^(w1/w2) = (SCALE^2/g1)^(w1/w2)
        // Using approximation for small deviations from 1
        
        let deviation = if g1 > REBASE_INDEX_SCALE {
            g1 - REBASE_INDEX_SCALE
        } else {
            REBASE_INDEX_SCALE - g1
        };
        
        let scaled_deviation = safe_mul_div_u128(
            deviation,
            w1 as u128,
            w2 as u128
        )?;
        
        if g1 > REBASE_INDEX_SCALE {
            Ok(REBASE_INDEX_SCALE.saturating_sub(scaled_deviation))
        } else {
            Ok(REBASE_INDEX_SCALE + scaled_deviation)
        }
    }
    
    /// Set supply rates for yield accrual
    pub fn set_supply_rates(&mut self, rate_a: u64, rate_b: u64) {
        self.supply_rate_a = rate_a;
        self.supply_rate_b = rate_b;
    }
    
    /// Set funding rate
    pub fn set_funding_rate(&mut self, funding_rate: i64) {
        self.funding_rate = funding_rate;
    }
}

// ============================================================================
// Position Rebasing
// ============================================================================

/// Apply lazy rebase evaluation to a position
pub fn apply_position_rebase(
    position_value_a: u64,
    position_value_b: u64,
    checkpoint: &RebaseCheckpoint,
    accumulator: &RebaseAccumulator,
    is_leveraged: bool,
    is_long: bool,
) -> Result<(u64, u64)> {
    // Calculate rebase multipliers
    let rebase_a = safe_mul_div_u128(
        position_value_a as u128,
        accumulator.index_a,
        checkpoint.index_a_checkpoint
    )? as u64;
    
    let rebase_b = safe_mul_div_u128(
        position_value_b as u128,
        accumulator.index_b,
        checkpoint.index_b_checkpoint
    )? as u64;
    
    // Apply funding if leveraged
    let (final_a, final_b) = if is_leveraged {
        let funding_index = if is_long {
            accumulator.funding_index_long
        } else {
            accumulator.funding_index_short
        };
        
        let funding_multiplier = safe_mul_div_u128(
            REBASE_INDEX_SCALE,
            funding_index,
            checkpoint.funding_index_checkpoint
        )?;
        
        let funded_a = safe_mul_div_u128(
            rebase_a as u128,
            funding_multiplier,
            REBASE_INDEX_SCALE
        )? as u64;
        
        let funded_b = safe_mul_div_u128(
            rebase_b as u128,
            funding_multiplier,
            REBASE_INDEX_SCALE
        )? as u64;
        
        (funded_a, funded_b)
    } else {
        (rebase_a, rebase_b)
    };
    
    Ok((final_a, final_b))
}

/// Create a new checkpoint at current indices
pub fn create_checkpoint(accumulator: &RebaseAccumulator, is_long: bool) -> RebaseCheckpoint {
    RebaseCheckpoint {
        index_a_checkpoint: accumulator.index_a,
        index_b_checkpoint: accumulator.index_b,
        funding_index_checkpoint: if is_long {
            accumulator.funding_index_long
        } else {
            accumulator.funding_index_short
        },
        checkpoint_timestamp: accumulator.last_update,
    }
}

// ============================================================================
// Rate Calculations
// ============================================================================

/// Calculate supply rate based on utilization
pub fn calculate_supply_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
    reserve_factor_bps: u64,
) -> u64 {
    // supply_rate = borrow_rate * utilization * (1 - reserve_factor)
    let borrow_rate = calculate_borrow_rate(utilization_bps, base_rate_bps);
    
    safe_mul_div_u64(
        safe_mul_div_u64(
            borrow_rate,
            utilization_bps,
            BPS_DENOMINATOR
        ).unwrap_or(0),
        BPS_DENOMINATOR - reserve_factor_bps,
        BPS_DENOMINATOR
    ).unwrap_or(0)
}

/// Calculate borrow rate based on utilization
pub fn calculate_borrow_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
) -> u64 {
    // Simple linear model: rate = base + utilization * slope
    // At 80% utilization, rate = 4x base rate
    let slope = base_rate_bps * 3 / 80; // 3x increase over 80%
    
    base_rate_bps + safe_mul_div_u64(
        utilization_bps,
        slope,
        100 // Convert from percentage
    ).unwrap_or(0)
}

/// Calculate funding rate based on long/short imbalance
pub fn calculate_funding_rate(
    long_value: u128,
    short_value: u128,
    max_funding_rate_bps: u64,
) -> i64 {
    let total = long_value.saturating_add(short_value);
    if total == 0 {
        return 0;
    }
    
    // Imbalance ratio: positive if more longs, negative if more shorts
    let long_ratio = safe_mul_div_u128(
        long_value,
        BPS_DENOMINATOR as u128,
        total
    ).unwrap_or(5000) as i64; // Default to 50%
    
    let imbalance = long_ratio - 5000; // -5000 to +5000
    
    // Linear funding rate based on imbalance
    // Max funding at 100% imbalance (all long or all short)
    (imbalance * max_funding_rate_bps as i64) / 5000
}