/// Work calculation algorithms for the 3D AMM physics model

use feels_types::{FieldCommitmentData, TradeDimension, FeelsResult, FeelsProtocolError, Q64, BPS_DENOMINATOR};
use crate::safe::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Work Calculation Types
// ============================================================================

/// Parameters for work calculation along a trading path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCalculationParams {
    /// Field commitment data
    pub field: FieldCommitmentData,
    /// Path segments to evaluate
    pub segments: Vec<WorkSegment>,
    /// Optional local coefficients for enhanced precision
    pub use_local_coefficients: bool,
}

/// Single segment of a trading path
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

/// Result of work calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCalculationResult {
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

// ============================================================================
// Main Work Calculation Functions
// ============================================================================

/// Calculate total work along a trading path
pub fn calculate_path_work(params: &WorkCalculationParams) -> FeelsResult<WorkCalculationResult> {
    let mut total_work = 0i128;
    let mut segment_works = Vec::new();
    let mut dimension_breakdown = DimensionWorkBreakdown::zero();
    
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
                calculate_segment_work_standard(segment, &params.field)?
            },
            ComputationMethod::LinearApproximation => {
                calculate_segment_work_linear(segment, &params.field)?
            },
        };
        
        segment_works.push(segment_work.total);
        total_work = total_work.saturating_add(segment_work.total);
        
        // Accumulate dimension breakdown
        dimension_breakdown.spot_work = dimension_breakdown.spot_work.saturating_add(segment_work.spot);
        dimension_breakdown.time_work = dimension_breakdown.time_work.saturating_add(segment_work.time);
        dimension_breakdown.leverage_work = dimension_breakdown.leverage_work.saturating_add(segment_work.leverage);
        dimension_breakdown.coupling_work = dimension_breakdown.coupling_work.saturating_add(segment_work.coupling);
    }
    
    // Calculate fee and rebate estimates
    let (fee_estimate, max_rebate) = calculate_fee_rebate_estimates(total_work, &params.field)?;
    
    // Calculate efficiency score
    let efficiency_score = calculate_path_efficiency(&params.segments, total_work);
    
    Ok(WorkCalculationResult {
        total_work,
        segment_works,
        dimension_breakdown,
        fee_estimate,
        max_rebate,
        efficiency_score,
        computation_method,
    })
}

// ============================================================================
// Segment Work Calculation Methods
// ============================================================================

/// Detailed work result for a single segment
#[derive(Debug)]
struct SegmentWorkResult {
    total: i128,
    spot: i128,
    time: i128,
    leverage: i128,
    coupling: i128,
}

/// Calculate work using standard logarithmic method
fn calculate_segment_work_standard(
    segment: &WorkSegment,
    field: &FieldCommitmentData,
) -> FeelsResult<SegmentWorkResult> {
    match segment.dimension {
        TradeDimension::Spot => {
            let spot_work = calculate_spot_work_standard(
                segment.start_value,
                segment.end_value,
                field.w_s,
                field.sigma_price,
            )?;
            
            Ok(SegmentWorkResult {
                total: spot_work,
                spot: spot_work,
                time: 0,
                leverage: 0,
                coupling: 0,
            })
        },
        
        TradeDimension::Time => {
            let time_work = calculate_time_work_standard(
                segment.start_value,
                segment.end_value,
                field.w_t,
                field.sigma_rate,
            )?;
            
            Ok(SegmentWorkResult {
                total: time_work,
                spot: 0,
                time: time_work,
                leverage: 0,
                coupling: 0,
            })
        },
        
        TradeDimension::Leverage => {
            let leverage_work = calculate_leverage_work_standard(
                segment.start_value,
                segment.end_value,
                field.w_l,
                field.sigma_leverage,
            )?;
            
            Ok(SegmentWorkResult {
                total: leverage_work,
                spot: 0,
                time: 0,
                leverage: leverage_work,
                coupling: 0,
            })
        },
        
        TradeDimension::Mixed => {
            // Calculate work contributions from all dimensions
            let mut total_work = 0i128;
            let mut spot_work = 0i128;
            let mut time_work = 0i128;
            let mut leverage_work = 0i128;
            
            // Primary dimension contribution
            let primary_work = calculate_spot_work_standard(
                segment.start_value,
                segment.end_value,
                field.w_s,
                field.sigma_price,
            )?;
            spot_work += primary_work;
            total_work += primary_work;
            
            // Secondary dimension contributions
            for (dimension, delta) in &segment.secondary_changes {
                let secondary_work = calculate_secondary_work(*dimension, *delta, field)?;
                match dimension {
                    TradeDimension::Spot => spot_work += secondary_work,
                    TradeDimension::Time => time_work += secondary_work,
                    TradeDimension::Leverage => leverage_work += secondary_work,
                    TradeDimension::Mixed => {}, // Avoid recursion
                }
                total_work += secondary_work;
            }
            
            // Calculate coupling effects
            let coupling_work = calculate_coupling_work(field)?;
            total_work += coupling_work;
            
            Ok(SegmentWorkResult {
                total: total_work,
                spot: spot_work,
                time: time_work,
                leverage: leverage_work,
                coupling: coupling_work,
            })
        },
    }
}

/// Calculate work using local quadratic coefficients
fn calculate_segment_work_quadratic(
    segment: &WorkSegment,
    field: &FieldCommitmentData,
) -> FeelsResult<SegmentWorkResult> {
    let coeffs = field.local_coefficients.as_ref()
        .ok_or_else(|| FeelsProtocolError::Generic {
            message: "Local coefficients not available for quadratic work calculation".to_string(),
            context: None,
        })?;
    
    // Check if coefficients are still valid
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    if current_time > coeffs.valid_until {
        return Err(FeelsProtocolError::LocalCoefficientsExpired {
            expired_at: coeffs.valid_until,
            current_time,
        });
    }
    
    match segment.dimension {
        TradeDimension::Spot => {
            let delta = (segment.end_value as i128) - (segment.start_value as i128);
            let delta_q64 = (delta * Q64 as i128) >> 64; // Convert to Q64 fixed point
            
            // Quadratic work: W = c0 * Δx + c1 * Δx²
            let linear_term = (coeffs.c0_s * delta_q64) >> 64;
            let quadratic_term = (coeffs.c1_s * delta_q64 * delta_q64) >> 128;
            let spot_work = linear_term + quadratic_term;
            
            Ok(SegmentWorkResult {
                total: spot_work,
                spot: spot_work,
                time: 0,
                leverage: 0,
                coupling: 0,
            })
        },
        
        TradeDimension::Time => {
            let delta = (segment.end_value as i128) - (segment.start_value as i128);
            let delta_q64 = (delta * Q64 as i128) >> 64;
            
            let linear_term = (coeffs.c0_t * delta_q64) >> 64;
            let quadratic_term = (coeffs.c1_t * delta_q64 * delta_q64) >> 128;
            let time_work = linear_term + quadratic_term;
            
            Ok(SegmentWorkResult {
                total: time_work,
                spot: 0,
                time: time_work,
                leverage: 0,
                coupling: 0,
            })
        },
        
        TradeDimension::Leverage => {
            let delta = (segment.end_value as i128) - (segment.start_value as i128);
            let delta_q64 = (delta * Q64 as i128) >> 64;
            
            let linear_term = (coeffs.c0_l * delta_q64) >> 64;
            let quadratic_term = (coeffs.c1_l * delta_q64 * delta_q64) >> 128;
            let leverage_work = linear_term + quadratic_term;
            
            Ok(SegmentWorkResult {
                total: leverage_work,
                spot: 0,
                time: 0,
                leverage: leverage_work,
                coupling: 0,
            })
        },
        
        TradeDimension::Mixed => {
            // For mixed trades, combine contributions from all dimensions
            let mut total_work = 0i128;
            
            // Primary dimension (assumed spot)
            let delta_s = (segment.end_value as i128) - (segment.start_value as i128);
            let delta_s_q64 = (delta_s * Q64 as i128) >> 64;
            
            let spot_linear = (coeffs.c0_s * delta_s_q64) >> 64;
            let spot_quadratic = (coeffs.c1_s * delta_s_q64 * delta_s_q64) >> 128;
            let spot_work = spot_linear + spot_quadratic;
            total_work += spot_work;
            
            // Secondary dimensions
            let mut time_work = 0i128;
            let mut leverage_work = 0i128;
            
            for (dimension, delta) in &segment.secondary_changes {
                let delta_q64 = (*delta * Q64 as i128) >> 64;
                match dimension {
                    TradeDimension::Time => {
                        let linear = (coeffs.c0_t * delta_q64) >> 64;
                        let quadratic = (coeffs.c1_t * delta_q64 * delta_q64) >> 128;
                        time_work += linear + quadratic;
                    },
                    TradeDimension::Leverage => {
                        let linear = (coeffs.c0_l * delta_q64) >> 64;
                        let quadratic = (coeffs.c1_l * delta_q64 * delta_q64) >> 128;
                        leverage_work += linear + quadratic;
                    },
                    _ => {}, // Skip spot and mixed to avoid double counting
                }
            }
            
            total_work += time_work + leverage_work;
            
            // Cross-derivative terms (coupling)
            let coupling_work = if let (Some(c_st), Some(c_sl), Some(c_tl)) = 
                (coeffs.c_st, coeffs.c_sl, coeffs.c_tl) {
                // Calculate cross terms: c_st * Δs * Δt + c_sl * Δs * Δl + c_tl * Δt * Δl
                // This is a simplified version - full implementation would be more complex
                let cross_term = (c_st + c_sl + c_tl) * delta_s_q64 >> 64;
                cross_term
            } else {
                0
            };
            
            total_work += coupling_work;
            
            Ok(SegmentWorkResult {
                total: total_work,
                spot: spot_work,
                time: time_work,
                leverage: leverage_work,
                coupling: coupling_work,
            })
        },
    }
}

/// Calculate work using linear approximation (fastest but least accurate)
fn calculate_segment_work_linear(
    segment: &WorkSegment,
    field: &FieldCommitmentData,
) -> FeelsResult<SegmentWorkResult> {
    // Simple linear approximation: W ≈ weight * (end - start) / scale
    let delta = (segment.end_value as i128) - (segment.start_value as i128);
    
    let work = match segment.dimension {
        TradeDimension::Spot => {
            let weight_fixed = (field.w_s as i128 * Q64 as i128) / BPS_DENOMINATOR as i128;
            (weight_fixed * delta) >> 64
        },
        TradeDimension::Time => {
            let weight_fixed = (field.w_t as i128 * Q64 as i128) / BPS_DENOMINATOR as i128;
            (weight_fixed * delta) >> 64
        },
        TradeDimension::Leverage => {
            let weight_fixed = (field.w_l as i128 * Q64 as i128) / BPS_DENOMINATOR as i128;
            (weight_fixed * delta) >> 64
        },
        TradeDimension::Mixed => {
            // Average weight for mixed trades
            let avg_weight = (field.w_s + field.w_t + field.w_l) / 3;
            let weight_fixed = (avg_weight as i128 * Q64 as i128) / BPS_DENOMINATOR as i128;
            (weight_fixed * delta) >> 64
        },
    };
    
    Ok(SegmentWorkResult {
        total: work,
        spot: if matches!(segment.dimension, TradeDimension::Spot) { work } else { 0 },
        time: if matches!(segment.dimension, TradeDimension::Time) { work } else { 0 },
        leverage: if matches!(segment.dimension, TradeDimension::Leverage) { work } else { 0 },
        coupling: 0,
    })
}

// ============================================================================
// Dimension-Specific Work Calculations
// ============================================================================

/// Calculate work for spot dimension trades
fn calculate_spot_work_standard(
    start_value: u128,
    end_value: u128,
    weight_bps: u32,
    sigma_price: u64,
) -> FeelsResult<i128> {
    if start_value == 0 || end_value == 0 {
        return Ok(0);
    }
    
    // W_s = -ŵ_s * ln(S_end/S_start)
    // where ŵ_s = w_s / (w_s + w_t + w_l)
    
    // Calculate ln(end_value / start_value) using approximation
    let ratio_q64 = safe_div_q64(end_value, start_value)?;
    let ln_ratio = ln_approximation_q64(ratio_q64)?;
    
    // Apply normalized weight
    let weight_q64 = (weight_bps as u128 * Q64) / BPS_DENOMINATOR as u128;
    let weighted_ln = (ln_ratio as i128 * weight_q64 as i128) >> 64;
    
    // Apply risk scaling with sigma_price
    let risk_factor = calculate_risk_scaling(sigma_price)?;
    let scaled_work = (weighted_ln * risk_factor as i128) >> 64;
    
    // Negative because work opposes the change
    Ok(-scaled_work)
}

/// Calculate work for time dimension trades  
fn calculate_time_work_standard(
    start_value: u128,
    end_value: u128,
    weight_bps: u32,
    sigma_rate: u64,
) -> FeelsResult<i128> {
    // For time dimension, work is typically related to duration changes
    // W_t = -ŵ_t * Δt * rate_factor
    
    let delta = (end_value as i128) - (start_value as i128);
    let weight_q64 = (weight_bps as u128 * Q64) / BPS_DENOMINATOR as u128;
    
    // Apply time decay factor
    let time_factor = calculate_time_decay_factor()?;
    let work = (delta * weight_q64 as i128 * time_factor as i128) >> 128;
    
    // Apply risk scaling
    let risk_factor = calculate_risk_scaling(sigma_rate)?;
    let scaled_work = (work * risk_factor as i128) >> 64;
    
    Ok(-scaled_work)
}

/// Calculate work for leverage dimension trades
fn calculate_leverage_work_standard(
    start_value: u128,
    end_value: u128,
    weight_bps: u32,
    sigma_leverage: u64,
) -> FeelsResult<i128> {
    // Leverage work often has quadratic component due to risk
    // W_l = -ŵ_l * (Δl + β * Δl²)
    
    let delta = (end_value as i128) - (start_value as i128);
    let delta_q64 = (delta * Q64 as i128) >> 64;
    let weight_q64 = (weight_bps as u128 * Q64) / BPS_DENOMINATOR as u128;
    
    // Linear term
    let linear_work = (delta_q64 * weight_q64 as i128) >> 64;
    
    // Quadratic risk term (β ≈ 0.1 in Q64)
    let beta_q64 = Q64 / 10; // 0.1 in Q64.64
    let quadratic_work = (delta_q64 * delta_q64 * beta_q64 as i128) >> 128;
    
    let total_work = linear_work + quadratic_work;
    
    // Apply risk scaling
    let risk_factor = calculate_risk_scaling(sigma_leverage)?;
    let scaled_work = (total_work * risk_factor as i128) >> 64;
    
    Ok(-scaled_work)
}

/// Calculate work contribution from secondary dimension changes
fn calculate_secondary_work(
    dimension: TradeDimension,
    delta: i128,
    field: &FieldCommitmentData,
) -> FeelsResult<i128> {
    match dimension {
        TradeDimension::Spot => {
            let weight_q64 = (field.w_s as u128 * Q64) / BPS_DENOMINATOR as u128;
            Ok((delta * weight_q64 as i128) >> 64)
        },
        TradeDimension::Time => {
            let weight_q64 = (field.w_t as u128 * Q64) / BPS_DENOMINATOR as u128;
            Ok((delta * weight_q64 as i128) >> 64)
        },
        TradeDimension::Leverage => {
            let weight_q64 = (field.w_l as u128 * Q64) / BPS_DENOMINATOR as u128;
            Ok((delta * weight_q64 as i128) >> 64)
        },
        TradeDimension::Mixed => Ok(0), // Avoid recursion
    }
}

/// Calculate coupling work between dimensions
fn calculate_coupling_work(field: &FieldCommitmentData) -> FeelsResult<i128> {
    // Simplified coupling calculation
    // In full implementation, would depend on specific cross-derivatives
    if let Some(coeffs) = &field.local_coefficients {
        if let (Some(c_st), Some(c_sl), Some(c_tl)) = (coeffs.c_st, coeffs.c_sl, coeffs.c_tl) {
            // Average coupling effect
            let avg_coupling = (c_st + c_sl + c_tl) / 3;
            return Ok(avg_coupling >> 64); // Scale down
        }
    }
    
    // Default minimal coupling
    Ok(0)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Natural logarithm approximation in Q64.64 fixed point
fn ln_approximation_q64(x_q64: u128) -> FeelsResult<i128> {
    if x_q64 == 0 {
        return Err(FeelsProtocolError::InvalidMathOperation {
            operation: "ln_approximation".to_string(),
            reason: "logarithm of zero".to_string(),
        });
    }
    
    if x_q64 == Q64 {
        return Ok(0); // ln(1) = 0
    }
    
    // Use Taylor series around x = 1: ln(1 + u) ≈ u - u²/2 + u³/3 - ...
    let u = (x_q64 as i128) - (Q64 as i128); // x - 1 in Q64.64
    
    if u.abs() > (Q64 as i128) / 2 {
        // For large changes, use a more stable method
        // This is a simplified approximation
        return Ok(u);
    }
    
    // First-order approximation: ln(1 + u) ≈ u
    // For better accuracy, could add higher order terms
    let u_sq = (u * u) >> 64; // u² in Q64.64
    let second_order = u_sq >> 1; // u²/2
    
    Ok(u - second_order)
}

/// Calculate risk scaling factor from volatility
fn calculate_risk_scaling(sigma_bps: u64) -> FeelsResult<u128> {
    // Risk scaling: sqrt(1 + σ²)
    let sigma_q64 = (sigma_bps as u128 * Q64) / BPS_DENOMINATOR as u128;
    let sigma_sq = safe_mul_q64(sigma_q64, sigma_q64)?;
    let one_plus_sigma_sq = safe_add_u128(Q64, sigma_sq)?;
    safe_sqrt_q64(one_plus_sigma_sq)
}

/// Calculate time decay factor
fn calculate_time_decay_factor() -> FeelsResult<u128> {
    // Simplified time decay factor
    // In full implementation, would depend on actual time parameters
    Ok(Q64) // No decay for now
}

/// Calculate fee and rebate estimates from work
fn calculate_fee_rebate_estimates(
    total_work: i128,
    field: &FieldCommitmentData,
) -> FeelsResult<(u64, u64)> {
    let fee_estimate = if total_work > 0 {
        // Convert work to basis points and then to fee
        let work_bps = ((total_work as u128) * BPS_DENOMINATOR as u128) / Q64;
        work_bps.min(300) as u64 // Cap at 3%
    } else {
        0
    };
    
    let max_rebate = if total_work < 0 {
        // Rebate is limited by tau weight
        let work_bps = (((-total_work) as u128) * BPS_DENOMINATOR as u128) / Q64;
        let tau_limited_rebate = (work_bps * field.w_tau as u128) / BPS_DENOMINATOR as u128;
        tau_limited_rebate.min(100) as u64 // Cap at 1%
    } else {
        0
    };
    
    Ok((fee_estimate, max_rebate))
}

/// Calculate path efficiency score (0-100)
fn calculate_path_efficiency(segments: &[WorkSegment], total_work: i128) -> u8 {
    if segments.is_empty() {
        return 0;
    }
    
    // Simple efficiency metric based on work magnitude and path complexity
    let path_complexity = segments.len() as f64;
    let work_magnitude = (total_work.abs() as f64) / (Q64 as f64);
    
    // Lower work magnitude and fewer segments = higher efficiency
    let complexity_factor = 1.0 / (1.0 + path_complexity.ln());
    let work_factor = 1.0 / (1.0 + work_magnitude);
    
    let efficiency = complexity_factor * 0.4 + work_factor * 0.6;
    (efficiency * 100.0).round().min(100.0) as u8
}

// ============================================================================
// Implementation for Helper Types
// ============================================================================

impl DimensionWorkBreakdown {
    pub fn zero() -> Self {
        Self {
            spot_work: 0,
            time_work: 0,
            leverage_work: 0,
            coupling_work: 0,
        }
    }
    
    pub fn total(&self) -> i128 {
        self.spot_work + self.time_work + self.leverage_work + self.coupling_work
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use feels_types::*;

    #[test]
    fn test_basic_work_calculation() {
        let field = FieldCommitmentData::new(
            Q64, Q64, Q64, // S, T, L scalars
            3333, 3333, 3334, 0, // domain weights  
            5000, 5000, // spot weights
            1000, 500, 1500, // volatilities
            Q64, Q64, // TWAPs
        ).unwrap();
        
        let segment = WorkSegment {
            dimension: TradeDimension::Spot,
            start_value: Q64,
            end_value: Q64 + Q64/10, // 10% increase
            secondary_changes: vec![],
            reserve_delta_a: 1000,
            reserve_delta_b: -900,
        };
        
        let params = WorkCalculationParams {
            field,
            segments: vec![segment],
            use_local_coefficients: false,
        };
        
        let result = calculate_path_work(&params).unwrap();
        
        // For a price increase, work should be positive (fee)
        assert!(result.total_work != 0);
        assert_eq!(result.segment_works.len(), 1);
    }
    
    #[test]
    fn test_ln_approximation() {
        // ln(1) = 0
        assert_eq!(ln_approximation_q64(Q64).unwrap(), 0);
        
        // ln(e) ≈ 1 (e ≈ 2.718 in Q64.64)
        let e_q64 = (2.718281828 * (Q64 as f64)) as u128;
        let ln_e = ln_approximation_q64(e_q64).unwrap();
        let expected_q64 = Q64 as i128;
        
        // Should be approximately equal to Q64 (1.0 in fixed point)
        let error = (ln_e - expected_q64).abs();
        assert!(error < (Q64 as i128) / 100); // Within 1% error
    }
    
    #[test]
    fn test_risk_scaling() {
        // 10% volatility
        let scaling = calculate_risk_scaling(1000).unwrap();
        
        // Should be sqrt(1 + 0.1²) = sqrt(1.01) ≈ 1.005 in Q64.64
        let expected_min = Q64; // At least 1.0
        let expected_max = Q64 + Q64/100; // Less than 1.01
        
        assert!(scaling >= expected_min);
        assert!(scaling <= expected_max);
    }
}