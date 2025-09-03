/// Work calculation module implementing the unified fee model's work computation.
/// Provides approximations for on-chain work calculation and interfaces for off-chain exact computation.

use anchor_lang::prelude::*;
use crate::state::{
    FeelsProtocolError, MarketField, FieldCommitment, MarketManager,
};
use crate::constant::{Q64, BASIS_POINTS_DENOMINATOR as BPS_DENOMINATOR};

// ============================================================================
// Work Types
// ============================================================================

/// Work calculation result with detailed breakdown
#[derive(Debug, Clone, Default)]
pub struct WorkResult {
    /// Total work done (negative = price improvement)
    pub total_work: i128,
    
    /// Work components by dimension
    pub spot_work: i128,
    pub time_work: i128,
    pub leverage_work: i128,
    
    /// Weighted work (after applying domain weights)
    pub weighted_work: i128,
    
    /// Path information
    pub path_length: u128,
    pub path_curvature: u128,
    
    /// Confidence in calculation (0-10000 bps)
    pub confidence: u64,
}

/// Position in 3D market space
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
pub struct Position3D {
    pub S: u128,  // Spot scalar
    pub T: u128,  // Time scalar
    pub L: u128,  // Leverage scalar
}

/// Path segment for work integration
#[derive(Debug, Clone)]
pub struct PathSegment {
    pub start: Position3D,
    pub end: Position3D,
    pub liquidity: u128,
    pub distance: u128,
}

// ============================================================================
// Work Calculation Functions
// ============================================================================

/// Calculate work for a complete path through 3D market space
pub fn calculate_path_work(
    segments: &[PathSegment],
    field: &MarketField,
) -> Result<WorkResult> {
    let mut result = WorkResult::default();
    
    // Get normalized weights
    let (w_hat_s, w_hat_t, w_hat_l) = field.get_hat_weights();
    
    // Process each segment
    for segment in segments {
        let segment_work = calculate_segment_work(segment, w_hat_s, w_hat_t, w_hat_l)?;
        
        // Accumulate work components
        result.spot_work = result.spot_work.saturating_add(segment_work.spot_work);
        result.time_work = result.time_work.saturating_add(segment_work.time_work);
        result.leverage_work = result.leverage_work.saturating_add(segment_work.leverage_work);
        
        // Track path metrics
        result.path_length = result.path_length.saturating_add(segment.distance);
    }
    
    // Calculate total work
    result.total_work = result.spot_work
        .saturating_add(result.time_work)
        .saturating_add(result.leverage_work);
    
    // Apply domain weights to get weighted work
    result.weighted_work = calculate_weighted_work(&result, w_hat_s, w_hat_t, w_hat_l)?;
    
    // Estimate confidence based on path characteristics
    result.confidence = estimate_work_confidence(segments, field)?;
    
    Ok(result)
}

/// Calculate work for a single path segment
fn calculate_segment_work(
    segment: &PathSegment,
    w_hat_s: u64,
    w_hat_t: u64,
    w_hat_l: u64,
) -> Result<WorkResult> {
    let mut result = WorkResult::default();
    
    // Calculate work using linear approximation of ln
    // W = -ŵ · ln(end/start) ≈ -ŵ · (end/start - 1) for small changes
    
    // Spot dimension
    if segment.start.S != segment.end.S && w_hat_s > 0 {
        let work_s = approximate_log_work(segment.end.S, segment.start.S, segment.liquidity)?;
        result.spot_work = apply_dimension_weight(work_s, w_hat_s)?;
    }
    
    // Time dimension
    if segment.start.T != segment.end.T && w_hat_t > 0 {
        let work_t = approximate_log_work(segment.end.T, segment.start.T, segment.liquidity)?;
        result.time_work = apply_dimension_weight(work_t, w_hat_t)?;
    }
    
    // Leverage dimension
    if segment.start.L != segment.end.L && w_hat_l > 0 {
        let work_l = approximate_log_work(segment.end.L, segment.start.L, segment.liquidity)?;
        result.leverage_work = apply_dimension_weight(work_l, w_hat_l)?;
    }
    
    result.total_work = result.spot_work
        .saturating_add(result.time_work)
        .saturating_add(result.leverage_work);
    
    Ok(result)
}

/// Approximate logarithmic work using Taylor expansion
/// ln(b/a) ≈ (b-a)/a - (b-a)²/(2a²) + (b-a)³/(3a³) - ...
fn approximate_log_work(end_value: u128, start_value: u128, liquidity: u128) -> Result<i128> {
    if start_value == 0 {
        return Err(FeelsProtocolError::MathOverflow.into());
    }
    
    // Scale liquidity factor
    let liquidity_factor = (liquidity.min(u128::MAX / Q64) * Q64) / (1u128 << 64);
    
    if end_value > start_value {
        // Price increasing (negative work)
        let delta = end_value.saturating_sub(start_value);
        
        // First order: (b-a)/a
        let first_order = (delta as i128 * Q64 as i128) / start_value as i128;
        
        // Second order correction: -(b-a)²/(2a²)
        let delta_sq = (delta as i128).saturating_mul(delta as i128);
        let second_order = delta_sq
            .saturating_div(2)
            .saturating_div(start_value as i128)
            .saturating_div(start_value as i128);
        
        // Apply liquidity scaling
        let raw_work = first_order.saturating_sub(second_order);
        let scaled_work = (raw_work.saturating_mul(liquidity_factor as i128)) / Q64 as i128;
        
        Ok(-scaled_work) // Negative because W = -ln(ratio)
    } else {
        // Price decreasing (positive work)
        let delta = start_value.saturating_sub(end_value);
        
        // First order: -(a-b)/a = (b-a)/a
        let first_order = -(delta as i128 * Q64 as i128) / start_value as i128;
        
        // Second order correction
        let delta_sq = (delta as i128).saturating_mul(delta as i128);
        let second_order = delta_sq
            .saturating_div(2)
            .saturating_div(start_value as i128)
            .saturating_div(start_value as i128);
        
        // Apply liquidity scaling
        let raw_work = first_order.saturating_add(second_order);
        let scaled_work = (raw_work.saturating_mul(liquidity_factor as i128)) / Q64 as i128;
        
        Ok(-scaled_work)
    }
}

/// Apply dimension weight to work component
fn apply_dimension_weight(work: i128, weight_bps: u64) -> Result<i128> {
    let weighted = (work as i128)
        .saturating_mul(weight_bps as i128)
        .saturating_div(BPS_DENOMINATOR as i128);
    Ok(weighted)
}

/// Calculate total weighted work
fn calculate_weighted_work(
    result: &WorkResult,
    _w_hat_s: u64,
    _w_hat_t: u64,
    _w_hat_l: u64,
) -> Result<i128> {
    // Already weighted in segment calculation, just sum
    Ok(result.spot_work
        .saturating_add(result.time_work)
        .saturating_add(result.leverage_work))
}

/// Estimate confidence in work calculation
fn estimate_work_confidence(
    segments: &[PathSegment],
    field: &MarketField,
) -> Result<u64> {
    let mut confidence = 10000u64; // Start at 100%
    
    // Reduce confidence for long paths
    if segments.len() > 10 {
        confidence = confidence.saturating_sub(1000); // -10%
    }
    
    // Reduce confidence for large position changes
    for segment in segments {
        let spot_change = calculate_relative_change(segment.end.S, segment.start.S)?;
        let time_change = calculate_relative_change(segment.end.T, segment.start.T)?;
        let leverage_change = calculate_relative_change(segment.end.L, segment.start.L)?;
        
        // If any dimension changes by more than 10%, reduce confidence
        if spot_change > 1000 || time_change > 1000 || leverage_change > 1000 {
            confidence = confidence.saturating_sub(2000); // -20%
        }
    }
    
    // Reduce confidence if field is stale
    let current_time = Clock::get()?.unix_timestamp;
    let staleness = current_time - field.snapshot_ts;
    if staleness > field.max_staleness / 2 {
        confidence = confidence.saturating_sub(1500); // -15%
    }
    
    Ok(confidence.max(1000)) // Minimum 10% confidence
}

/// Calculate relative change in basis points
fn calculate_relative_change(new_value: u128, old_value: u128) -> Result<u64> {
    if old_value == 0 {
        return Ok(10000); // 100% change
    }
    
    let change = if new_value > old_value {
        ((new_value - old_value) * BPS_DENOMINATOR as u128) / old_value
    } else {
        ((old_value - new_value) * BPS_DENOMINATOR as u128) / old_value
    };
    
    Ok(change.min(10000) as u64)
}

// ============================================================================
// Field Commitment Integration
// ============================================================================

/// Calculate work from field commitment data
pub fn calculate_work_from_field(
    field_commitment: &FieldCommitment,
    start_sqrt_price: u128,
    end_sqrt_price: u128,
    liquidity: u128,
) -> Result<WorkResult> {
    let mut result = WorkResult::default();
    
    // Convert sqrt prices to position scalars
    let start_s = sqrt_price_to_scalar(start_sqrt_price)?;
    let end_s = sqrt_price_to_scalar(end_sqrt_price)?;
    
    // Use field commitment scalars for T and L (simplified)
    let start_pos = Position3D {
        S: start_s,
        T: field_commitment.T,
        L: field_commitment.L,
    };
    
    let end_pos = Position3D {
        S: end_s,
        T: field_commitment.T,
        L: field_commitment.L,
    };
    
    // Create single segment
    let segment = PathSegment {
        start: start_pos,
        end: end_pos,
        liquidity,
        distance: calculate_position_distance(&start_pos, &end_pos)?,
    };
    
    // Get weights from field commitment
    let (w_hat_s, w_hat_t, w_hat_l) = get_normalized_weights(
        field_commitment.w_s,
        field_commitment.w_t,
        field_commitment.w_l,
    );
    
    // Calculate work
    let segment_work = calculate_segment_work(&segment, w_hat_s, w_hat_t, w_hat_l)?;
    
    result.spot_work = segment_work.spot_work;
    result.time_work = segment_work.time_work;
    result.leverage_work = segment_work.leverage_work;
    result.total_work = segment_work.total_work;
    result.weighted_work = segment_work.total_work;
    result.path_length = segment.distance;
    result.confidence = 8000; // 80% confidence for field-based calculation
    
    Ok(result)
}

/// Convert sqrt price to scalar for work calculation
fn sqrt_price_to_scalar(sqrt_price: u128) -> Result<u128> {
    // S = price = (sqrt_price)² / Q96²
    let price = (sqrt_price as u128)
        .saturating_mul(sqrt_price as u128)
        .saturating_div(1u128 << 96)
        .saturating_div(1u128 << 96);
    
    // Convert to Q64 format
    let scalar = price.saturating_mul(Q64).saturating_div(1u128 << 64);
    
    Ok(scalar.max(1)) // Ensure non-zero
}

/// Calculate Euclidean distance between positions
fn calculate_position_distance(start: &Position3D, end: &Position3D) -> Result<u128> {
    // Simplified distance calculation
    // d = sqrt((S2-S1)² + (T2-T1)² + (L2-L1)²)
    
    let ds = if end.S > start.S {
        end.S - start.S
    } else {
        start.S - end.S
    };
    
    let dt = if end.T > start.T {
        end.T - start.T
    } else {
        start.T - end.T
    };
    
    let dl = if end.L > start.L {
        end.L - start.L
    } else {
        start.L - end.L
    };
    
    // Approximate distance (avoid sqrt on-chain)
    let distance = ds.saturating_add(dt).saturating_add(dl);
    
    Ok(distance)
}

/// Get normalized weights from basis points
fn get_normalized_weights(w_s: u32, w_t: u32, w_l: u32) -> (u64, u64, u64) {
    let total = (w_s + w_t + w_l) as u64;
    if total == 0 {
        return (3333, 3333, 3334); // Equal weights
    }
    
    let w_hat_s = ((w_s as u64) * (BPS_DENOMINATOR as u64)) / total;
    let w_hat_t = ((w_t as u64) * (BPS_DENOMINATOR as u64)) / total;
    let w_hat_l = ((w_l as u64) * (BPS_DENOMINATOR as u64)) / total;
    
    (w_hat_s, w_hat_t, w_hat_l)
}

// ============================================================================
// Integration with Fees
// ============================================================================

/// Convert work to fee amount
pub fn work_to_fee(
    work: i128,
    amount_in: u64,
    base_fee_bps: u64,
) -> Result<u64> {
    // Positive work = fee, negative work = potential rebate
    if work <= 0 {
        return Ok(0); // No fee for price improvement
    }
    
    // Scale work to fee
    // fee = min(work * base_fee_bps / 10000, amount_in * max_fee_bps / 10000)
    let work_fee = (work as u128)
        .saturating_mul(base_fee_bps as u128)
        .saturating_div(BPS_DENOMINATOR as u128)
        .min(u64::MAX as u128) as u64;
    
    // Cap at maximum percentage of input
    let max_fee = (amount_in as u128)
        .saturating_mul(1000) // 10% max
        .saturating_div(BPS_DENOMINATOR as u128)
        .min(u64::MAX as u128) as u64;
    
    Ok(work_fee.min(max_fee))
}

/// Check if work qualifies for rebate
pub fn work_qualifies_for_rebate(
    work: i128,
    min_improvement_bps: u64,
) -> bool {
    // Negative work with sufficient magnitude qualifies
    if work >= 0 {
        return false;
    }
    
    // Check if improvement exceeds threshold
    let improvement = (-work as u128)
        .saturating_mul(BPS_DENOMINATOR as u128)
        .saturating_div(Q64);
    
    improvement >= min_improvement_bps as u128
}

// ============================================================================
// Tests
// ============================================================================

/// Work calculation tests
#[cfg(test)]
mod tests {
    use super::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approximate_log_work() {
        // Test price increase
        let work = approximate_log_work(110 * Q64, 100 * Q64, Q64).unwrap();
        assert!(work < 0); // Negative work for price increase
        
        // Test price decrease
        let work = approximate_log_work(90 * Q64, 100 * Q64, Q64).unwrap();
        assert!(work > 0); // Positive work for price decrease
    }

    #[test]
    fn test_work_to_fee() {
        let work = 1000_000; // Positive work
        let fee = work_to_fee(work, 1_000_000, 30).unwrap(); // 0.3% base fee
        
        // Fee should be proportional to work and base fee
        assert!(fee > 0);
        assert!(fee < 100_000); // Less than 10% max
    }

    #[test]
    fn test_position_distance() {
        let start = Position3D {
            S: 100 * Q64,
            T: 50 * Q64,
            L: 10 * Q64,
        };
        
        let end = Position3D {
            S: 110 * Q64,
            T: 55 * Q64,
            L: 12 * Q64,
        };
        
        let distance = calculate_position_distance(&start, &end).unwrap();
        assert!(distance > 0);
    }
}