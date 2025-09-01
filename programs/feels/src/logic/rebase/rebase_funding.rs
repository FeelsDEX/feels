/// Funding rate rebase implementation for the market physics model.
/// Funding transfers value between crowded/uncrowded sides via rebasing.
use anchor_lang::prelude::*;
use crate::logic::market_physics::conservation::verify_conservation;
use crate::logic::market_physics::potential::{FixedPoint, exp_fixed};
use super::rebase_leverage::LeverageImbalance;
use super::rebase::{RebaseStrategy, RebaseFactors, RebaseState, RebaseParams, DomainParams};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum funding rate (100% APY)
pub const MAX_FUNDING_RATE_BPS: u64 = 10_000;

/// Funding rate scaling factor
pub const FUNDING_RATE_SCALE: u64 = 10_000;

/// Seconds per year
pub const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;

/// Minimum imbalance for funding (1%)
pub const MIN_IMBALANCE_BPS: u64 = 100;

// ============================================================================
// Funding Rebase Factors
// ============================================================================

/// Growth factors for funding rate rebase
#[derive(Clone, Debug)]
pub struct FundingRebaseFactors {
    /// Growth factor for crowded side (pays funding)
    pub g_crowded: u128,
    
    /// Growth factor for uncrowded side (receives funding)
    pub g_uncrowded: u128,
    
    /// Whether longs are the crowded side
    pub is_long_crowded: bool,
    
    /// Funding rate applied (basis points per year)
    pub funding_rate_bps: i64,
    
    /// Timestamp of rebase
    pub timestamp: i64,
}

// ============================================================================
// Funding Rate Calculation
// ============================================================================

/// Calculate funding rate based on long/short imbalance
pub fn calculate_funding_rate(
    long_value: u128,
    short_value: u128,
    max_funding_rate_bps: u64,
) -> Result<i64> {
    let total = long_value.saturating_add(short_value);
    if total == 0 {
        return Ok(0);
    }
    
    // Calculate imbalance ratio: positive if more longs, negative if more shorts
    let long_ratio = (long_value as u128)
        .checked_mul(FUNDING_RATE_SCALE as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(total)
        .ok_or(FeelsProtocolError::DivisionByZero)? as i64;
    
    let imbalance = long_ratio - (FUNDING_RATE_SCALE as i64 / 2); // -5000 to +5000
    
    // Linear funding rate based on imbalance
    // Max funding at 100% imbalance (all long or all short)
    let funding_rate = (imbalance * max_funding_rate_bps as i64) / (FUNDING_RATE_SCALE as i64 / 2);
    
    Ok(funding_rate)
}

/// Calculate funding rate with dampening for small imbalances
pub fn calculate_dampened_funding_rate(
    imbalance: &LeverageImbalance,
    max_funding_rate_bps: u64,
) -> Result<i64> {
    // No funding for small imbalances
    if imbalance.imbalance_ratio.abs() < MIN_IMBALANCE_BPS as i64 {
        return Ok(0);
    }
    
    // Apply square root dampening for smoother rates
    let abs_imbalance = imbalance.imbalance_ratio.abs() as u64;
    let sqrt_imbalance = crate::utils::math::sqrt_u64(abs_imbalance * 100)?; // Scale up for precision
    
    // Calculate funding rate
    let funding_magnitude = (sqrt_imbalance * max_funding_rate_bps) / 100;
    
    // Apply sign based on imbalance direction
    let funding_rate = if imbalance.imbalance_ratio > 0 {
        funding_magnitude as i64
    } else {
        -(funding_magnitude as i64)
    };
    
    Ok(funding_rate)
}

// ============================================================================
// Funding Rebase Calculation
// ============================================================================

/// Calculate funding rebase factors preserving conservation
pub fn calculate_funding_rebase(
    funding_rate: FixedPoint,
    time_elapsed: i64,
    imbalance: &LeverageImbalance,
) -> Result<FundingRebaseFactors> {
    require!(time_elapsed > 0, FeelsProtocolError::InvalidInput);
    
    // Determine crowded side
    let (w_crowded, w_uncrowded) = if imbalance.long_value > imbalance.short_value {
        (imbalance.w_long, imbalance.w_short)
    } else {
        (imbalance.w_short, imbalance.w_long)
    };
    
    // Skip if one side has no weight
    if w_crowded == 0 || w_uncrowded == 0 {
        return Ok(FundingRebaseFactors {
            g_crowded: 1u128 << 64,
            g_uncrowded: 1u128 << 64,
            is_long_crowded: imbalance.long_value > imbalance.short_value,
            funding_rate_bps: 0,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }
    
    // Calculate growth factor for crowded side: g_crowded = e^(-f * Δt)
    let funding_per_second = funding_rate.div(FixedPoint::from_int(SECONDS_PER_YEAR))?;
    let exponent = funding_per_second
        .mul(FixedPoint::from_int(time_elapsed))?
        .neg()?;
    
    let g_crowded = exp_fixed(exponent)?;
    
    // Calculate g_uncrowded to preserve conservation
    let g_uncrowded = calculate_conservation_factor(
        g_crowded,
        w_crowded,
        w_uncrowded,
    )?;
    
    // Verify conservation
    let weights = [w_crowded as u64, w_uncrowded as u64];
    let factors = [g_crowded, g_uncrowded];
    verify_conservation(&weights, &factors)?;
    
    Ok(FundingRebaseFactors {
        g_crowded,
        g_uncrowded,
        is_long_crowded: imbalance.long_value > imbalance.short_value,
        funding_rate_bps: funding_rate.to_u64() * 10_000 / (1 << 64),
        timestamp: Clock::get()?.unix_timestamp,
    })
}

/// Calculate conservation-preserving factor
/// Given g1 and weights w1, w2, find g2 such that w1*ln(g1) + w2*ln(g2) = 0
fn calculate_conservation_factor(
    g1: u128,
    w1: u32,
    w2: u32,
) -> Result<u128> {
    // g2 = g1^(-w1/w2)
    
    // For small deviations from 1, use linear approximation
    let g1_fixed = FixedPoint::from_scaled(g1 as i128);
    let one = FixedPoint::ONE;
    
    if (g1_fixed.value - one.value).abs() < one.value / 100 {
        // Linear approximation: g2 ≈ 1 - (g1 - 1) * w1/w2
        let deviation = g1_fixed.sub(one)?;
        let scaled_deviation = deviation
            .mul(FixedPoint::from_scaled((w1 as i128 * FixedPoint::SCALE) / w2 as i128))?;
        
        let g2_fixed = one.sub(scaled_deviation)?;
        return Ok(g2_fixed.value as u128);
    }
    
    // For larger deviations, use full calculation
    // ln(g2) = -w1/w2 * ln(g1)
    let ln_g1 = crate::logic::potential::ln_fixed(g1)?;
    let weight_ratio = FixedPoint::from_scaled(-((w1 as i128 * FixedPoint::SCALE) / w2 as i128));
    let ln_g2 = ln_g1.mul(weight_ratio)?;
    
    exp_fixed(ln_g2)
}

// ============================================================================
// Funding Application
// ============================================================================

/// Apply funding rebase to positions
pub fn apply_funding_rebase(
    long_positions: &mut u128,
    short_positions: &mut u128,
    factors: &FundingRebaseFactors,
) -> Result<()> {
    if factors.is_long_crowded {
        // Longs pay, shorts receive
        *long_positions = apply_growth_factor(*long_positions, factors.g_crowded)?;
        *short_positions = apply_growth_factor(*short_positions, factors.g_uncrowded)?;
    } else {
        // Shorts pay, longs receive
        *long_positions = apply_growth_factor(*long_positions, factors.g_uncrowded)?;
        *short_positions = apply_growth_factor(*short_positions, factors.g_crowded)?;
    }
    
    Ok(())
}

/// Apply growth factor to a value
fn apply_growth_factor(value: u128, growth_factor: u128) -> Result<u128> {
    value
        .checked_mul(growth_factor)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero.into())
}

// ============================================================================
// Funding Tracking
// ============================================================================

/// Track cumulative funding for a position
#[derive(Clone, Debug, Default)]
pub struct FundingTracker {
    /// Cumulative funding index when position was opened
    pub funding_index_open: u128,
    
    /// Current cumulative funding index
    pub funding_index_current: u128,
    
    /// Total funding paid (negative) or received (positive)
    pub net_funding: i128,
}

impl FundingTracker {
    /// Update funding for a position
    pub fn update(
        &mut self,
        position_value: u128,
        is_long: bool,
        current_funding_index_long: u128,
        current_funding_index_short: u128,
    ) -> Result<()> {
        let current_index = if is_long {
            current_funding_index_long
        } else {
            current_funding_index_short
        };
        
        // Calculate funding multiplier since position open
        let funding_multiplier = current_index
            .checked_mul(1u128 << 64)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(self.funding_index_open)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        // Calculate new position value after funding
        let new_value = apply_growth_factor(position_value, funding_multiplier)?;
        
        // Track net funding
        let funding_change = if new_value > position_value {
            (new_value - position_value) as i128
        } else {
            -((position_value - new_value) as i128)
        };
        
        self.net_funding = self.net_funding
            .checked_add(funding_change)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        self.funding_index_current = current_index;
        
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_funding_rate_calculation() {
        // 60% longs, 40% shorts
        let long_value = 600_000u128;
        let short_value = 400_000u128;
        let max_rate = 5000; // 50% max funding
        
        let rate = calculate_funding_rate(long_value, short_value, max_rate).unwrap();
        
        // Should be positive (longs pay) and proportional to imbalance
        assert!(rate > 0);
        assert!(rate < max_rate as i64);
    }
    
    #[test]
    fn test_funding_conservation() {
        let imbalance = LeverageImbalance {
            long_value: 700_000 * (1u128 << 64),
            short_value: 300_000 * (1u128 << 64),
            w_long: 7000,
            w_short: 3000,
            imbalance_ratio: 2000, // 20% more longs
        };
        
        let funding_rate = FixedPoint::from_scaled((1000 * FixedPoint::SCALE) / 10_000); // 10% APY
        let time_elapsed = 3600; // 1 hour
        
        let factors = calculate_funding_rebase(
            funding_rate,
            time_elapsed,
            &imbalance,
        ).unwrap();
        
        // Verify conservation
        assert!(factors.is_long_crowded);
        assert!(factors.g_crowded < (1u128 << 64)); // Longs lose value
        assert!(factors.g_uncrowded > (1u128 << 64)); // Shorts gain value
        
        // Verify conservation law
        let weights = [imbalance.w_long as u64, imbalance.w_short as u64];
        let factors_array = if factors.is_long_crowded {
            [factors.g_crowded, factors.g_uncrowded]
        } else {
            [factors.g_uncrowded, factors.g_crowded]
        };
        
        assert!(verify_conservation(&weights, &factors_array).is_ok());
    }
    
    #[test]
    fn test_no_funding_when_balanced() {
        let imbalance = LeverageImbalance {
            long_value: 500_000 * (1u128 << 64),
            short_value: 500_000 * (1u128 << 64),
            w_long: 5000,
            w_short: 5000,
            imbalance_ratio: 0,
        };
        
        let rate = calculate_funding_rate(
            imbalance.long_value,
            imbalance.short_value,
            5000,
        ).unwrap();
        
        assert_eq!(rate, 0);
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl RebaseFactors for FundingRebaseFactors {
    fn as_array(&self) -> Vec<u128> {
        vec![self.g_crowded, self.g_uncrowded]
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn is_identity(&self) -> bool {
        let one = 1u128 << 64;
        self.g_crowded == one && self.g_uncrowded == one
    }
}

/// Funding rebase strategy implementation
pub struct FundingRebaseStrategy;

impl RebaseStrategy for FundingRebaseStrategy {
    type Factors = FundingRebaseFactors;
    type Masses = LeverageImbalance;
    
    fn calculate_factors(
        &self,
        masses: &Self::Masses,
        params: &RebaseParams,
    ) -> Result<Self::Factors> {
        match &params.domain_params {
            DomainParams::Funding { funding_rate, .. } => {
                let rate_fp = FixedPoint::from_scaled(
                    (*funding_rate as i128 * FixedPoint::SCALE) / 10_000
                );
                
                calculate_funding_rebase(
                    rate_fp,
                    params.time_elapsed,
                    masses,
                )
            }
            _ => Err(FeelsProtocolError::InvalidInput.into()),
        }
    }
    
    fn apply_rebase(
        &self,
        state: &mut impl RebaseState,
        factors: &Self::Factors,
    ) -> Result<()> {
        if factors.is_long_crowded {
            state.apply_growth("long_positions", factors.g_crowded)?;
            state.apply_growth("short_positions", factors.g_uncrowded)?;
        } else {
            state.apply_growth("long_positions", factors.g_uncrowded)?;
            state.apply_growth("short_positions", factors.g_crowded)?;
        }
        Ok(())
    }
    
    fn verify_conservation(
        &self,
        factors: &Self::Factors,
        masses: &Self::Masses,
    ) -> Result<()> {
        let weights = [masses.w_long as u64, masses.w_short as u64];
        let growth_factors = if factors.is_long_crowded {
            [factors.g_crowded, factors.g_uncrowded]
        } else {
            [factors.g_uncrowded, factors.g_crowded]
        };
        
        verify_conservation(&weights, &growth_factors)
    }
}