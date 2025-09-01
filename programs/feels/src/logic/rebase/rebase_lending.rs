/// Lending domain rebase implementation for the market physics model.
/// Interest delivery via rebasing must preserve weighted log-sum conservation.
use anchor_lang::prelude::*;
use crate::logic::market_physics::conservation::{verify_conservation, solve_conservation_factor};
use crate::logic::market_physics::potential::{FixedPoint, exp_fixed, ln_fixed};
use super::rebase::{RebaseStrategy, RebaseFactors, RebaseState, RebaseParams, DomainParams};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Seconds per year for rate calculations
pub const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Reserve factor (protocol take from interest)
pub const DEFAULT_RESERVE_FACTOR_BPS: u64 = 1000; // 10%

// ============================================================================
// Lending Mass Tracking
// ============================================================================

/// Masses participating in lending domain rebase
#[derive(Clone, Debug, Default)]
pub struct LendingMasses {
    /// Total deposits (A)
    pub A: u128,
    
    /// Total debt (D)
    pub D: u128,
    
    /// Buffer participation in lending
    pub tau_participation: u128,
    
    /// Numeraire conversion rates
    pub deposit_price: u128,
    pub debt_price: u128,
}

impl LendingMasses {
    /// Calculate total mass in lending domain
    pub fn total_mass(&self) -> u128 {
        self.A
            .saturating_add(self.D)
            .saturating_add(self.tau_participation)
    }
    
    /// Calculate weights at epoch snapshot
    pub fn calculate_weights(&self) -> Result<(u64, u64, u64)> {
        let total = self.total_mass();
        require!(total > 0, FeelsProtocolError::DivisionByZero);
        
        let w_A = ((self.A as u128 * BPS_DENOMINATOR as u128) / total) as u64;
        let w_D = ((self.D as u128 * BPS_DENOMINATOR as u128) / total) as u64;
        let w_tau_lend = BPS_DENOMINATOR - w_A - w_D;
        
        Ok((w_A, w_D, w_tau_lend))
    }
}

// ============================================================================
// Lending Rebase Factors
// ============================================================================

/// Growth factors for lending domain rebase
#[derive(Clone, Debug)]
pub struct LendingRebaseFactors {
    /// Growth factor for deposits (suppliers)
    pub g_A: u128,
    
    /// Growth factor for debt (borrowers)
    pub g_D: u128,
    
    /// Growth factor for buffer participation
    pub g_tau: u128,
    
    /// Timestamp of rebase
    pub timestamp: i64,
}

// ============================================================================
// Lending Rebase Calculation
// ============================================================================

/// Calculate lending rebase factors preserving conservation
pub fn calculate_lending_rebase(
    supply_rate: FixedPoint,
    borrow_rate: FixedPoint,
    time_elapsed: i64,
    masses: &LendingMasses,
) -> Result<LendingRebaseFactors> {
    require!(time_elapsed > 0, FeelsProtocolError::InvalidInput);
    
    // Calculate growth factors for suppliers and borrowers
    let g_A = calculate_growth_factor(supply_rate, time_elapsed)?;
    let g_D = calculate_growth_factor(borrow_rate, time_elapsed)?;
    
    // Calculate weights at snapshot time
    let (w_A, w_D, w_tau_lend) = masses.calculate_weights()?;
    
    // If buffer doesn't participate, simple two-party conservation
    if w_tau_lend == 0 {
        // Verify two-party conservation: w_A * ln(g_A) + w_D * ln(g_D) = 0
        let weights = [w_A, w_D];
        let factors = [g_A, g_D];
        verify_conservation(&weights, &factors)?;
        
        return Ok(LendingRebaseFactors {
            g_A,
            g_D,
            g_tau: 1u128 << 64, // No change
            timestamp: Clock::get()?.unix_timestamp,
        });
    }
    
    // Solve for g_tau to preserve conservation
    // w_A * ln(g_A) + w_D * ln(g_D) + w_tau * ln(g_tau) = 0
    let weights = [w_A, w_D, w_tau_lend];
    let known_factors = [g_A, g_D];
    
    let g_tau = solve_conservation_factor(&weights, &known_factors, 2)?;
    
    // Verify full conservation
    let all_factors = [g_A, g_D, g_tau];
    verify_conservation(&weights, &all_factors)?;
    
    Ok(LendingRebaseFactors {
        g_A,
        g_D,
        g_tau,
        timestamp: Clock::get()?.unix_timestamp,
    })
}

/// Calculate growth factor from interest rate and time
fn calculate_growth_factor(rate: FixedPoint, time_elapsed: i64) -> Result<u128> {
    // g = e^(rate * time_elapsed / seconds_per_year)
    
    // Calculate exponent
    let exponent = rate
        .mul(FixedPoint::from_int(time_elapsed))?
        .div(FixedPoint::from_int(SECONDS_PER_YEAR))?;
    
    // Calculate e^exponent
    exp_fixed(exponent)
}

// ============================================================================
// Interest Rate Calculation
// ============================================================================

/// Calculate supply rate based on utilization
pub fn calculate_supply_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
    reserve_factor_bps: u64,
) -> FixedPoint {
    // supply_rate = borrow_rate * utilization * (1 - reserve_factor)
    
    let borrow_rate = calculate_borrow_rate(utilization_bps, base_rate_bps);
    
    // Apply utilization
    let utilized_rate = borrow_rate
        .mul(FixedPoint::from_scaled(
            (utilization_bps as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
        ))
        .unwrap_or(FixedPoint::ZERO);
    
    // Apply reserve factor
    let net_rate = utilized_rate
        .mul(FixedPoint::from_scaled(
            ((BPS_DENOMINATOR - reserve_factor_bps) as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
        ))
        .unwrap_or(FixedPoint::ZERO);
    
    net_rate
}

/// Calculate borrow rate based on utilization
pub fn calculate_borrow_rate(
    utilization_bps: u64,
    base_rate_bps: u64,
) -> FixedPoint {
    // Simple linear model: rate = base + utilization * slope
    // At 80% utilization, rate = 4x base rate
    
    let base = FixedPoint::from_scaled(
        (base_rate_bps as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
    );
    
    // Slope = 3 * base_rate / 80%
    let slope = base
        .mul(FixedPoint::from_int(3))
        .unwrap_or(FixedPoint::ZERO)
        .div(FixedPoint::from_scaled(
            (8000i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
        ))
        .unwrap_or(FixedPoint::ZERO);
    
    // rate = base + utilization * slope
    let utilization_factor = FixedPoint::from_scaled(
        (utilization_bps as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
    );
    
    base.add(slope.mul(utilization_factor).unwrap_or(FixedPoint::ZERO))
        .unwrap_or(base)
}

// ============================================================================
// Lending State Update
// ============================================================================

/// Apply lending rebase to pool state
pub fn apply_lending_rebase(
    deposits: &mut u128,
    borrows: &mut u128,
    buffer_value: &mut u128,
    factors: &LendingRebaseFactors,
) -> Result<()> {
    // Apply growth factors
    *deposits = (*deposits as u128)
        .checked_mul(factors.g_A)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    *borrows = (*borrows as u128)
        .checked_mul(factors.g_D)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    *buffer_value = (*buffer_value as u128)
        .checked_mul(factors.g_tau)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    Ok(())
}

// ============================================================================
// Utilization Calculation
// ============================================================================

/// Calculate pool utilization rate
pub fn calculate_utilization(
    total_borrows: u128,
    total_deposits: u128,
) -> Result<u64> {
    if total_deposits == 0 {
        return Ok(0);
    }
    
    let utilization = (total_borrows as u128)
        .checked_mul(BPS_DENOMINATOR as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(total_deposits as u128)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    Ok(utilization.min(BPS_DENOMINATOR as u128) as u64)
}

// ============================================================================
// Edge Case Handling
// ============================================================================

/// Handle zero deposits case
pub fn handle_zero_deposits(
    borrows: u128,
    borrow_rate: FixedPoint,
    time_elapsed: i64,
) -> Result<LendingRebaseFactors> {
    // With no deposits, only borrowers pay interest to buffer
    let g_D = calculate_growth_factor(borrow_rate, time_elapsed)?;
    
    // All interest goes to buffer
    // Since w_A = 0, conservation becomes: w_D * ln(g_D) + w_tau * ln(g_tau) = 0
    // This gives: g_tau = g_D^(-w_D/w_tau) = g_D (since w_D + w_tau = 1)
    
    Ok(LendingRebaseFactors {
        g_A: 1u128 << 64, // No change
        g_D,
        g_tau: g_D, // Buffer receives all interest
        timestamp: Clock::get()?.unix_timestamp,
    })
}

/// Handle zero debt case
pub fn handle_zero_debt(
    deposits: u128,
    supply_rate: FixedPoint,
    time_elapsed: i64,
) -> Result<LendingRebaseFactors> {
    // With no debt, suppliers earn from buffer
    let g_A = calculate_growth_factor(supply_rate, time_elapsed)?;
    
    // Conservation: w_A * ln(g_A) + w_tau * ln(g_tau) = 0
    // This gives: g_tau = g_A^(-w_A/w_tau)
    
    // For simplicity, if no borrowing, no interest accrual
    Ok(LendingRebaseFactors {
        g_A: 1u128 << 64, // No change
        g_D: 1u128 << 64, // No change
        g_tau: 1u128 << 64, // No change
        timestamp: Clock::get()?.unix_timestamp,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lending_conservation() {
        let masses = LendingMasses {
            A: 1_000_000 * (1u128 << 64), // 1M deposits
            D: 800_000 * (1u128 << 64),   // 800K debt (80% utilization)
            tau_participation: 200_000 * (1u128 << 64), // 200K buffer
            deposit_price: 1u128 << 64,
            debt_price: 1u128 << 64,
        };
        
        let supply_rate = FixedPoint::from_scaled((500 * FixedPoint::SCALE) / BPS_DENOMINATOR); // 5% APY
        let borrow_rate = FixedPoint::from_scaled((800 * FixedPoint::SCALE) / BPS_DENOMINATOR); // 8% APY
        let time_elapsed = 86400; // 1 day
        
        let factors = calculate_lending_rebase(
            supply_rate,
            borrow_rate,
            time_elapsed,
            &masses,
        ).unwrap();
        
        // Verify conservation
        let (w_A, w_D, w_tau) = masses.calculate_weights().unwrap();
        let weights = [w_A, w_D, w_tau];
        let growth_factors = [factors.g_A, factors.g_D, factors.g_tau];
        
        assert!(verify_conservation(&weights, &growth_factors).is_ok());
    }
    
    #[test]
    fn test_utilization_calculation() {
        let borrows = 750_000u128;
        let deposits = 1_000_000u128;
        
        let utilization = calculate_utilization(borrows, deposits).unwrap();
        assert_eq!(utilization, 7500); // 75%
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl RebaseFactors for LendingRebaseFactors {
    fn as_array(&self) -> Vec<u128> {
        vec![self.g_A, self.g_D, self.g_tau]
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn is_identity(&self) -> bool {
        let one = 1u128 << 64;
        self.g_A == one && self.g_D == one && self.g_tau == one
    }
}

/// Lending rebase strategy implementation
pub struct LendingRebaseStrategy;

impl RebaseStrategy for LendingRebaseStrategy {
    type Factors = LendingRebaseFactors;
    type Masses = LendingMasses;
    
    fn calculate_factors(
        &self,
        masses: &Self::Masses,
        params: &RebaseParams,
    ) -> Result<Self::Factors> {
        match &params.domain_params {
            DomainParams::Lending { supply_rate, borrow_rate, .. } => {
                let supply_fp = FixedPoint::from_scaled(
                    (*supply_rate as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
                );
                let borrow_fp = FixedPoint::from_scaled(
                    (*borrow_rate as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128
                );
                
                calculate_lending_rebase(
                    supply_fp,
                    borrow_fp,
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
        state.apply_growth("deposits", factors.g_A)?;
        state.apply_growth("borrows", factors.g_D)?;
        state.apply_growth("buffer", factors.g_tau)?;
        Ok(())
    }
    
    fn verify_conservation(
        &self,
        factors: &Self::Factors,
        masses: &Self::Masses,
    ) -> Result<()> {
        let (w_A, w_D, w_tau) = masses.calculate_weights()?;
        let weights = [w_A, w_D, w_tau];
        let growth_factors = [factors.g_A, factors.g_D, factors.g_tau];
        
        verify_conservation(&weights, &growth_factors)
    }
}