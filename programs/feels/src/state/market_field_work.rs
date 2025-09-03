/// Market field work calculation with proper fixed-point math implementation.
/// This module provides two options for work calculation:
/// - Option A: On-chain recompute with deterministic fixed-point ln
/// - Option B: Use local coefficients (no ln computation needed)

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::market_field::{MarketField, WorkCalculationParams};
// Note: ln_q64 calculations are performed off-chain in the keeper
// On-chain code receives pre-computed work values

// ============================================================================
// Feature Flags
// ============================================================================

/// Feature flag to select work calculation method per market
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum WorkCalculationMethod {
    /// Option A: On-chain recompute with fixed-point ln
    OnChainRecompute,
    /// Option B: Use local quadratic coefficients from field commitment
    LocalCoefficients,
}

// ============================================================================
// Work Calculation Implementation
// ============================================================================

/// Calculate work based on the selected method
pub fn calculate_work_for_market(
    params: &WorkCalculationParams,
    method: WorkCalculationMethod,
) -> Result<i128> {
    match method {
        WorkCalculationMethod::OnChainRecompute => {
            calculate_work_with_ln_recompute(params)
        }
        WorkCalculationMethod::LocalCoefficients => {
            // For Option B, work should come from field commitment coefficients
            // This path should not compute ln on-chain
            msg!("Local coefficients method requires field commitment with coefficients");
            Err(FeelsProtocolError::InvalidOperation.into())
        }
    }
}

/// Option A: Calculate work using on-chain fixed-point ln computation
fn calculate_work_with_ln_recompute(params: &WorkCalculationParams) -> Result<i128> {
    let (w_hat_s, w_hat_t, w_hat_l) = params.field.get_hat_weights();
    
    let mut work = 0i128;
    
    // Spot component
    if params.S_start != params.S_end && w_hat_s > 0 {
        let ln_ratio = calculate_ln_ratio_fixed_point(params.S_end, params.S_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_s)?;
        work = work.saturating_sub(w_component);
    }
    
    // Time component
    if params.T_start != params.T_end && w_hat_t > 0 {
        let ln_ratio = calculate_ln_ratio_fixed_point(params.T_end, params.T_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_t)?;
        work = work.saturating_sub(w_component);
    }
    
    // Leverage component
    if params.L_start != params.L_end && w_hat_l > 0 {
        let ln_ratio = calculate_ln_ratio_fixed_point(params.L_end, params.L_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_l)?;
        work = work.saturating_sub(w_component);
    }
    
    Ok(work)
}

/// Natural logarithm in Q64 fixed point using Taylor series approximation
/// Accurate for values near Q64 (1.0 in fixed point)
fn ln_q64(x: u128) -> Result<i128> {
    use crate::constant::Q64;
    
    if x == 0 {
        return Err(FeelsProtocolError::MathOverflow.into());
    }
    
    // For better accuracy, normalize x to be close to 1.0
    // Count how many times we need to divide/multiply by 2
    let mut normalized = x;
    let mut log2_count = 0i32;
    
    // Normalize to range [0.5, 1.0] in Q64
    while normalized > Q64 {
        normalized >>= 1;
        log2_count += 1;
    }
    while normalized < (Q64 >> 1) {
        normalized <<= 1;
        log2_count -= 1;
    }
    
    // Now normalized is in range [0.5, 1.0] in Q64 format
    // Calculate ln using Taylor series around 1: ln(1+z) = z - z²/2 + z³/3 - ...
    // Where z = (x - 1)
    
    let z = (normalized as i128) - (Q64 as i128);  // z in Q64 format
    
    // Calculate first few terms of Taylor series
    // z - z²/2 + z³/3 - z⁴/4
    let z2 = ((z as i128) * (z as i128)) >> 64;  // z² in Q64
    let z3 = ((z2 as i128) * (z as i128)) >> 64; // z³ in Q64
    let z4 = ((z3 as i128) * (z as i128)) >> 64; // z⁴ in Q64
    
    // Calculate terms with proper scaling
    let term1 = z;
    let term2 = z2 >> 1;  // z²/2
    let term3 = z3 / 3;   // z³/3
    let term4 = z4 >> 2;  // z⁴/4
    
    // Sum terms with alternating signs
    let ln_normalized = term1 - term2 + term3 - term4;
    
    // Add back the log2 adjustment: ln(x) = ln(normalized) + log2_count * ln(2)
    // ln(2) ≈ 0.693147 in Q64 format
    const LN2_Q64: i128 = 12786308645202655660; // 0.693147 * 2^64
    let log2_adjustment = (log2_count as i128) * LN2_Q64;
    
    let result = ln_normalized + log2_adjustment;
    
    Ok(result)
}

/// Calculate ln(a/b) using fixed-point Q64.64 arithmetic
fn calculate_ln_ratio_fixed_point(a: u128, b: u128) -> Result<i128> {
    // Validate inputs
    require!(a > 0 && b > 0, FeelsProtocolError::InvalidParameter);
    
    // For ln(a/b) = ln(a) - ln(b)
    let ln_a = ln_q64(a).map_err(|_| FeelsProtocolError::MathOverflow)?;
    let ln_b = ln_q64(b).map_err(|_| FeelsProtocolError::MathOverflow)?;
    
    // Return the difference
    ln_a.checked_sub(ln_b)
        .ok_or(FeelsProtocolError::MathOverflow.into())
}

/// Apply weight to value (same as original)
fn apply_weight(value: i128, weight: u64) -> Result<i128> {
    // weight is in basis points, convert to fixed point
    let weight_fp = (weight as i128 * (1i128 << 64)) / 10000;
    
    // Multiply and scale back
    let result = (value.saturating_mul(weight_fp)) >> 64;
    
    Ok(result)
}

// ============================================================================
// Field Commitment Extension
// ============================================================================

/// Extended field commitment that includes work calculation method
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FieldCommitmentWithMethod {
    /// Base field commitment data
    pub field: MarketField,
    
    /// Work calculation method for this market
    pub work_method: WorkCalculationMethod,
    
    /// Optional local quadratic coefficients (for Option B)
    pub local_coefficients: Option<LocalQuadraticCoeffs>,
}

/// Local quadratic coefficients for Option B
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LocalQuadraticCoeffs {
    /// Spot dimension coefficients [c0, c1]
    pub spot_coeffs: [i128; 2],
    
    /// Time dimension coefficients [c0, c1]
    pub time_coeffs: [i128; 2],
    
    /// Leverage dimension coefficients [c0, c1]
    pub leverage_coeffs: [i128; 2],
    
    /// Validity bounds for these coefficients
    pub valid_range_start: u128,
    pub valid_range_end: u128,
}

impl LocalQuadraticCoeffs {
    /// Calculate work using quadratic approximation
    /// W ≈ c0 * dx + 0.5 * c1 * dx^2
    pub fn calculate_work_quadratic(
        &self,
        delta_s: i128,
        delta_t: i128,
        delta_l: i128,
    ) -> Result<i128> {
        let mut work = 0i128;
        
        // Spot component
        if delta_s != 0 {
            let w_s = self.spot_coeffs[0]
                .saturating_mul(delta_s)
                .saturating_add(
                    self.spot_coeffs[1]
                        .saturating_mul(delta_s)
                        .saturating_mul(delta_s)
                        / 2
                );
            work = work.saturating_add(w_s);
        }
        
        // Time component
        if delta_t != 0 {
            let w_t = self.time_coeffs[0]
                .saturating_mul(delta_t)
                .saturating_add(
                    self.time_coeffs[1]
                        .saturating_mul(delta_t)
                        .saturating_mul(delta_t)
                        / 2
                );
            work = work.saturating_add(w_t);
        }
        
        // Leverage component
        if delta_l != 0 {
            let w_l = self.leverage_coeffs[0]
                .saturating_mul(delta_l)
                .saturating_add(
                    self.leverage_coeffs[1]
                        .saturating_mul(delta_l)
                        .saturating_mul(delta_l)
                        / 2
                );
            work = work.saturating_add(w_l);
        }
        
        Ok(work)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ln_ratio_calculation() {
        // Test ln(2/1) ≈ 0.693
        let a = 2u128 << 64; // 2 in Q64 format
        let b = 1u128 << 64; // 1 in Q64 format
        
        let ln_ratio = calculate_ln_ratio_fixed_point(a, b).unwrap();
        
        // ln(2) ≈ 0.693147
        // In Q64 format: 0.693147 * 2^64 ≈ 12786267502870756713
        let expected = 12786267502870756713i128;
        let tolerance = 1i128 << 54; // Allow ~0.00006 error
        
        assert!((ln_ratio - expected).abs() < tolerance);
    }
    
    #[test]
    fn test_work_calculation_methods() {
        let field = MarketField {
            S: 1u128 << 64,
            T: 1u128 << 64,
            L: 1u128 << 64,
            w_s: 5000,
            w_t: 3000,
            w_l: 2000,
            w_tau: 0,
            ..Default::default()
        };
        
        let params = WorkCalculationParams {
            S_start: 1u128 << 64,
            T_start: 1u128 << 64,
            L_start: 1u128 << 64,
            S_end: 2u128 << 64,
            T_end: 1u128 << 64,
            L_end: 1u128 << 64,
            field,
        };
        
        // Test Option A (on-chain recompute)
        let work_a = calculate_work_for_market(&params, WorkCalculationMethod::OnChainRecompute);
        assert!(work_a.is_ok());
        
        // Test Option B (should fail without coefficients)
        let work_b = calculate_work_for_market(&params, WorkCalculationMethod::LocalCoefficients);
        assert!(work_b.is_err());
    }
}