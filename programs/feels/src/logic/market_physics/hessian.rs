/// Hessian matrix calculations for the market physics model.
/// The Hessian (second derivatives) enables quadratic approximation for accurate path integration.
use anchor_lang::prelude::*;
use crate::state::{MarketState, gradient_cache::Hessian3x3};
use crate::logic::market_physics::potential::{FixedPoint, normalize_weight};
use crate::logic::market_physics::gradient::Position3D;
use crate::error::FeelsProtocolError;

// ============================================================================
// Hessian Calculation
// ============================================================================

/// Calculate the 3x3 Hessian matrix at a given market state
/// H[i,j] = ∂²V/∂x_i∂x_j
pub fn calculate_hessian_3x3(
    state: &MarketState,
    position: &Position3D,
) -> Result<Hessian3x3> {
    // For V = -Σ ŵ_i ln(x_i), the Hessian is diagonal
    // ∂²V/∂x_i² = ŵ_i / x_i²
    // ∂²V/∂x_i∂x_j = 0 (for i ≠ j)
    
    let (w_hat_s, w_hat_t, w_hat_l) = state.get_weights().get_hat_weights();
    
    // Calculate diagonal elements
    let d2v_ds2 = calculate_diagonal_element(state.S, w_hat_s, state.w_tau)?;
    let d2v_dt2 = calculate_diagonal_element(state.T, w_hat_t, state.w_tau)?;
    let d2v_dl2 = calculate_diagonal_element(state.L, w_hat_l, state.w_tau)?;
    
    Ok(Hessian3x3 {
        d2V_dS2: d2v_ds2.value as u128,
        d2V_dT2: d2v_dt2.value as u128,
        d2V_dL2: d2v_dl2.value as u128,
        // Cross terms are zero for our logarithmic potential
        d2V_dSdT: 0,
        d2V_dSdL: 0,
        d2V_dTdL: 0,
    })
}

/// Calculate diagonal Hessian element: ŵ_i / x_i²
fn calculate_diagonal_element(
    x: u128,
    w_hat: u32,
    w_tau: u32,
) -> Result<FixedPoint> {
    require!(x > 0, FeelsProtocolError::DivisionByZero);
    
    // Get normalized weight
    let w_hat_fixed = normalize_weight(w_hat, w_tau)?;
    
    // Calculate x²
    let x_fixed = FixedPoint::from_scaled(x as i128);
    let x_squared = x_fixed.mul(x_fixed)?;
    
    // ŵ_i / x_i²
    w_hat_fixed.div(x_squared)
}

// ============================================================================
// Hessian Properties
// ============================================================================

/// Verify Hessian is positive definite (required for convexity)
pub fn verify_positive_definite(hessian: &Hessian3x3) -> Result<bool> {
    // For a diagonal matrix, positive definite iff all diagonal elements > 0
    Ok(
        hessian.d2V_dS2 > 0 &&
        hessian.d2V_dT2 > 0 &&
        hessian.d2V_dL2 > 0
    )
}

/// Ensure Hessian is positive definite by adding Levenberg-Marquardt damping
pub fn ensure_positive_definite(
    hessian: Hessian3x3,
    damping_factor: u128,
) -> Result<Hessian3x3> {
    // Add λI to the Hessian (Levenberg-Marquardt damping)
    Ok(Hessian3x3 {
        d2V_dS2: hessian.d2V_dS2.saturating_add(damping_factor),
        d2V_dT2: hessian.d2V_dT2.saturating_add(damping_factor),
        d2V_dL2: hessian.d2V_dL2.saturating_add(damping_factor),
        d2V_dSdT: hessian.d2V_dSdT,
        d2V_dSdL: hessian.d2V_dSdL,
        d2V_dTdL: hessian.d2V_dTdL,
    })
}

/// Calculate the eigenvalues of the Hessian (for diagonal matrix)
pub fn calculate_eigenvalues(hessian: &Hessian3x3) -> (FixedPoint, FixedPoint, FixedPoint) {
    // For diagonal matrix, eigenvalues are the diagonal elements
    (
        FixedPoint::from_scaled(hessian.d2V_dS2 as i128),
        FixedPoint::from_scaled(hessian.d2V_dT2 as i128),
        FixedPoint::from_scaled(hessian.d2V_dL2 as i128),
    )
}

/// Calculate condition number (ratio of largest to smallest eigenvalue)
pub fn calculate_condition_number(hessian: &Hessian3x3) -> Result<FixedPoint> {
    let (lambda_s, lambda_t, lambda_l) = calculate_eigenvalues(hessian);
    
    // Find max and min eigenvalues
    let max_eigen = lambda_s.value
        .max(lambda_t.value)
        .max(lambda_l.value);
    
    let min_eigen = lambda_s.value
        .min(lambda_t.value)
        .min(lambda_l.value);
    
    require!(min_eigen > 0, FeelsProtocolError::DivisionByZero);
    
    // Condition number = max / min
    FixedPoint::from_scaled(max_eigen).div(FixedPoint::from_scaled(min_eigen))
}

// ============================================================================
// Lipschitz Constant Estimation
// ============================================================================

/// Estimate Lipschitz constant for gradient field
/// L = max eigenvalue of Hessian (bounds gradient change rate)
pub fn estimate_lipschitz_constant(
    hessians: &[Hessian3x3],
) -> Result<u64> {
    let mut max_eigenvalue = 0u128;
    
    for hessian in hessians {
        let (lambda_s, lambda_t, lambda_l) = calculate_eigenvalues(hessian);
        
        let local_max = (lambda_s.value as u128)
            .max(lambda_t.value as u128)
            .max(lambda_l.value as u128);
        
        max_eigenvalue = max_eigenvalue.max(local_max);
    }
    
    // Convert to u64 with appropriate scaling
    Ok((max_eigenvalue / (1u128 << 32)) as u64)
}

// ============================================================================
// Hessian-Vector Products
// ============================================================================

/// Calculate Hessian-vector product: H · v
pub fn hessian_vector_product(
    hessian: &Hessian3x3,
    vector: &crate::logic::gradient::PositionDelta3D,
) -> Result<crate::logic::gradient::PositionDelta3D> {
    // For diagonal Hessian: H·v = (H_SS*v_S, H_TT*v_T, H_LL*v_L)
    
    let h_ss = FixedPoint::from_scaled(hessian.d2V_dS2 as i128);
    let h_tt = FixedPoint::from_scaled(hessian.d2V_dT2 as i128);
    let h_ll = FixedPoint::from_scaled(hessian.d2V_dL2 as i128);
    
    Ok(crate::logic::gradient::PositionDelta3D {
        dS: h_ss.mul(vector.dS)?,
        dT: h_tt.mul(vector.dT)?,
        dL: h_ll.mul(vector.dL)?,
    })
}

/// Calculate quadratic form: v^T · H · v
pub fn quadratic_form(
    hessian: &Hessian3x3,
    vector: &crate::logic::gradient::PositionDelta3D,
) -> Result<FixedPoint> {
    let hv = hessian_vector_product(hessian, vector)?;
    
    // v^T · (H·v) = v_S*(H·v)_S + v_T*(H·v)_T + v_L*(H·v)_L
    let term_s = vector.dS.mul(hv.dS)?;
    let term_t = vector.dT.mul(hv.dT)?;
    let term_l = vector.dL.mul(hv.dL)?;
    
    term_s.add(term_t)?.add(term_l)
}

// ============================================================================
// Hessian Interpolation
// ============================================================================

/// Interpolate Hessian between tick boundaries
pub fn interpolate_hessian(
    lower_hessian: &Hessian3x3,
    upper_hessian: &Hessian3x3,
    position_ratio: FixedPoint, // 0 = lower, 1 = upper
) -> Result<Hessian3x3> {
    let one_minus_ratio = FixedPoint::ONE.sub(position_ratio)?;
    
    // Linear interpolation for each element
    let interp_ss = interpolate_element(
        lower_hessian.d2V_dS2,
        upper_hessian.d2V_dS2,
        position_ratio,
        one_minus_ratio,
    )?;
    
    let interp_tt = interpolate_element(
        lower_hessian.d2V_dT2,
        upper_hessian.d2V_dT2,
        position_ratio,
        one_minus_ratio,
    )?;
    
    let interp_ll = interpolate_element(
        lower_hessian.d2V_dL2,
        upper_hessian.d2V_dL2,
        position_ratio,
        one_minus_ratio,
    )?;
    
    Ok(Hessian3x3 {
        d2V_dS2: interp_ss,
        d2V_dT2: interp_tt,
        d2V_dL2: interp_ll,
        // Cross terms remain zero
        d2V_dSdT: 0,
        d2V_dSdL: 0,
        d2V_dTdL: 0,
    })
}

fn interpolate_element(
    lower: u128,
    upper: u128,
    ratio: FixedPoint,
    one_minus_ratio: FixedPoint,
) -> Result<u128> {
    let lower_contrib = one_minus_ratio.mul(FixedPoint::from_scaled(lower as i128))?;
    let upper_contrib = ratio.mul(FixedPoint::from_scaled(upper as i128))?;
    
    let result = lower_contrib.add(upper_contrib)?;
    
    Ok(result.value as u128)
}

// ============================================================================
// Curvature Metrics
// ============================================================================

/// Calculate mean curvature (trace of Hessian)
pub fn calculate_mean_curvature(hessian: &Hessian3x3) -> FixedPoint {
    let trace = (hessian.d2V_dS2 + hessian.d2V_dT2 + hessian.d2V_dL2) as i128;
    FixedPoint::from_scaled(trace)
}

/// Calculate Gaussian curvature (determinant of Hessian)
/// For diagonal matrix: det(H) = H_SS * H_TT * H_LL
pub fn calculate_gaussian_curvature(hessian: &Hessian3x3) -> Result<FixedPoint> {
    let h_ss = FixedPoint::from_scaled(hessian.d2V_dS2 as i128);
    let h_tt = FixedPoint::from_scaled(hessian.d2V_dT2 as i128);
    let h_ll = FixedPoint::from_scaled(hessian.d2V_dL2 as i128);
    
    h_ss.mul(h_tt)?.mul(h_ll)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hessian_calculation() {
        let mut state = MarketState::default();
        state.S = 2u128 << 64; // 2.0
        state.T = 1u128 << 64; // 1.0
        state.L = 1u128 << 64; // 1.0
        state.w_s = 3333;
        state.w_t = 3333;
        state.w_l = 3334;
        state.w_tau = 0;
        
        let position = Position3D::from_market_state(&state);
        let hessian = calculate_hessian_3x3(&state, &position).unwrap();
        
        // Verify positive definite
        assert!(verify_positive_definite(&hessian).unwrap());
        
        // Verify diagonal dominance (cross terms are zero)
        assert_eq!(hessian.d2V_dSdT, 0);
        assert_eq!(hessian.d2V_dSdL, 0);
        assert_eq!(hessian.d2V_dTdL, 0);
    }
    
    #[test]
    fn test_condition_number() {
        let hessian = Hessian3x3 {
            d2V_dS2: 100 << 64,
            d2V_dT2: 50 << 64,
            d2V_dL2: 25 << 64,
            d2V_dSdT: 0,
            d2V_dSdL: 0,
            d2V_dTdL: 0,
        };
        
        let cond = calculate_condition_number(&hessian).unwrap();
        // Condition number should be 100/25 = 4
        assert_eq!(cond.to_u64(), 4);
    }
}