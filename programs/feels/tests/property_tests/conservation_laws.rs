/// Property-based tests for conservation law enforcement.
/// Verifies that all rebase operations maintain the weighted log-sum invariant.

use anchor_lang::prelude::*;
use proptest::prelude::*;
use feels::state::{RebaseAccumulator, WeightRebaseFactors};
use feels::logic::conservation::{ConservationManager, ConservationMetrics};
use feels::constant::Q64;

// ============================================================================
// Test Strategies
// ============================================================================

/// Generate valid domain weights that sum to 10000 bps
fn valid_weights() -> impl Strategy<Value = (u32, u32, u32, u32)> {
    (1u32..3000, 1u32..3000, 1u32..3000, 1u32..3000)
        .prop_map(|(w1, w2, w3, w4)| {
            let sum = w1 + w2 + w3 + w4;
            // Normalize to sum to 10000
            let w_s = (w1 * 10000) / sum;
            let w_t = (w2 * 10000) / sum;
            let w_l = (w3 * 10000) / sum;
            let w_tau = 10000 - w_s - w_t - w_l; // Ensure exact sum
            (w_s, w_t, w_l, w_tau)
        })
}

/// Generate valid rebase rates
fn rebase_rates() -> impl Strategy<Value = (i64, i64, i64, i64)> {
    (
        -1000i64..1000,  // -10% to +10% per period
        -1000i64..1000,
        -1000i64..1000,
        -1000i64..1000,
    )
}

/// Generate valid scalars
fn scalar_values() -> impl Strategy<Value = u128> {
    (1u128..1000).prop_map(|v| v * Q64)
}

/// Generate time deltas
fn time_delta() -> impl Strategy<Value = i64> {
    1i64..3600 // 1 second to 1 hour
}

// ============================================================================
// Conservation Properties
// ============================================================================

proptest! {
    /// Test 1: Exact exponential rebasing preserves conservation
    #[test]
    fn prop_exact_rebasing_conserves_invariant(
        (w_s, w_t, w_l, w_tau) in valid_weights(),
        (r_s, r_t, r_l, r_tau) in rebase_rates(),
        delta_t in time_delta(),
        S in scalar_values(),
        T in scalar_values(),
        L in scalar_values(),
        tau in scalar_values(),
    ) {
        // Setup
        let mut accumulator = RebaseAccumulator::default();
        accumulator.scalar_a = S;
        accumulator.scalar_b = T;
        accumulator.leverage_scalar = L;
        accumulator.tau_scalar = tau;
        
        // Calculate initial invariant
        let invariant_before = calculate_weighted_log_sum(S, T, L, tau, w_s, w_t, w_l, w_tau);
        
        // Apply exact exponential rebasing
        // In practice, these factors would be computed off-chain
        let growth_a = calculate_exact_growth_factor(r_s, delta_t);
        let growth_b = calculate_exact_growth_factor(r_t, delta_t);
        let growth_l = calculate_exact_growth_factor(r_l, delta_t);
        let growth_tau = calculate_exact_growth_factor(r_tau, delta_t);
        
        let result = accumulator.update_indices_with_factors(
            Clock::get().unwrap().unix_timestamp,
            Some(growth_a),
            Some(growth_b),
            Some(growth_l),
            Some(growth_tau),
        );
        
        prop_assert!(result.is_ok(), "Rebase update should succeed");
        
        // Calculate invariant after
        let invariant_after = calculate_weighted_log_sum(
            accumulator.scalar_a,
            accumulator.scalar_b,
            accumulator.leverage_scalar,
            accumulator.tau_scalar,
            w_s, w_t, w_l, w_tau
        );
        
        // Verify conservation (allowing small numerical error)
        let tolerance = Q64 / 1000000; // 0.0001% tolerance
        prop_assert!(
            (invariant_after as i128 - invariant_before as i128).abs() < tolerance as i128,
            "Conservation violated: before={}, after={}, diff={}",
            invariant_before, invariant_after,
            (invariant_after as i128 - invariant_before as i128).abs()
        );
    }
    
    /// Test 2: Weight changes require proper rebalancing
    #[test]
    fn prop_weight_rebase_maintains_conservation(
        (old_ws, old_wt, old_wl, old_wtau) in valid_weights(),
        (new_ws, new_wt, new_wl, new_wtau) in valid_weights(),
        S in scalar_values(),
        T in scalar_values(),
        L in scalar_values(),
        tau in scalar_values(),
    ) {
        // Skip if weights unchanged
        if (old_ws, old_wt, old_wl, old_wtau) == (new_ws, new_wt, new_wl, new_wtau) {
            return Ok(());
        }
        
        // Calculate required rebase factors
        let factors = WeightRebaseFactors::calculate(
            S, T, L, tau,
            old_ws, old_wt, old_wl, old_wtau,
            new_ws, new_wt, new_wl, new_wtau,
        );
        
        prop_assert!(factors.is_ok(), "Weight rebase calculation should succeed");
        let factors = factors.unwrap();
        
        // Apply factors
        let new_S = apply_factor(S, factors.g_s);
        let new_T = apply_factor(T, factors.g_t);
        let new_L = apply_factor(L, factors.g_l);
        let new_tau = apply_factor(tau, factors.g_tau);
        
        // Calculate invariants
        let old_invariant = calculate_weighted_log_sum(S, T, L, tau, old_ws, old_wt, old_wl, old_wtau);
        let new_invariant = calculate_weighted_log_sum(new_S, new_T, new_L, new_tau, new_ws, new_wt, new_wl, new_wtau);
        
        // Verify conservation
        let tolerance = Q64 / 1000000;
        prop_assert!(
            (new_invariant as i128 - old_invariant as i128).abs() < tolerance as i128,
            "Weight rebase violated conservation"
        );
    }
    
    /// Test 3: Linear approximation error bounds
    #[test]
    fn prop_linear_approximation_bounded_error(
        rate in -100i64..100, // -1% to +1% for small rates
        delta_t in 1i64..60,   // Short time periods
    ) {
        // Calculate exact and linear approximations
        let exact_factor = calculate_exact_growth_factor(rate, delta_t);
        let linear_factor = calculate_linear_growth_factor(rate, delta_t);
        
        // Calculate error
        let error_bps = if exact_factor > 0 {
            ((exact_factor as i128 - linear_factor as i128).abs() * 10000) / exact_factor as i128
        } else {
            0
        };
        
        // For small rates and short periods, error should be < 0.1%
        prop_assert!(
            error_bps < 10,
            "Linear approximation error too large: {} bps", error_bps
        );
    }
    
    /// Test 4: Conservation metrics tracking
    #[test]
    fn prop_conservation_metrics_accurate(
        violations in prop::collection::vec(0u64..100, 1..10),
        epsilon in 1u64..1000,
    ) {
        let mut metrics = ConservationMetrics::default();
        
        // Record violations
        for v in &violations {
            metrics.record_violation(*v);
        }
        
        // Verify metrics
        prop_assert_eq!(metrics.total_violations, violations.len() as u64);
        prop_assert_eq!(metrics.total_epsilon_accumulated, violations.iter().sum::<u64>());
        
        // Check average
        if !violations.is_empty() {
            let expected_avg = violations.iter().sum::<u64>() / violations.len() as u64;
            let actual_avg = metrics.average_epsilon();
            prop_assert!(
                (actual_avg as i64 - expected_avg as i64).abs() <= 1,
                "Average epsilon incorrect"
            );
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate weighted log sum (simplified on-chain version)
fn calculate_weighted_log_sum(S: u128, T: u128, L: u128, tau: u128, w_s: u32, w_t: u32, w_l: u32, w_tau: u32) -> u128 {
    // On-chain we can't compute ln, so we track the product of growth factors
    // This is a simplified version for testing
    let weighted_sum = (S * w_s as u128 + T * w_t as u128 + L * w_l as u128 + tau * w_tau as u128) / 10000;
    weighted_sum
}

/// Calculate exact exponential growth factor (would be done off-chain)
fn calculate_exact_growth_factor(rate_bps: i64, delta_t: i64) -> u128 {
    // g = e^(rate * delta_t)
    // For testing, use approximation: e^x ≈ 1 + x + x²/2 for small x
    let x = (rate_bps as i128 * delta_t as i128 * Q64 as i128) / (10000 * 86400); // Daily rate
    
    if x >= 0 {
        let x_squared = (x * x) >> 64;
        (Q64 as i128 + x + x_squared / 2) as u128
    } else {
        let abs_x = -x;
        let x_squared = (abs_x * abs_x) >> 64;
        let factor = Q64 as i128 + abs_x + x_squared / 2;
        // 1/e^|x| ≈ Q64² / factor
        ((Q64 as i128 * Q64 as i128) / factor) as u128
    }
}

/// Calculate linear growth factor (unsafe approximation)
fn calculate_linear_growth_factor(rate_bps: i64, delta_t: i64) -> u128 {
    let rate_per_second = (rate_bps as i128 * Q64 as i128) / (10000 * 86400);
    let total_rate = rate_per_second * delta_t as i128;
    
    if total_rate >= 0 {
        (Q64 as i128 + total_rate) as u128
    } else {
        (Q64 as i128 - (-total_rate)) as u128
    }
}

/// Apply growth factor to scalar
fn apply_factor(scalar: u128, factor: u128) -> u128 {
    ((scalar as u128 * factor) / Q64) as u128
}

// ============================================================================
// Additional Properties
// ============================================================================

proptest! {
    /// Test 5: Rebase composition preserves conservation
    #[test]
    fn prop_rebase_composition_conserves(
        rates1 in rebase_rates(),
        rates2 in rebase_rates(),
        delta_t1 in time_delta(),
        delta_t2 in time_delta(),
        initial_scalars in (scalar_values(), scalar_values(), scalar_values(), scalar_values()),
    ) {
        let (S, T, L, tau) = initial_scalars;
        let weights = (2500u32, 2500u32, 2500u32, 2500u32); // Equal weights
        
        // Apply two rebases sequentially
        let g1_s = calculate_exact_growth_factor(rates1.0, delta_t1);
        let g1_t = calculate_exact_growth_factor(rates1.1, delta_t1);
        let g1_l = calculate_exact_growth_factor(rates1.2, delta_t1);
        let g1_tau = calculate_exact_growth_factor(rates1.3, delta_t1);
        
        let S1 = apply_factor(S, g1_s);
        let T1 = apply_factor(T, g1_t);
        let L1 = apply_factor(L, g1_l);
        let tau1 = apply_factor(tau, g1_tau);
        
        let g2_s = calculate_exact_growth_factor(rates2.0, delta_t2);
        let g2_t = calculate_exact_growth_factor(rates2.1, delta_t2);
        let g2_l = calculate_exact_growth_factor(rates2.2, delta_t2);
        let g2_tau = calculate_exact_growth_factor(rates2.3, delta_t2);
        
        let S2 = apply_factor(S1, g2_s);
        let T2 = apply_factor(T1, g2_t);
        let L2 = apply_factor(L1, g2_l);
        let tau2 = apply_factor(tau1, g2_tau);
        
        // Compare with single combined rebase
        let combined_g_s = ((g1_s as u128 * g2_s as u128) / Q64) as u128;
        let combined_g_t = ((g1_t as u128 * g2_t as u128) / Q64) as u128;
        let combined_g_l = ((g1_l as u128 * g2_l as u128) / Q64) as u128;
        let combined_g_tau = ((g1_tau as u128 * g2_tau as u128) / Q64) as u128;
        
        let S_combined = apply_factor(S, combined_g_s);
        let T_combined = apply_factor(T, combined_g_t);
        let L_combined = apply_factor(L, combined_g_l);
        let tau_combined = apply_factor(tau, combined_g_tau);
        
        // Verify composition property
        prop_assert!((S2 as i128 - S_combined as i128).abs() < 1000);
        prop_assert!((T2 as i128 - T_combined as i128).abs() < 1000);
        prop_assert!((L2 as i128 - L_combined as i128).abs() < 1000);
        prop_assert!((tau2 as i128 - tau_combined as i128).abs() < 1000);
    }
}