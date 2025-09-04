//! # Work Calculations
//! 
//! Thermodynamic work calculations for the 3D AMM.
//! 
//! This module provides both basic work calculations for on-chain use
//! and advanced work calculation methods for off-chain use by the keeper.

use crate::types::{Position3D, PathSegment, WorkResult, DomainWeights, Gradient3D};
use crate::errors::CoreResult;
use crate::math::safe_math::*;
use crate::constants::{Q64, BPS_DENOMINATOR};

#[cfg(feature = "advanced")]
use crate::types::{TradeDimension, FieldCommitmentData};

/// Calculate total work along a trading path
pub fn calculate_path_work(
    segments: &[PathSegment],
    weights: &DomainWeights,
    _current_position: &Position3D,
) -> CoreResult<WorkResult> {
    if segments.is_empty() {
        return Ok(WorkResult::default());
    }
    
    let mut total_work = 0u128;
    let mut weighted_work = 0u128;
    let mut total_distance = 0u128;
    let mut net_work = 0i128;
    
    for segment in segments {
        // Calculate gradient at midpoint
        let gradient = calculate_gradient(&segment.start, &segment.end, weights)?;
        
        // Work = ∇V · displacement
        let segment_work = calculate_segment_work(segment, &gradient)?;
        
        // Accumulate work
        total_work = safe_add_u128(total_work, segment_work.unsigned_abs() as u128)?;
        net_work = safe_add_i128(net_work, segment_work)?;
        
        // Weight by distance for averaging
        let weighted = safe_mul_u128(segment_work.unsigned_abs() as u128, segment.distance)?;
        weighted_work = safe_add_u128(weighted_work, weighted)?;
        total_distance = safe_add_u128(total_distance, segment.distance)?;
    }
    
    // Calculate weighted average
    let avg_work = if total_distance > 0 {
        safe_div_u128(weighted_work, total_distance)?
    } else {
        0
    };
    
    Ok(WorkResult {
        total_work,
        net_work,
        weighted_work: avg_work,
        segments: segments.len() as u32,
    })
}

/// Calculate gradient of potential function at a position
pub fn calculate_gradient(
    start: &Position3D,
    end: &Position3D,
    weights: &DomainWeights,
) -> CoreResult<Gradient3D> {
    // Get normalized weights
    let (w_hat_s, w_hat_t, w_hat_l) = weights.get_hat_weights();
    
    // Calculate finite differences
    let ds = if end.S > start.S {
        safe_sub_u128(end.S, start.S)? as i128
    } else {
        -(safe_sub_u128(start.S, end.S)? as i128)
    };
    
    let dt = if end.T > start.T {
        safe_sub_u128(end.T, start.T)? as i128
    } else {
        -(safe_sub_u128(start.T, end.T)? as i128)
    };
    
    let dl = if end.L > start.L {
        safe_sub_u128(end.L, start.L)? as i128
    } else {
        -(safe_sub_u128(start.L, end.L)? as i128)
    };
    
    // Apply weights to get gradient components
    // ∇V = -ŵ / x (simplified from -ŵ ln(x))
    let grad_s = apply_weight_to_gradient(ds, w_hat_s)?;
    let grad_t = apply_weight_to_gradient(dt, w_hat_t)?;
    let grad_l = apply_weight_to_gradient(dl, w_hat_l)?;
    
    Ok(Gradient3D {
        grad_s,
        grad_t,
        grad_l,
    })
}

/// Calculate work for a single segment
pub fn calculate_segment_work(
    segment: &PathSegment,
    gradient: &Gradient3D,
) -> CoreResult<i128> {
    // Work = gradient · displacement
    let displacement = calculate_displacement(segment)?;
    
    // Dot product
    let work_s = safe_mul_i128(gradient.grad_s, displacement.s)?;
    let work_t = safe_mul_i128(gradient.grad_t, displacement.t)?;
    let work_l = safe_mul_i128(gradient.grad_l, displacement.l)?;
    
    let total_work = safe_add_i128(work_s, safe_add_i128(work_t, work_l)?)?;
    
    // Scale by distance
    let scaled_work = safe_div_i128(
        safe_mul_i128(total_work, segment.distance as i128)?,
        Q64 as i128
    )?;
    
    Ok(scaled_work)
}

/// Calculate displacement vector for a segment
fn calculate_displacement(segment: &PathSegment) -> CoreResult<Displacement3D> {
    let s = if segment.end.S > segment.start.S {
        safe_sub_u128(segment.end.S, segment.start.S)? as i128
    } else {
        -(safe_sub_u128(segment.start.S, segment.end.S)? as i128)
    };
    
    let t = if segment.end.T > segment.start.T {
        safe_sub_u128(segment.end.T, segment.start.T)? as i128
    } else {
        -(safe_sub_u128(segment.start.T, segment.end.T)? as i128)
    };
    
    let l = if segment.end.L > segment.start.L {
        safe_sub_u128(segment.end.L, segment.start.L)? as i128
    } else {
        -(safe_sub_u128(segment.start.L, segment.end.L)? as i128)
    };
    
    Ok(Displacement3D { s, t, l })
}

/// Apply weight to gradient component
fn apply_weight_to_gradient(component: i128, weight: u64) -> CoreResult<i128> {
    // Scale by weight (in basis points)
    let scaled = safe_mul_i128(component, weight as i128)?;
    safe_div_i128(scaled, 10_000)
}

// Helper types
struct Displacement3D {
    s: i128,
    t: i128,
    l: i128,
}

// ============================================================================
// Advanced Work Calculation Functions (for off-chain use)
// ============================================================================

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use serde::{Deserialize, Serialize};
    
    /// Parameters for advanced work calculation along a trading path
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WorkCalculationParams {
        /// Field commitment data
        pub field: FieldCommitmentData,
        /// Path segments to evaluate
        pub segments: Vec<WorkSegment>,
        /// Optional local coefficients for enhanced precision
        pub use_local_coefficients: bool,
    }
    
    /// Single segment of a trading path with detailed parameters
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WorkSegment {
        /// Primary trading dimension
        pub dimension: TradeDimension,
        /// Starting position in the primary dimension
        pub start_value: u128,
        /// Ending position in the primary dimension
        pub end_value: u128,
        /// Secondary dimension changes (for mixed trades)
        pub secondary_changes: Vec<(TradeDimension, i128)>,
        /// Reserve changes for spot trades
        pub reserve_delta_a: i128,
        pub reserve_delta_b: i128,
    }
    
    /// Detailed result of work calculation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DetailedWorkResult {
        /// Total work along the path (negative = rebate, positive = fee)
        pub total_work: i128,
        /// Work contribution from each segment
        pub segment_works: Vec<i128>,
        /// Breakdown by dimension
        pub dimension_breakdown: DimensionWorkBreakdown,
        /// Fee/rebate estimates
        pub fee_estimate: u64,
        pub max_rebate: u64,
        /// Path efficiency metrics
        pub efficiency_score: u8,
        /// Computation metadata
        pub computation_method: ComputationMethod,
    }
    
    /// Work breakdown by trading dimension
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DimensionWorkBreakdown {
        pub spot_work: i128,      // W_s component
        pub time_work: i128,      // W_t component  
        pub leverage_work: i128,  // W_l component
        pub coupling_work: i128,  // Cross-dimensional coupling effects
    }
    
    /// Method used for work computation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ComputationMethod {
        /// Standard logarithmic work calculation
        Standard,
        /// Enhanced with local quadratic coefficients
        LocalQuadratic,
        /// Simplified linear approximation
        LinearApproximation,
    }
    
    /// Calculate total work along a trading path with advanced methods
    pub fn calculate_path_work_advanced(params: &WorkCalculationParams) -> CoreResult<DetailedWorkResult> {
        let mut total_work = 0i128;
        let mut segment_works = Vec::new();
        let mut dimension_breakdown = DimensionWorkBreakdown {
            spot_work: 0,
            time_work: 0,
            leverage_work: 0,
            coupling_work: 0,
        };
        
        // Choose computation method based on available data
        let computation_method = if params.use_local_coefficients && 
                                   params.field.local_coefficients.is_some() {
            ComputationMethod::LocalQuadratic
        } else {
            ComputationMethod::Standard
        };
        
        // Calculate work for each segment
        for segment in &params.segments {
            let segment_work = match computation_method {
                ComputationMethod::LocalQuadratic => {
                    calculate_segment_work_quadratic(segment, &params.field)?
                },
                ComputationMethod::Standard => {
                    calculate_segment_work_logarithmic(segment, &params.field)?
                },
                ComputationMethod::LinearApproximation => {
                    calculate_segment_work_linear(segment, &params.field)?
                },
            };
            
            // Update breakdown based on dimension
            match segment.dimension {
                TradeDimension::Spot => dimension_breakdown.spot_work = safe_add_i128(dimension_breakdown.spot_work, segment_work)?,
                TradeDimension::Time => dimension_breakdown.time_work = safe_add_i128(dimension_breakdown.time_work, segment_work)?,
                TradeDimension::Leverage => dimension_breakdown.leverage_work = safe_add_i128(dimension_breakdown.leverage_work, segment_work)?,
                TradeDimension::Mixed => dimension_breakdown.coupling_work = safe_add_i128(dimension_breakdown.coupling_work, segment_work)?,
            }
            
            total_work = safe_add_i128(total_work, segment_work)?;
            segment_works.push(segment_work);
        }
        
        // Calculate fee/rebate estimates
        let (fee_estimate, max_rebate) = calculate_fee_rebate_estimates(total_work, &params.field)?;
        
        // Calculate efficiency score (0-100)
        let efficiency_score = calculate_path_efficiency(&segment_works, total_work)?;
        
        Ok(DetailedWorkResult {
            total_work,
            segment_works,
            dimension_breakdown,
            fee_estimate,
            max_rebate,
            efficiency_score,
            computation_method,
        })
    }
    
    /// Calculate work for a segment using logarithmic method
    fn calculate_segment_work_logarithmic(segment: &WorkSegment, field: &FieldCommitmentData) -> CoreResult<i128> {
        // W = -ŵ * ln(end_value/start_value)
        let ratio = if segment.end_value > segment.start_value {
            safe_div_u128(segment.end_value, segment.start_value)?
        } else {
            safe_div_u128(segment.start_value, segment.end_value)?
        };
        
        // Use fixed-point logarithm if available
        #[cfg(feature = "advanced")]
        {
            use crate::math::fixed_point::ln_q64;
            let ln_ratio = ln_q64(ratio)?;
            let weight = field.get_dimension_weight(segment.dimension)?;
            let weighted = safe_mul_i128(ln_ratio, weight as i128)?;
            
            // Negate if movement is downhill
            if segment.end_value < segment.start_value {
                Ok(-weighted)
            } else {
                Ok(weighted)
            }
        }
        
        #[cfg(not(feature = "advanced"))]
        {
            // Fallback to simple approximation
            Ok(0) // Simplified for on-chain
        }
    }
    
    /// Calculate work for a segment using quadratic approximation
    fn calculate_segment_work_quadratic(segment: &WorkSegment, field: &FieldCommitmentData) -> CoreResult<i128> {
        // Use local quadratic coefficients if available
        if let Some(coeffs) = &field.local_coefficients {
            // W = a*dx² + b*dx + c (local approximation)
            let dx = if segment.end_value > segment.start_value {
                safe_sub_u128(segment.end_value, segment.start_value)? as i128
            } else {
                -(safe_sub_u128(segment.start_value, segment.end_value)? as i128)
            };
            
            // Apply simplified linear approximation using available coefficients
            // Using linear coefficient c0_s as a simplified gradient estimate
            let gradient = match segment.dimension {
                TradeDimension::Spot => coeffs.c0_s,
                TradeDimension::Time => coeffs.c0_t,
                TradeDimension::Leverage => coeffs.c0_l,
                TradeDimension::Mixed => coeffs.c0_s, // Use spot as default for mixed
            };
            
            let work = safe_mul_i128(gradient, dx)?;
            Ok(work)
        } else {
            // Fallback to logarithmic method
            calculate_segment_work_logarithmic(segment, field)
        }
    }
    
    /// Calculate work for a segment using linear approximation
    fn calculate_segment_work_linear(segment: &WorkSegment, _field: &FieldCommitmentData) -> CoreResult<i128> {
        // Simple linear approximation: W ≈ gradient * displacement
        let displacement = if segment.end_value > segment.start_value {
            safe_sub_u128(segment.end_value, segment.start_value)? as i128
        } else {
            -(safe_sub_u128(segment.start_value, segment.end_value)? as i128)
        };
        
        // Use a simple gradient estimate
        let gradient_estimate = Q64 as i128 / 100; // 1% gradient
        safe_mul_i128(gradient_estimate, displacement)
    }
    
    /// Calculate fee and rebate estimates from work
    fn calculate_fee_rebate_estimates(total_work: i128, field: &FieldCommitmentData) -> CoreResult<(u64, u64)> {
        if total_work > 0 {
            // Uphill work → fee
            let fee_bps = safe_div_i128(safe_mul_i128(total_work, BPS_DENOMINATOR as i128)?, Q64 as i128)?;
            // Cap at reasonable maximum (e.g. 500 bps = 5%)
            let max_fee_bps = 500i128;
            let fee_estimate = fee_bps.min(max_fee_bps).max(0) as u64;
            Ok((fee_estimate, 0))
        } else {
            // Downhill work → rebate
            let rebate_bps = safe_div_i128(safe_mul_i128(-total_work, BPS_DENOMINATOR as i128)?, Q64 as i128)?;
            // Cap at reasonable maximum (e.g. 300 bps = 3%)
            let max_rebate_bps = 300i128;
            let max_rebate = rebate_bps.min(max_rebate_bps).max(0) as u64;
            Ok((0, max_rebate))
        }
    }
    
    /// Calculate path efficiency score (0-100)
    fn calculate_path_efficiency(segment_works: &[i128], total_work: i128) -> CoreResult<u8> {
        if segment_works.is_empty() || total_work == 0 {
            return Ok(100); // Perfect efficiency for null paths
        }
        
        // Calculate sum of absolute segment works
        let mut total_absolute = 0i128;
        for &work in segment_works {
            total_absolute = safe_add_i128(total_absolute, work.abs())?;
        }
        
        // Efficiency = |net_work| / sum(|segment_works|) * 100
        let efficiency_ratio = safe_div_i128(safe_mul_i128(total_work.abs(), 100)?, total_absolute)?;
        
        Ok(efficiency_ratio.min(100).max(0) as u8)
    }
    
    impl DimensionWorkBreakdown {
        pub fn zero() -> Self {
            Self {
                spot_work: 0,
                time_work: 0,
                leverage_work: 0,
                coupling_work: 0,
            }
        }
        
        pub fn total(&self) -> CoreResult<i128> {
            let sum = safe_add_i128(self.spot_work, self.time_work)?;
            let sum = safe_add_i128(sum, self.leverage_work)?;
            safe_add_i128(sum, self.coupling_work)
        }
    }
}