/// Client-side work calculator for optimal routing using field commitment data.
/// Implements closed-form log work calculation for the Feels Protocol.

use anchor_lang::prelude::*;
use integer_sqrt::IntegerSquareRoot;
use feels_core::{
    errors::extended::{ExtendedError as FeelsProtocolError, ExtendedResult as FeelsResult},
    constants::{Q64, BPS_DENOMINATOR},
    types::{
        market::extended::MarketFieldData,
        field::TradeDimension,
        Position3D, PathSegment as CorePathSegment, WorkResult as CoreWorkResult,
    },
    physics::{
        work::{
            calculate_path_work as core_calculate_path_work,
            calculate_detailed_work,
            advanced::{calculate_work_logarithmic, LocalCoefficients},
        },
        potential::{MarketField, calculate_potential_linear},
    },
    math::fixed_point::ln_q64,
};

// Import field commitment type
use crate::field_commitment::FieldCommitmentData;

// ============================================================================
// Work Calculation
// ============================================================================

/// Parameters for calculating work along a path
#[derive(Clone, Debug)]
pub struct PathWorkParams {
    /// Market field data
    pub field: MarketFieldData,
    
    /// Path segments to evaluate
    pub segments: Vec<PathSegment>,
    
    /// Optional field commitment for enhanced calculations
    pub field_commitment: Option<FieldCommitmentData>,
}

impl MarketFieldData {
    /// Convert to MarketField for physics calculations
    fn to_market_field(&self) -> MarketField {
        use feels_core::types::DomainWeights;
        
        MarketField {
            S: self.S,
            T: self.T,
            L: self.L,
            weights: DomainWeights {
                w_s: self.w_s,
                w_t: self.w_t,
                w_l: self.w_l,
                w_tau: self.w_tau,
            },
            sigma_price: self.sigma_price,
            sigma_rate: self.sigma_rate,
            sigma_leverage: self.sigma_leverage,
        }
    }
}

impl FieldCommitmentData {
    /// Convert to MarketField for physics calculations
    fn to_market_field(&self) -> MarketField {
        use feels_core::types::DomainWeights;
        
        MarketField {
            S: self.S,
            T: self.T,
            L: self.L,
            weights: DomainWeights {
                w_s: self.w_s,
                w_t: self.w_t,
                w_l: self.w_l,
                w_tau: self.w_tau,
            },
            sigma_price: self.sigma_price,
            sigma_rate: self.sigma_rate,
            sigma_leverage: self.sigma_leverage,
        }
    }
}

/// Path segment representing a single swap step
#[derive(Clone, Debug)]
pub struct PathSegment {
    /// Starting reserves
    pub reserve_0_start: u128,
    pub reserve_1_start: u128,
    
    /// Ending reserves
    pub reserve_0_end: u128,
    pub reserve_1_end: u128,
    
    /// Dimension being traded (spot, time, leverage)
    pub dimension: TradeDimension,
}

/// Convert SDK PathSegment to Core PathSegment
fn convert_to_core_segment(segment: &PathSegment, field: &MarketFieldData) -> Result<CorePathSegment, String> {
    // Calculate 3D positions from reserves
    let start = calculate_position_3d(
        segment.reserve_0_start,
        segment.reserve_1_start,
        field,
    )?;
    
    let end = calculate_position_3d(
        segment.reserve_0_end,
        segment.reserve_1_end,
        field,
    )?;
    
    // Calculate distance (simplified - Euclidean in 3D space)
    let ds = if end.S > start.S { end.S - start.S } else { start.S - end.S };
    let dt = if end.T > start.T { end.T - start.T } else { start.T - end.T };
    let dl = if end.L > start.L { end.L - start.L } else { start.L - end.L };
    
    let distance = ((ds*ds + dt*dt + dl*dl) as f64).sqrt() as u128;
    
    Ok(CorePathSegment {
        start,
        end,
        distance,
        dimension: segment.dimension.clone(),
    })
}

/// Calculate 3D position from reserves
fn calculate_position_3d(
    reserve_0: u128,
    reserve_1: u128,
    field: &MarketFieldData,
) -> Result<Position3D, String> {
    // Spot dimension: geometric mean of reserves
    let s = sqrt_u128(reserve_0 * reserve_1)?;
    
    // Time dimension: placeholder (would use duration metrics)
    let t = field.T;
    
    // Leverage dimension: placeholder (would use leverage metrics)
    let l = field.L;
    
    Ok(Position3D::new(s, t, l))
}

/// Result of work calculation
#[derive(Clone, Debug)]
pub struct WorkResult {
    /// Total work along path
    pub total_work: i128,
    
    /// Work per segment
    pub segment_work: Vec<i128>,
    
    /// Estimated fee (work mapped to token units)
    pub estimated_fee: u64,
    
    /// Maximum possible rebate
    pub max_rebate: u64,
}

/// Calculate work for a complete path
pub fn calculate_path_work(params: &PathWorkParams) -> Result<WorkResult, String> {
    // Convert to MarketField for physics calculations
    let market_field = if let Some(commitment) = &params.field_commitment {
        commitment.to_market_field()
    } else {
        params.field.to_market_field()
    };
    
    // Convert PathSegments to CorePathSegments
    let core_segments: Vec<CorePathSegment> = params.segments.iter()
        .map(|s| convert_to_core_segment(s, &params.field))
        .collect::<Result<Vec<_>, _>>()?;
    
    // Use feels-core's detailed work calculator
    let base_fee_bps = params.field_commitment
        .as_ref()
        .map(|c| c.base_fee_bps as u16)
        .unwrap_or(30);
    
    let detailed_result = calculate_detailed_work(
        &core_segments,
        &market_field,
        base_fee_bps,
        200, // max surcharge bps
        5000, // 50% rebate participation
    ).map_err(|e| format!("Work calculation error: {:?}", e))?;
    
    Ok(WorkResult {
        total_work: detailed_result.basic.net_work,
        segment_work: detailed_result.segment_works,
        estimated_fee: detailed_result.estimated_fee_bps,
        max_rebate: detailed_result.max_rebate_bps,
    })
}

/// Calculate work for a single segment
fn calculate_segment_work(
    segment: &PathSegment,
    field: &MarketFieldData,
    w_hat_s: u64,
    w_hat_t: u64,
    w_hat_l: u64,
) -> Result<i128, String> {
    match segment.dimension {
        TradeDimension::Spot => {
            // Calculate S_start and S_end from reserves
            let S_start = calculate_spot_value(
                segment.reserve_0_start,
                segment.reserve_1_start,
                field.twap_0,
                field.twap_1,
                field.sigma_price,
            )?;
            
            let S_end = calculate_spot_value(
                segment.reserve_0_end,
                segment.reserve_1_end,
                field.twap_0,
                field.twap_1,
                field.sigma_price,
            )?;
            
            // W_s = -ŵ_s * ln(S_end/S_start)
            calculate_dimensional_work(S_start, S_end, w_hat_s)
        }
        
        TradeDimension::Time => {
            // For time dimension, would use lending/borrowing changes
            // Placeholder for now
            Ok(0)
        }
        
        TradeDimension::Leverage => {
            // For leverage dimension, would use position changes
            // Placeholder for now
            Ok(0)
        }
        
        TradeDimension::Mixed => {
            // Mixed trades affect multiple dimensions
            // Would sum contributions from each
            Ok(0)
        }
    }
}

/// Calculate spot value from reserves and prices
fn calculate_spot_value(
    reserve_0: u128,
    reserve_1: u128,
    twap_0: u128,
    twap_1: u128,
    sigma_price: u64,
) -> Result<u128, String> {
    // Value in numeraire
    let value_0 = (reserve_0 * twap_0) / Q64;
    let value_1 = (reserve_1 * twap_1) / Q64;
    
    // Geometric mean (for equal weights)
    let spot_value = sqrt_u128(value_0 * value_1)?;
    
    // Apply risk scaling: S / sqrt(1 + σ²)
    let risk_factor = calculate_risk_factor(sigma_price)?;
    Ok((spot_value * Q64) / risk_factor)
}

/// Calculate risk factor: sqrt(1 + σ²)
fn calculate_risk_factor(sigma_bps: u64) -> Result<u128, String> {
    // σ² in fixed point
    let sigma_squared = (sigma_bps as u128 * sigma_bps as u128 * Q64) / (BPS_DENOMINATOR * BPS_DENOMINATOR) as u128;
    
    // 1 + σ²
    let one_plus_sigma_sq = Q64 + sigma_squared;
    
    // sqrt(1 + σ²)
    sqrt_u128(one_plus_sigma_sq)
}

/// Calculate dimensional work: W = -ŵ * ln(end/start)
fn calculate_dimensional_work(start: u128, end: u128, weight: u64) -> Result<i128, String> {
    if start == 0 || end == 0 {
        return Ok(0);
    }
    
    // ln(end/start) = ln(end) - ln(start)
    let ln_ratio = ln_ratio(end, start)?;
    
    // Apply weight (convert from bps to fixed point)
    let weight_fp = (weight as i128 * Q64 as i128) / BPS_DENOMINATOR as i128;
    
    // W = -ŵ * ln(end/start)
    Ok(-((ln_ratio * weight_fp) >> 64))
}

/// Get normalized hat weights
fn get_hat_weights(field: &MarketFieldData) -> (u64, u64, u64) {
    let trade_total = (field.w_s + field.w_t + field.w_l) as u64;
    if trade_total == 0 {
        return (0, 0, 0);
    }
    
    let w_hat_s = (field.w_s as u64 * BPS_DENOMINATOR) / trade_total;
    let w_hat_t = (field.w_t as u64 * BPS_DENOMINATOR) / trade_total;
    let w_hat_l = (field.w_l as u64 * BPS_DENOMINATOR) / trade_total;
    
    (w_hat_s, w_hat_t, w_hat_l)
}

// ============================================================================
// Fee Mapping
// ============================================================================

/// Map work to fee in token units
fn map_work_to_fee(work: i128, field: &MarketFieldData) -> Result<u64, String> {
    if work <= 0 {
        return Ok(0);
    }
    
    // Simple linear mapping for now
    // In production, would use marginal price and curvature
    let fee_bps = (work.abs() as u64 * BPS_DENOMINATOR) / Q64;
    
    // Apply fee cap (e.g., 30 bps max)
    let max_fee_bps = 30;
    Ok(fee_bps.min(max_fee_bps))
}

/// Calculate maximum rebate
fn calculate_max_rebate(work: i128, field: &MarketFieldData) -> Result<u64, String> {
    if work >= 0 {
        return Ok(0);
    }
    
    // Rebate capped by tau weight
    let max_rebate_bps = (field.w_tau as u64 * work.abs() as u64) / (BPS_DENOMINATOR * Q64);
    
    // Apply protocol cap (e.g., 10 bps max)
    let protocol_max_rebate = 10;
    Ok(max_rebate_bps.min(protocol_max_rebate))
}

// ============================================================================
// Math Helpers
// ============================================================================

/// Calculate natural logarithm ratio ln(a/b)
fn ln_ratio(a: u128, b: u128) -> Result<i128, String> {
    if a == b {
        return Ok(0);
    }
    
    // Use feels-core's fixed-point logarithm
    if a > b {
        // Positive ln(a/b)
        let ratio = (a * Q64) / b;
        ln_q64(ratio).map_err(|e| format!("Logarithm error: {:?}", e))
    } else {
        // Negative ln(a/b) = -ln(b/a)
        let ratio = (b * Q64) / a;
        ln_q64(ratio).map(|v| -v).map_err(|e| format!("Logarithm error: {:?}", e))
    }
}

/// Integer square root using the integer-sqrt crate
fn sqrt_u128(n: u128) -> Result<u128, String> {
    Ok(n.integer_sqrt())
}

// ============================================================================
// Enhanced Work Calculation with Field Commitments
// ============================================================================

/// Enhanced segment work calculation using local coefficients
fn calculate_segment_work_enhanced(
    segment: &PathSegment,
    commitment: &FieldCommitmentData,
    w_hat_s: u64,
    w_hat_t: u64,
    w_hat_l: u64,
) -> Result<i128, String> {
    // If local coefficients are available, use quadratic approximation
    if let Some(coeffs) = &commitment.local_coefficients {
        match segment.dimension {
            TradeDimension::Spot => {
                // Calculate position change
                let delta_s = calculate_spot_delta(segment, commitment)?;
                
                // Quadratic work: W = c0 * delta + c1 * delta^2
                let linear_term = (coeffs.c0_s * delta_s) >> 64;
                let quadratic_term = (coeffs.c1_s * delta_s * delta_s) >> 128;
                
                Ok(linear_term + quadratic_term)
            }
            TradeDimension::Time => {
                let delta_t = calculate_time_delta(segment)?;
                let linear_term = (coeffs.c0_t * delta_t) >> 64;
                let quadratic_term = (coeffs.c1_t * delta_t * delta_t) >> 128;
                Ok(linear_term + quadratic_term)
            }
            TradeDimension::Leverage => {
                let delta_l = calculate_leverage_delta(segment)?;
                let linear_term = (coeffs.c0_l * delta_l) >> 64;
                let quadratic_term = (coeffs.c1_l * delta_l * delta_l) >> 128;
                Ok(linear_term + quadratic_term)
            }
            TradeDimension::Mixed => {
                // Sum contributions from all dimensions
                let work_s = calculate_segment_work_enhanced(
                    &PathSegment { dimension: TradeDimension::Spot, ..segment.clone() },
                    commitment, w_hat_s, w_hat_t, w_hat_l
                )?;
                let work_t = calculate_segment_work_enhanced(
                    &PathSegment { dimension: TradeDimension::Time, ..segment.clone() },
                    commitment, w_hat_s, w_hat_t, w_hat_l
                )?;
                let work_l = calculate_segment_work_enhanced(
                    &PathSegment { dimension: TradeDimension::Leverage, ..segment.clone() },
                    commitment, w_hat_s, w_hat_t, w_hat_l
                )?;
                Ok(work_s + work_t + work_l)
            }
        }
    } else {
        // Fallback to standard calculation
        let field_data = commitment.to_market_field_data();
        calculate_segment_work(segment, &field_data, w_hat_s, w_hat_t, w_hat_l)
    }
}

/// Calculate spot dimension delta
fn calculate_spot_delta(segment: &PathSegment, commitment: &FieldCommitmentData) -> Result<i128, String> {
    let S_start = calculate_spot_value(
        segment.reserve_0_start,
        segment.reserve_1_start,
        commitment.twap_0,
        commitment.twap_1,
        commitment.sigma_price,
    )?;
    
    let S_end = calculate_spot_value(
        segment.reserve_0_end,
        segment.reserve_1_end,
        commitment.twap_0,
        commitment.twap_1,
        commitment.sigma_price,
    )?;
    
    Ok((S_end as i128) - (S_start as i128))
}

/// Calculate time dimension delta (placeholder)
fn calculate_time_delta(segment: &PathSegment) -> Result<i128, String> {
    // Would calculate based on duration changes
    Ok(0)
}

/// Calculate leverage dimension delta (placeholder)
fn calculate_leverage_delta(segment: &PathSegment) -> Result<i128, String> {
    // Would calculate based on leverage position changes
    Ok(0)
}

// ============================================================================
// Router Integration
// ============================================================================

/// Find optimal path and calculate work
pub fn find_optimal_route(
    start_token: Pubkey,
    end_token: Pubkey,
    amount_in: u64,
    pools: Vec<PoolData>,
) -> Result<OptimalRoute, String> {
    // This would integrate with existing router logic
    // For now, return placeholder
    
    Ok(OptimalRoute {
        path: vec![],
        expected_out: 0,
        total_work: 0,
        total_fee: 0,
    })
}

#[derive(Clone, Debug)]
pub struct PoolData {
    pub pool_id: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub field: MarketFieldData,
}

#[derive(Clone, Debug)]
pub struct OptimalRoute {
    pub path: Vec<Pubkey>,
    pub expected_out: u64,
    pub total_work: i128,
    pub total_fee: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spot_work_calculation() {
        let field = MarketFieldData {
            S: Q64,
            T: Q64,
            L: Q64,
            w_s: 3333,  // 33.33%
            w_t: 3333,  // 33.33%
            w_l: 3334,  // 33.34%
            w_tau: 0,   // 0%
            sigma_price: 1000, // 10%
            sigma_rate: 500,   // 5%
            sigma_leverage: 1500, // 15%
            twap_0: Q64,
            twap_1: Q64,
        };
        
        let segment = PathSegment {
            reserve_0_start: 1000 * Q64,
            reserve_1_start: 1000 * Q64,
            reserve_0_end: 900 * Q64,
            reserve_1_end: 1111 * Q64,
            dimension: TradeDimension::Spot,
        };
        
        let params = PathWorkParams {
            field,
            segments: vec![segment],
            field_commitment: None,
        };
        
        let result = calculate_path_work(&params).unwrap();
        
        // Work should be positive (fee paid)
        assert!(result.total_work > 0);
        assert!(result.estimated_fee > 0);
    }
}