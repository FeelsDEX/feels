//! # Work Calculations
//! 
//! Thermodynamic work calculations for trading paths.
//! W = V(P₂) - V(P₁)

use crate::errors::CoreResult;
use crate::constants::{Q64, BPS_DENOMINATOR};
use crate::math::safe_math::{safe_add_i128, safe_sub_i128, safe_mul_i128, safe_div_i128, safe_add_u128, safe_sub_u128, safe_div_u128, safe_mul_u128};
use crate::types::{Position3D, PathSegment, WorkResult};
use crate::physics::potential::{MarketField, calculate_potential_linear};

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Calculate total work along a trading path
pub fn calculate_path_work(
    segments: &[PathSegment],
    field: &MarketField,
) -> CoreResult<WorkResult> {
    let mut total_work = 0i128;
    let mut total_unsigned = 0u128;
    let mut uphill_work = 0u128;
    let mut downhill_work = 0u128;
    
    for segment in segments {
        // Calculate work for this segment
        let segment_work = calculate_segment_work(segment, field)?;
        
        // Accumulate signed and unsigned totals
        total_work = safe_add_i128(total_work, segment_work)?;
        total_unsigned = safe_add_u128(total_unsigned, segment_work.unsigned_abs() as u128)?;
        
        // Split into uphill/downhill
        if segment_work > 0 {
            uphill_work = safe_add_u128(uphill_work, segment_work as u128)?;
        } else {
            downhill_work = safe_add_u128(downhill_work, (-segment_work) as u128)?;
        }
    }
    
    // Calculate weighted average
    let weighted_work = if segments.is_empty() {
        0
    } else {
        safe_div_u128(total_unsigned, segments.len() as u128)?
    };
    
    Ok(WorkResult {
        total_work: total_unsigned,
        net_work: total_work,
        weighted_work,
        segments: segments.len() as u32,
    })
}

/// Calculate work for a single segment
pub fn calculate_segment_work(
    segment: &PathSegment,
    field: &MarketField,
) -> CoreResult<i128> {
    // Calculate potential at start and end
    let v_start = calculate_potential_linear(&segment.start, field)?;
    let v_end = calculate_potential_linear(&segment.end, field)?;
    
    // Work = V(end) - V(start)
    safe_sub_i128(v_end, v_start)
}

/// Extended work result with detailed breakdown
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct DetailedWorkResult {
    /// Basic work result
    pub basic: WorkResult,
    /// Uphill work component
    pub uphill_work: u128,
    /// Downhill work component
    pub downhill_work: u128,
    /// Work per segment
    pub segment_works: Vec<i128>,
    /// Estimated fee in basis points
    pub estimated_fee_bps: u64,
    /// Maximum rebate in basis points
    pub max_rebate_bps: u64,
}

/// Calculate detailed work with fee/rebate estimates
pub fn calculate_detailed_work(
    segments: &[PathSegment],
    field: &MarketField,
    base_fee_bps: u16,
    max_surcharge_bps: u16,
    rebate_eta: u32, // Rebate participation (basis points)
) -> CoreResult<DetailedWorkResult> {
    let mut segment_works = Vec::new();
    let mut total_work = 0i128;
    let mut uphill_work = 0u128;
    let mut downhill_work = 0u128;
    
    for segment in segments {
        let work = calculate_segment_work(segment, field)?;
        segment_works.push(work);
        
        total_work = safe_add_i128(total_work, work)?;
        if work > 0 {
            uphill_work = safe_add_u128(uphill_work, work as u128)?;
        } else {
            downhill_work = safe_add_u128(downhill_work, (-work) as u128)?;
        }
    }
    
    // Estimate fees and rebates
    let (fee_bps, rebate_bps) = estimate_fee_rebate(
        uphill_work,
        downhill_work,
        base_fee_bps,
        max_surcharge_bps,
        rebate_eta,
    )?;
    
    let basic = WorkResult {
        total_work: uphill_work.saturating_add(downhill_work),
        net_work: total_work,
        weighted_work: if segments.is_empty() { 0 } else { 
            (uphill_work.saturating_add(downhill_work)) / segments.len() as u128 
        },
        segments: segments.len() as u32,
    };
    
    Ok(DetailedWorkResult {
        basic,
        uphill_work,
        downhill_work,
        segment_works,
        estimated_fee_bps: fee_bps,
        max_rebate_bps: rebate_bps,
    })
}

/// Estimate fee and rebate from work components
fn estimate_fee_rebate(
    uphill_work: u128,
    downhill_work: u128,
    base_fee_bps: u16,
    max_surcharge_bps: u16,
    rebate_eta: u32,
) -> CoreResult<(u64, u64)> {
    // Fee estimation (simplified)
    let surcharge = if uphill_work > Q64 {
        // Scale work to basis points
        let work_bps = safe_div_u128(
            safe_mul_u128(uphill_work, BPS_DENOMINATOR as u128)?,
            Q64
        )?;
        work_bps.min(max_surcharge_bps as u128)
    } else {
        0
    };
    
    let total_fee = (base_fee_bps as u128).saturating_add(surcharge);
    
    // Rebate estimation
    let rebate = if downhill_work > 0 && rebate_eta > 0 {
        let work_bps = safe_div_u128(
            safe_mul_u128(downhill_work, BPS_DENOMINATOR as u128)?,
            Q64
        )?;
        // Apply participation rate
        safe_div_u128(
            safe_mul_u128(work_bps, rebate_eta as u128)?,
            BPS_DENOMINATOR as u128
        )?
    } else {
        0
    };
    
    Ok((total_fee as u64, rebate as u64))
}

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use crate::math::fixed_point::ln_q64;
    use crate::physics::potential::advanced::calculate_potential_logarithmic;
    
    /// Calculate work using logarithmic potential
    /// W = -ŵₛ ln(S₂/S₁) - ŵₜ ln(T₂/T₁) - ŵₗ ln(L₂/L₁)
    pub fn calculate_work_logarithmic(
        start: &Position3D,
        end: &Position3D,
        field: &MarketField,
    ) -> CoreResult<i128> {
        let (w_hat_s, w_hat_t, w_hat_l) = field.weights.get_hat_weights();
        
        // Calculate log ratios
        let ln_s_ratio = if end.S > start.S {
            ln_q64(safe_div_u128(end.S, start.S)?)?
        } else {
            -ln_q64(safe_div_u128(start.S, end.S)?)?
        };
        
        let ln_t_ratio = if end.T > start.T {
            ln_q64(safe_div_u128(end.T, start.T)?)?
        } else {
            -ln_q64(safe_div_u128(start.T, end.T)?)?
        };
        
        let ln_l_ratio = if end.L > start.L {
            ln_q64(safe_div_u128(end.L, start.L)?)?
        } else {
            -ln_q64(safe_div_u128(start.L, end.L)?)?
        };
        
        // Apply weights: -w * ln(ratio)
        let work_s = -safe_div_i128(
            safe_mul_i128(w_hat_s as i128, ln_s_ratio)?,
            10000 // weights in bps
        )?;
        
        let work_t = -safe_div_i128(
            safe_mul_i128(w_hat_t as i128, ln_t_ratio)?,
            10000
        )?;
        
        let work_l = -safe_div_i128(
            safe_mul_i128(w_hat_l as i128, ln_l_ratio)?,
            10000
        )?;
        
        // Total work
        safe_add_i128(work_s, safe_add_i128(work_t, work_l)?)
    }
    
    /// Calculate work with local quadratic approximation
    pub fn calculate_work_quadratic(
        segment: &PathSegment,
        coefficients: &LocalCoefficients,
    ) -> CoreResult<i128> {
        // W ≈ a*dx² + b*dx + c
        let dx_s = if segment.end.S > segment.start.S {
            safe_sub_u128(segment.end.S, segment.start.S)? as i128
        } else {
            -(safe_sub_u128(segment.start.S, segment.end.S)? as i128)
        };
        
        let dx_t = if segment.end.T > segment.start.T {
            safe_sub_u128(segment.end.T, segment.start.T)? as i128
        } else {
            -(safe_sub_u128(segment.start.T, segment.end.T)? as i128)
        };
        
        let dx_l = if segment.end.L > segment.start.L {
            safe_sub_u128(segment.end.L, segment.start.L)? as i128
        } else {
            -(safe_sub_u128(segment.start.L, segment.end.L)? as i128)
        };
        
        // Apply quadratic formula per dimension
        let work_s = apply_quadratic(dx_s, &coefficients.s_coeffs)?;
        let work_t = apply_quadratic(dx_t, &coefficients.t_coeffs)?;
        let work_l = apply_quadratic(dx_l, &coefficients.l_coeffs)?;
        
        safe_add_i128(work_s, safe_add_i128(work_t, work_l)?)
    }
    
    /// Local quadratic coefficients
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
    pub struct LocalCoefficients {
        pub s_coeffs: QuadraticCoeffs,
        pub t_coeffs: QuadraticCoeffs,
        pub l_coeffs: QuadraticCoeffs,
        pub valid_range: (u128, u128),
    }
    
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
    pub struct QuadraticCoeffs {
        pub a: i128, // x² coefficient
        pub b: i128, // x coefficient
        pub c: i128, // constant
    }
    
    fn apply_quadratic(dx: i128, coeffs: &QuadraticCoeffs) -> CoreResult<i128> {
        // ax² + bx + c
        let dx_squared = safe_div_i128(
            safe_mul_i128(dx, dx)?,
            Q64 as i128
        )?;
        
        let quadratic = safe_mul_i128(coeffs.a, dx_squared)?;
        let linear = safe_mul_i128(coeffs.b, dx)?;
        
        safe_add_i128(quadratic, safe_add_i128(linear, coeffs.c)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DomainWeights;
    
    #[test]
    fn test_segment_work() {
        let start = Position3D::new(100 * Q64, 100 * Q64, 100 * Q64);
        let end = Position3D::new(110 * Q64, 100 * Q64, 100 * Q64);
        
        let segment = PathSegment {
            start,
            end,
            distance: 10 * Q64,
            dimension: crate::types::TradeDimension::Spot,
        };
        
        let field = MarketField {
            s: Q64,
            t: Q64,
            l: Q64,
            weights: DomainWeights { w_s: 3333, w_t: 3333, w_l: 3334, w_tau: 0 },
            sigma_price: 100,
            sigma_rate: 50,
            sigma_leverage: 200,
        };
        
        let work = calculate_segment_work(&segment, &field).unwrap();
        assert!(work > 0); // Uphill work
    }
}