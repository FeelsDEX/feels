/// Unified rebase framework for the market physics model.
/// All rebase operations share common conservation principles.
use anchor_lang::prelude::*;
use crate::logic::conservation::verify_conservation;
use crate::error::FeelsProtocolError;

// ============================================================================
// Core Rebase Trait
// ============================================================================

/// Common interface for all rebase strategies
pub trait RebaseStrategy {
    /// Type representing the rebase factors (growth multipliers)
    type Factors: RebaseFactors;
    
    /// Type representing the masses/values being rebased
    type Masses;
    
    /// Calculate rebase factors based on current state
    fn calculate_factors(
        &self,
        masses: &Self::Masses,
        params: &RebaseParams,
    ) -> Result<Self::Factors>;
    
    /// Apply rebase factors to update state
    fn apply_rebase(
        &self,
        state: &mut impl RebaseState,
        factors: &Self::Factors,
    ) -> Result<()>;
    
    /// Verify conservation laws are preserved
    fn verify_conservation(
        &self,
        factors: &Self::Factors,
        masses: &Self::Masses,
    ) -> Result<()>;
}

/// Common interface for rebase factors
pub trait RebaseFactors: Clone + Debug {
    /// Get all growth factors as array for conservation checking
    fn as_array(&self) -> Vec<u128>;
    
    /// Get timestamp of rebase
    fn timestamp(&self) -> i64;
    
    /// Check if this is an identity rebase (no change)
    fn is_identity(&self) -> bool;
}

/// Common interface for state that can be rebased
pub trait RebaseState {
    /// Apply growth factor to a specific component
    fn apply_growth(&mut self, component: &str, factor: u128) -> Result<()>;
    
    /// Get current value of a component
    fn get_value(&self, component: &str) -> Result<u128>;
}

// ============================================================================
// Rebase Parameters
// ============================================================================

/// Common parameters for all rebase operations
#[derive(Clone, Debug)]
pub struct RebaseParams {
    /// Time elapsed since last rebase
    pub time_elapsed: i64,
    
    /// Current timestamp
    pub timestamp: i64,
    
    /// Protocol numeraire for value conversions
    pub numeraire: Pubkey,
    
    /// Additional domain-specific parameters
    pub domain_params: DomainParams,
}

/// Domain-specific parameters
#[derive(Clone, Debug)]
pub enum DomainParams {
    /// Lending domain parameters
    Lending {
        supply_rate: u64,
        borrow_rate: u64,
        reserve_factor: u64,
    },
    
    /// Leverage domain parameters
    Leverage {
        old_price: u128,
        new_price: u128,
        avg_leverage: u64,
    },
    
    /// Funding domain parameters
    Funding {
        funding_rate: i64,
        imbalance_ratio: i64,
    },
    
    /// Weight rebase parameters
    Weight {
        old_weights: DomainWeights,
        new_weights: DomainWeights,
    },
}

// ============================================================================
// Domain Weights
// ============================================================================

/// Weights for each domain in the unified model
#[derive(Clone, Debug, Default)]
pub struct DomainWeights {
    /// Spot weight (basis points)
    pub w_s: u32,
    
    /// Time weight (basis points)
    pub w_t: u32,
    
    /// Leverage weight (basis points)
    pub w_l: u32,
    
    /// Buffer weight (basis points)
    pub w_tau: u32,
}

impl DomainWeights {
    /// Create new domain weights
    pub fn new(w_s: u32, w_t: u32, w_l: u32, w_tau: u32) -> Result<Self> {
        let total = w_s + w_t + w_l + w_tau;
        require!(
            total == 10_000,
            FeelsProtocolError::InvalidWeights
        );
        
        Ok(Self { w_s, w_t, w_l, w_tau })
    }
    
    /// Get weights as array
    pub fn as_array(&self) -> [u64; 4] {
        [
            self.w_s as u64,
            self.w_t as u64,
            self.w_l as u64,
            self.w_tau as u64,
        ]
    }
}

// ============================================================================
// Rebase Executor
// ============================================================================

/// Unified executor for all rebase operations
pub struct RebaseExecutor<S: RebaseStrategy> {
    strategy: S,
}

impl<S: RebaseStrategy> RebaseExecutor<S> {
    /// Create new rebase executor
    pub fn new(strategy: S) -> Self {
        Self { strategy }
    }
    
    /// Execute rebase with full validation
    pub fn execute(
        &self,
        state: &mut impl RebaseState,
        masses: &S::Masses,
        params: &RebaseParams,
    ) -> Result<S::Factors> {
        // Calculate rebase factors
        let factors = self.strategy.calculate_factors(masses, params)?;
        
        // Skip if identity rebase
        if factors.is_identity() {
            return Ok(factors);
        }
        
        // Verify conservation before applying
        self.strategy.verify_conservation(&factors, masses)?;
        
        // Apply rebase to state
        self.strategy.apply_rebase(state, &factors)?;
        
        // Emit rebase event
        emit!(RebaseExecutedEvent {
            timestamp: factors.timestamp(),
            domain: self.get_domain_name(),
        });
        
        Ok(factors)
    }
    
    /// Get domain name for logging
    fn get_domain_name(&self) -> String {
        // This would be implemented by each strategy
        "Unknown".to_string()
    }
}

// ============================================================================
// Common Rebase Helpers
// ============================================================================

/// Apply growth factor with overflow checking
pub fn apply_growth_factor(value: u128, factor: u128) -> Result<u128> {
    value
        .checked_mul(factor)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(1u128 << 64)
        .ok_or(FeelsProtocolError::DivisionByZero)
}

/// Calculate weighted log sum for conservation checking
pub fn calculate_weighted_log_sum(
    weights: &[u64],
    factors: &[u128],
) -> Result<i128> {
    require!(
        weights.len() == factors.len(),
        FeelsProtocolError::InvalidInput
    );
    
    let mut sum: i128 = 0;
    
    for (w, g) in weights.iter().zip(factors.iter()) {
        // ln(g) approximation for small deviations from 1
        let ln_g = calculate_ln_approximation(*g)?;
        let weighted = (*w as i128 * ln_g) / 10_000;
        sum = sum.saturating_add(weighted);
    }
    
    Ok(sum)
}

/// Approximate ln(x) for x near 1 (in Q64 fixed point)
fn calculate_ln_approximation(x: u128) -> Result<i128> {
    let one = 1u128 << 64;
    
    // For x close to 1: ln(x) ≈ (x - 1) / 1
    if x > one {
        Ok(((x - one) as i128 * 1000) / (one as i128))
    } else {
        Ok(-(((one - x) as i128 * 1000) / (one as i128)))
    }
}

// ============================================================================
// Events
// ============================================================================

/// Event emitted when rebase is executed
#[event]
pub struct RebaseExecutedEvent {
    /// Timestamp of rebase
    pub timestamp: i64,
    
    /// Domain that was rebased
    pub domain: String,
}

// ============================================================================
// Error Extensions
// ============================================================================

/// Rebase-specific errors
#[derive(Debug)]
pub enum RebaseError {
    /// Conservation law violated
    ConservationViolation {
        expected: i128,
        actual: i128,
    },
    
    /// Excessive rebase factor
    ExcessiveRebase {
        factor: u128,
        max_allowed: u128,
    },
    
    /// Invalid time elapsed
    InvalidTimeElapsed {
        elapsed: i64,
    },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weighted_log_sum_conservation() {
        // Test that sum is zero for balanced rebases
        let weights = vec![2500, 2500, 2500, 2500];
        let scale = 1u128 << 64;
        
        // Balanced rebase: some go up, some go down
        let factors = vec![
            scale * 11 / 10,  // +10%
            scale * 9 / 10,   // -10%
            scale * 11 / 10,  // +10%
            scale * 9 / 10,   // -10%
        ];
        
        let sum = calculate_weighted_log_sum(&weights, &factors).unwrap();
        
        // Should be approximately zero
        assert!(sum.abs() < 100);
    }
    
    #[test]
    fn test_apply_growth_factor() {
        let value = 1000u128;
        let scale = 1u128 << 64;
        
        // 50% growth
        let factor = scale * 3 / 2;
        let result = apply_growth_factor(value, factor).unwrap();
        assert_eq!(result, 1500);
        
        // 25% reduction
        let factor = scale * 3 / 4;
        let result = apply_growth_factor(value, factor).unwrap();
        assert_eq!(result, 750);
    }
}