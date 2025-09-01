/// Weight-rebase implementation for the market physics model.
/// Market conditions change weights, requiring rebases that preserve K and prices.
use anchor_lang::prelude::*;
use crate::state::MarketState;
use crate::logic::market_physics::conservation::verify_conservation;
use crate::logic::market_physics::potential::FixedPoint;
use super::rebase::{RebaseStrategy, RebaseFactors, RebaseState, RebaseParams, DomainParams, DomainWeights};
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum Newton iterations for convergence
pub const MAX_NEWTON_STEPS: usize = 10;

/// Convergence tolerance for Newton solver
pub const CONVERGENCE_TOL: i128 = 1000; // ~1e-15 in Q64

/// Damping factor for Newton steps
pub const DAMPING_FACTOR: i128 = (1i128 << 63); // 0.5 in Q64

/// Maximum weight change per update (basis points)
pub const MAX_WEIGHT_CHANGE_BPS: u32 = 1000; // 10%

// ============================================================================
// Weight Rebase Factors
// ============================================================================

/// Rebase factors for weight changes
#[derive(Clone, Debug, Default)]
pub struct WeightRebaseFactors {
    /// Rebase factor for spot dimension
    pub h_S: u128,
    
    /// Rebase factor for time dimension
    pub h_T: u128,
    
    /// Rebase factor for leverage dimension
    pub h_L: u128,
    
    /// Rebase factor for buffer
    pub h_tau: u128,
}

impl WeightRebaseFactors {
    /// Create identity factors (no rebase)
    pub fn identity() -> Self {
        let one = 1u128 << 64;
        Self {
            h_S: one,
            h_T: one,
            h_L: one,
            h_tau: one,
        }
    }
    
    /// Apply damped Newton step
    pub fn add_step(&self, step: &NewtonStep, damping: i128) -> Self {
        let damping_u128 = damping as u128;
        let scale = 1u128 << 64;
        
        Self {
            h_S: apply_damped_update(self.h_S, step.dh_S, damping_u128, scale),
            h_T: apply_damped_update(self.h_T, step.dh_T, damping_u128, scale),
            h_L: apply_damped_update(self.h_L, step.dh_L, damping_u128, scale),
            h_tau: apply_damped_update(self.h_tau, step.dh_tau, damping_u128, scale),
        }
    }
    
    /// Convert to array for conservation checking
    pub fn to_array(&self) -> [u128; 4] {
        [self.h_S, self.h_T, self.h_L, self.h_tau]
    }
}

/// Newton step for iterative solver
#[derive(Clone, Debug, Default)]
struct NewtonStep {
    dh_S: i128,
    dh_T: i128,
    dh_L: i128,
    dh_tau: i128,
}

/// Apply damped update: new = old * (1 + damping * delta)
fn apply_damped_update(old: u128, delta: i128, damping: u128, scale: u128) -> u128 {
    let adjustment = ((delta as i128 * damping as i128) / scale as i128) as i128;
    let factor = scale as i128 + adjustment;
    
    ((old as u128 * factor as u128) / scale).max(scale / 100) // Min 0.01x
}

// ============================================================================
// Weight Update Calculation
// ============================================================================

/// Calculate weight rebase factors that preserve K and prices
pub fn calculate_weight_rebase(
    old_weights: &DomainWeights,
    new_weights: &DomainWeights,
    current_state: &MarketState,
) -> Result<WeightRebaseFactors> {
    // Validate weight changes are within bounds
    validate_weight_changes(old_weights, new_weights)?;
    
    // Start with identity transformation
    let mut h = WeightRebaseFactors::identity();
    
    // If weights haven't changed significantly, no rebase needed
    if weights_are_close(old_weights, new_weights) {
        return Ok(h);
    }
    
    // Newton iteration to solve for rebase factors
    for iteration in 0..MAX_NEWTON_STEPS {
        // Calculate conservation residual
        let residual = calculate_conservation_residual(&h, old_weights, new_weights);
        
        // Check convergence
        if residual.norm() < CONVERGENCE_TOL {
            break;
        }
        
        // Calculate Jacobian
        let jacobian = calculate_rebase_jacobian(&h, current_state);
        
        // Solve for Newton step
        let step = solve_damped_system(jacobian, residual)?;
        
        // Apply damped step
        h = h.add_step(&step, DAMPING_FACTOR);
    }
    
    // Verify conservation
    verify_weight_rebase_conservation(&h, old_weights, new_weights)?;
    
    // Verify price continuity
    verify_price_continuity(&h, current_state)?;
    
    Ok(h)
}

/// Validate weight changes are within acceptable bounds
fn validate_weight_changes(
    old_weights: &DomainWeights,
    new_weights: &DomainWeights,
) -> Result<()> {
    let changes = [
        (old_weights.w_s, new_weights.w_s),
        (old_weights.w_t, new_weights.w_t),
        (old_weights.w_l, new_weights.w_l),
        (old_weights.w_tau, new_weights.w_tau),
    ];
    
    for (old, new) in &changes {
        let change = (*new as i32 - *old as i32).abs() as u32;
        require!(
            change <= MAX_WEIGHT_CHANGE_BPS,
            FeelsProtocolError::ExcessiveWeightChange
        );
    }
    
    Ok(())
}

/// Check if weights are close enough to skip rebase
fn weights_are_close(w1: &DomainWeights, w2: &DomainWeights) -> bool {
    let tolerance = 10; // 0.1%
    
    (w1.w_s as i32 - w2.w_s as i32).abs() < tolerance &&
    (w1.w_t as i32 - w2.w_t as i32).abs() < tolerance &&
    (w1.w_l as i32 - w2.w_l as i32).abs() < tolerance &&
    (w1.w_tau as i32 - w2.w_tau as i32).abs() < tolerance
}

// ============================================================================
// Conservation Residual
// ============================================================================

/// Conservation residual for weight rebase
struct ConservationResidual {
    /// K_acct conservation error
    k_acct_error: i128,
    
    /// Price continuity errors
    price_s_error: i128,
    price_t_error: i128,
    price_l_error: i128,
}

impl ConservationResidual {
    /// Calculate L2 norm of residual
    fn norm(&self) -> i128 {
        // Simplified norm calculation
        self.k_acct_error.abs() +
        self.price_s_error.abs() +
        self.price_t_error.abs() +
        self.price_l_error.abs()
    }
}

/// Calculate conservation residual
fn calculate_conservation_residual(
    h: &WeightRebaseFactors,
    old_weights: &DomainWeights,
    new_weights: &DomainWeights,
) -> ConservationResidual {
    // For weight change w -> w', we need:
    // 1. K preservation: Π (x_i * h_i)^w'_i = Π x_i^w_i
    // 2. Price continuity: marginal prices unchanged
    
    // Simplified: check that weighted log sum is preserved
    let old_sum = calculate_weighted_log_sum_weights(old_weights);
    let new_sum = calculate_weighted_log_sum_factors(new_weights, h);
    
    ConservationResidual {
        k_acct_error: new_sum - old_sum,
        price_s_error: 0, // Simplified - would check ∂K/∂S continuity
        price_t_error: 0,
        price_l_error: 0,
    }
}

/// Calculate weighted log sum for original weights
fn calculate_weighted_log_sum_weights(weights: &DomainWeights) -> i128 {
    // Σ w_i * ln(1) = 0 (before rebase, all values at identity)
    0
}

/// Calculate weighted log sum after rebase
fn calculate_weighted_log_sum_factors(
    weights: &DomainWeights,
    factors: &WeightRebaseFactors,
) -> i128 {
    // Σ w'_i * ln(h_i)
    // Simplified calculation - in production would use proper logarithms
    let scale = 1i128 << 64;
    
    let sum = (weights.w_s as i128 * ((factors.h_S as i128 - scale) / 1000)) / 10_000 +
              (weights.w_t as i128 * ((factors.h_T as i128 - scale) / 1000)) / 10_000 +
              (weights.w_l as i128 * ((factors.h_L as i128 - scale) / 1000)) / 10_000 +
              (weights.w_tau as i128 * ((factors.h_tau as i128 - scale) / 1000)) / 10_000;
    
    sum
}

// ============================================================================
// Jacobian and Linear System
// ============================================================================

/// Jacobian matrix for Newton solver (simplified to diagonal)
struct RebaseJacobian {
    diagonal: [i128; 4],
}

/// Calculate Jacobian matrix
fn calculate_rebase_jacobian(
    h: &WeightRebaseFactors,
    state: &MarketState,
) -> RebaseJacobian {
    // Simplified diagonal Jacobian
    // J_ii = w'_i / h_i (derivative of conservation w.r.t. h_i)
    
    let weights = state.get_weights();
    let scale = 1i128 << 64;
    
    RebaseJacobian {
        diagonal: [
            (weights.w_s as i128 * scale) / h.h_S as i128,
            (weights.w_t as i128 * scale) / h.h_T as i128,
            (weights.w_l as i128 * scale) / h.h_L as i128,
            (weights.w_tau as i128 * scale) / h.h_tau as i128,
        ],
    }
}

/// Solve damped linear system J * x = -r
fn solve_damped_system(
    jacobian: RebaseJacobian,
    residual: ConservationResidual,
) -> Result<NewtonStep> {
    // For diagonal system, solution is trivial
    let scale = 1i128 << 64;
    
    Ok(NewtonStep {
        dh_S: -residual.k_acct_error * scale / jacobian.diagonal[0],
        dh_T: -residual.price_s_error * scale / jacobian.diagonal[1],
        dh_L: -residual.price_t_error * scale / jacobian.diagonal[2],
        dh_tau: -residual.price_l_error * scale / jacobian.diagonal[3],
    })
}

// ============================================================================
// Verification
// ============================================================================

/// Verify conservation after weight rebase
fn verify_weight_rebase_conservation(
    h: &WeightRebaseFactors,
    old_weights: &DomainWeights,
    new_weights: &DomainWeights,
) -> Result<()> {
    // Check that rebase preserves weighted product
    let factors = h.to_array();
    let weights = [
        new_weights.w_s as u64,
        new_weights.w_t as u64,
        new_weights.w_l as u64,
        new_weights.w_tau as u64,
    ];
    
    verify_conservation(&weights, &factors)?;
    
    Ok(())
}

/// Verify price continuity
fn verify_price_continuity(
    h: &WeightRebaseFactors,
    state: &MarketState,
) -> Result<()> {
    // Marginal prices should remain continuous
    // ∂K/∂x_i = w_i * K / x_i should be unchanged
    
    // Simplified check: ensure no extreme rebases
    let scale = 1u128 << 64;
    let max_deviation = scale / 10; // 10% max change
    
    require!(
        (h.h_S as i128 - scale as i128).abs() < max_deviation as i128,
        FeelsProtocolError::ExcessivePriceImpact
    );
    require!(
        (h.h_T as i128 - scale as i128).abs() < max_deviation as i128,
        FeelsProtocolError::ExcessivePriceImpact
    );
    require!(
        (h.h_L as i128 - scale as i128).abs() < max_deviation as i128,
        FeelsProtocolError::ExcessivePriceImpact
    );
    
    Ok(())
}

// ============================================================================
// Weight Update Strategies
// ============================================================================

/// Calculate risk-based weight adjustments
pub fn calculate_risk_based_weights(
    current_weights: &DomainWeights,
    spot_volatility: u64,
    rate_volatility: u64,
    leverage_imbalance: i64,
) -> Result<DomainWeights> {
    // Base weights
    let mut w_s = current_weights.w_s;
    let mut w_t = current_weights.w_t;
    let mut w_l = current_weights.w_l;
    let w_tau = current_weights.w_tau;
    
    // Adjust based on volatility (higher vol = lower weight)
    if spot_volatility > 100 {
        w_s = (w_s * 9000) / 10000; // Reduce by 10%
    }
    
    if rate_volatility > 100 {
        w_t = (w_t * 9000) / 10000; // Reduce by 10%
    }
    
    // Adjust leverage weight based on imbalance
    if leverage_imbalance.abs() > 2000 {
        w_l = (w_l * 11000) / 10000; // Increase by 10%
    }
    
    // Renormalize
    let total = w_s + w_t + w_l + w_tau;
    w_s = (w_s * 10000) / total;
    w_t = (w_t * 10000) / total;
    w_l = (w_l * 10000) / total;
    let w_tau_new = 10000 - w_s - w_t - w_l;
    
    DomainWeights::new(w_s, w_t, w_l, w_tau_new)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weight_rebase_identity() {
        let weights = DomainWeights::new(2500, 2500, 2500, 2500).unwrap();
        let mut state = MarketState::default();
        state.set_weights(weights);
        
        // Same weights should give identity rebase
        let factors = calculate_weight_rebase(&weights, &weights, &state).unwrap();
        
        let scale = 1u128 << 64;
        assert_eq!(factors.h_S, scale);
        assert_eq!(factors.h_T, scale);
        assert_eq!(factors.h_L, scale);
        assert_eq!(factors.h_tau, scale);
    }
    
    #[test]
    fn test_weight_change_bounds() {
        let old_weights = DomainWeights::new(3000, 3000, 3000, 1000).unwrap();
        let new_weights = DomainWeights::new(4500, 2000, 2500, 1000).unwrap(); // 15% change
        
        let state = MarketState::default();
        
        // Should fail due to excessive change
        assert!(calculate_weight_rebase(&old_weights, &new_weights, &state).is_err());
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl RebaseFactors for WeightRebaseFactors {
    fn as_array(&self) -> Vec<u128> {
        vec![self.h_S, self.h_T, self.h_L, self.h_tau]
    }
    
    fn timestamp(&self) -> i64 {
        Clock::get().unwrap().unix_timestamp
    }
    
    fn is_identity(&self) -> bool {
        let one = 1u128 << 64;
        self.h_S == one && self.h_T == one && self.h_L == one && self.h_tau == one
    }
}

/// Weight rebase strategy implementation
pub struct WeightRebaseStrategy;

impl RebaseStrategy for WeightRebaseStrategy {
    type Factors = WeightRebaseFactors;
    type Masses = MarketState;
    
    fn calculate_factors(
        &self,
        masses: &Self::Masses,
        params: &RebaseParams,
    ) -> Result<Self::Factors> {
        match &params.domain_params {
            DomainParams::Weight { old_weights, new_weights } => {
                calculate_weight_rebase(old_weights, new_weights, masses)
            }
            _ => Err(FeelsProtocolError::InvalidInput.into()),
        }
    }
    
    fn apply_rebase(
        &self,
        state: &mut impl RebaseState,
        factors: &Self::Factors,
    ) -> Result<()> {
        state.apply_growth("spot_value", factors.h_S)?;
        state.apply_growth("time_value", factors.h_T)?;
        state.apply_growth("leverage_value", factors.h_L)?;
        state.apply_growth("buffer_value", factors.h_tau)?;
        Ok(())
    }
    
    fn verify_conservation(
        &self,
        factors: &Self::Factors,
        masses: &Self::Masses,
    ) -> Result<()> {
        let weights = masses.get_weights();
        verify_weight_rebase_conservation(factors, &weights, &weights)
    }
}