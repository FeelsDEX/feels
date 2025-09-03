/// Shared conservation check primitive for all rebase operations.
/// 
/// This module provides a single, unified conservation verification function
/// that ensures no value is created or destroyed during rebasing operations.
/// 
/// # Conservation Law
/// 
/// The fundamental conservation law for any rebase operation is:
/// ```
/// Σ w_i · ln(g_i) = 0
/// ```
/// 
/// Where:
/// - w_i: Domain weights (must sum to 10000 basis points)
/// - g_i: Growth factors (Q64 fixed-point format)
/// - ln: Natural logarithm
/// 
/// Since we cannot compute ln on-chain, the keeper must provide pre-computed
/// values that satisfy this constraint exactly.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::rebase::REBASE_INDEX_SCALE;
use crate::state::{BufferAccount, FieldCommitment};
use crate::constant::{Q64, BASIS_POINTS_DENOMINATOR as BPS_DENOMINATOR};

/// Conservation verification data provided by keeper
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConservationProof {
    /// Growth factors in Q64 format (1.0 = 2^64)
    pub growth_factors: Vec<u128>,
    
    /// Domain weights in basis points (must sum to 10000)
    pub weights: Vec<u32>,
    
    /// Pre-computed weighted log sum (should be ~0)
    /// This is Σ w_i · ln(g_i) computed off-chain
    pub weighted_log_sum: i64,
    
    /// Maximum acceptable deviation in basis points
    pub tolerance_bps: u32,
}

/// Shared conservation check result
#[derive(Debug)]
pub struct ConservationCheckResult {
    /// Whether conservation is satisfied within tolerance
    pub is_valid: bool,
    
    /// The computed weighted log sum (from proof)
    pub weighted_sum: i64,
    
    /// Maximum allowed deviation
    pub max_deviation: i64,
}

/// Verify conservation law is satisfied for a rebase operation
/// 
/// This function should be called by ALL rebase operations to ensure
/// value conservation. It verifies that:
/// 1. Weights sum to exactly 10000 (100%)
/// 2. All growth factors are positive
/// 3. The weighted log sum is within tolerance of zero
/// 
/// # Arguments
/// * `proof` - Conservation proof from keeper with pre-computed values
/// 
/// # Returns
/// * `ConservationCheckResult` with validation details
pub fn verify_conservation(proof: &ConservationProof) -> Result<ConservationCheckResult> {
    // Validate inputs match
    require!(
        proof.growth_factors.len() == proof.weights.len(),
        FeelsProtocolError::InvalidInput
    );
    
    require!(
        !proof.growth_factors.is_empty(),
        FeelsProtocolError::InvalidInput
    );
    
    // Verify weights sum to exactly 10000 basis points
    let weight_sum: u32 = proof.weights.iter().sum();
    require!(
        weight_sum == 10000,
        FeelsProtocolError::InvalidWeights
    );
    
    // Verify all growth factors are positive
    for &factor in &proof.growth_factors {
        require!(
            factor > 0,
            FeelsProtocolError::InvalidParameter
        );
    }
    
    // Check that the pre-computed weighted log sum is within tolerance
    let max_deviation = (proof.tolerance_bps as i64 * REBASE_INDEX_SCALE as i64) / 10000;
    let is_valid = proof.weighted_log_sum.abs() <= max_deviation;
    
    if !is_valid {
        msg!("Conservation violation detected!");
        msg!("  Weighted log sum: {}", proof.weighted_log_sum);
        msg!("  Max allowed deviation: {}", max_deviation);
        msg!("  Tolerance (bps): {}", proof.tolerance_bps);
    }
    
    Ok(ConservationCheckResult {
        is_valid,
        weighted_sum: proof.weighted_log_sum,
        max_deviation,
    })
}

/// Verify conservation for a specific rebase scenario
/// 
/// # Lending/Funding Conservation
/// For lending operations with buffer participation:
/// - Weights: [w_lenders, w_borrowers, w_buffer]
/// - Factors: [g_lenders, g_borrowers, g_buffer]
/// 
/// # Leverage Conservation
/// For leverage operations:
/// - Weights: [w_long, w_short]
/// - Factors: [g_long, g_short]
/// 
/// # Buffer Conservation
/// For buffer-only operations:
/// - Weights: [w_spot, w_time, w_leverage]
/// - Factors: [g_spot, g_time, g_leverage]
pub fn verify_rebase_conservation(
    operation_type: &str,
    proof: &ConservationProof,
) -> Result<()> {
    msg!("Verifying conservation for {} operation", operation_type);
    msg!("  Num domains: {}", proof.weights.len());
    
    // Log the configuration for debugging
    for (i, (weight, factor)) in proof.weights.iter().zip(proof.growth_factors.iter()).enumerate() {
        msg!("  Domain {}: weight={} bps, factor={}", i, weight, factor);
    }
    
    let result = verify_conservation(proof)?;
    
    require!(
        result.is_valid,
        FeelsProtocolError::ConstraintViolation
    );
    
    msg!("Conservation verified successfully");
    msg!("  Weighted sum: {}", result.weighted_sum);
    msg!("  Within tolerance: {} <= {}", result.weighted_sum.abs(), result.max_deviation);
    
    Ok(())
}

/// Helper to create conservation proof for common scenarios
pub mod builders {
    use super::*;
    
    /// Build conservation proof for lending rebase
    pub fn lending_rebase_proof(
        lender_weight: u32,
        borrower_weight: u32,
        buffer_weight: u32,
        lender_factor: u128,
        borrower_factor: u128,
        buffer_factor: u128,
        weighted_sum: i64,
    ) -> ConservationProof {
        ConservationProof {
            growth_factors: vec![lender_factor, borrower_factor, buffer_factor],
            weights: vec![lender_weight, borrower_weight, buffer_weight],
            weighted_log_sum: weighted_sum,
            tolerance_bps: 1, // 0.01% tolerance
        }
    }
    
    /// Build conservation proof for leverage rebase
    pub fn leverage_rebase_proof(
        long_weight: u32,
        short_weight: u32,
        long_factor: u128,
        short_factor: u128,
        weighted_sum: i64,
    ) -> ConservationProof {
        ConservationProof {
            growth_factors: vec![long_factor, short_factor],
            weights: vec![long_weight, short_weight],
            weighted_log_sum: weighted_sum,
            tolerance_bps: 1, // 0.01% tolerance
        }
    }
    
    /// Build conservation proof for weight rebase
    pub fn weight_rebase_proof(
        weights: Vec<u32>,
        factors: Vec<u128>,
        weighted_sum: i64,
    ) -> ConservationProof {
        ConservationProof {
            growth_factors: factors,
            weights,
            weighted_log_sum: weighted_sum,
            tolerance_bps: 1, // 0.01% tolerance
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_conservation() {
        // Test exact conservation (sum = 0)
        let proof = ConservationProof {
            growth_factors: vec![REBASE_INDEX_SCALE + 1000, REBASE_INDEX_SCALE - 1000],
            weights: vec![5000, 5000],
            weighted_log_sum: 0, // Keeper computed this off-chain
            tolerance_bps: 1,
        };
        
        let result = verify_conservation(&proof).unwrap();
        assert!(result.is_valid);
    }
    
    #[test]
    fn test_invalid_weights() {
        // Weights don't sum to 10000
        let proof = ConservationProof {
            growth_factors: vec![REBASE_INDEX_SCALE, REBASE_INDEX_SCALE],
            weights: vec![4000, 5000], // Sum = 9000
            weighted_log_sum: 0,
            tolerance_bps: 1,
        };
        
        assert!(verify_conservation(&proof).is_err());
    }
}

// ============================================================================
// Buffer Participation in Conservation
// ============================================================================

/// Enhanced conservation verification with buffer participation
pub struct BufferConservationContext<'a> {
    pub buffer: &'a BufferAccount,
    pub field_commitment: &'a FieldCommitment,
    pub operation_type: RebaseOperationType,
}

/// Types of rebase operations for conservation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RebaseOperationType {
    Lending,
    Leverage,
    WeightRebase,
}

/// Enhanced conservation proof with buffer participation details
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BufferConservationProof {
    /// Base conservation proof
    pub base_proof: ConservationProof,
    
    /// Buffer participation rate (basis points)
    pub buffer_participation_bps: u32,
    
    /// Buffer growth factor
    pub buffer_growth_factor: u128,
    
    /// Fee shares allocated to buffer
    pub buffer_fee_share: u64,
    
    /// Rebates paid from buffer
    pub buffer_rebate_share: u64,
}

/// Verify conservation with buffer participation
pub fn verify_conservation_with_buffer(
    proof: &BufferConservationProof,
    ctx: &BufferConservationContext,
) -> Result<()> {
    // First verify base conservation
    let base_result = verify_conservation(&proof.base_proof)?;
    require!(
        base_result.is_valid,
        FeelsProtocolError::ConstraintViolation
    );
    
    // Verify buffer participation constraints
    verify_buffer_participation(proof, ctx)?;
    
    // Verify buffer growth is consistent
    verify_buffer_growth(proof, ctx)?;
    
    msg!("Conservation with buffer verified successfully");
    msg!("  Buffer participation: {} bps", proof.buffer_participation_bps);
    msg!("  Buffer growth factor: {}", proof.buffer_growth_factor);
    
    Ok(())
}

/// Verify buffer participation constraints
fn verify_buffer_participation(
    proof: &BufferConservationProof,
    ctx: &BufferConservationContext,
) -> Result<()> {
    // Get buffer weight from field commitment
    let buffer_weight = ctx.field_commitment.w_tau;
    
    // Verify participation rate is consistent with weight
    let expected_participation = ((buffer_weight as u64) * (BPS_DENOMINATOR as u64)) / 10000;
    let actual_participation = proof.buffer_participation_bps as u64;
    
    // Allow small deviation (1%)
    let deviation = if expected_participation > actual_participation {
        expected_participation - actual_participation
    } else {
        actual_participation - expected_participation
    };
    
    require!(
        deviation <= 100, // 1% tolerance
        FeelsProtocolError::InvalidParameter
    );
    
    // Verify buffer has sufficient tau for rebates
    let available_tau = ctx.buffer.get_available_tau()?;
    require!(
        available_tau >= proof.buffer_rebate_share,
        FeelsProtocolError::InsufficientBuffer
    );
    
    Ok(())
}

/// Verify buffer growth factor is consistent with conservation
fn verify_buffer_growth(
    proof: &BufferConservationProof,
    ctx: &BufferConservationContext,
) -> Result<()> {
    // Buffer growth should balance fees collected vs rebates paid
    let net_buffer_change = proof.buffer_fee_share
        .saturating_sub(proof.buffer_rebate_share);
    
    // Calculate expected growth factor
    let current_buffer_value = ctx.buffer.total_fees_collected
        .saturating_add(ctx.buffer.tau_value);
    
    if current_buffer_value == 0 {
        // No growth if buffer is empty
        require!(
            proof.buffer_growth_factor == Q64,
            FeelsProtocolError::InvalidParameter
        );
    } else {
        // Growth factor should reflect net change
        let expected_growth = if net_buffer_change > 0 {
            // Buffer growing
            Q64 + ((net_buffer_change as u128 * Q64) / current_buffer_value)
        } else {
            // Buffer shrinking
            Q64 - ((proof.buffer_rebate_share.saturating_sub(proof.buffer_fee_share) as u128 * Q64) 
                / current_buffer_value)
        };
        
        // Allow small deviation
        let growth_deviation = if expected_growth > proof.buffer_growth_factor {
            expected_growth - proof.buffer_growth_factor
        } else {
            proof.buffer_growth_factor - expected_growth
        };
        
        require!(
            growth_deviation <= Q64 / 1000, // 0.1% tolerance
            FeelsProtocolError::InvalidParameter
        );
    }
    
    Ok(())
}

/// Calculate buffer's share of fees based on domain activity
pub fn calculate_buffer_fee_share(
    total_fees: u64,
    domain_activity: &DomainActivity,
    field_commitment: &FieldCommitment,
) -> Result<u64> {
    // Buffer gets weighted share based on activity
    let (w_s, w_t, w_l) = (
        field_commitment.w_s,
        field_commitment.w_t,
        field_commitment.w_l,
    );
    
    // Calculate activity-weighted buffer share
    let spot_share = (total_fees as u128 * w_s as u128 * domain_activity.spot_volume) 
        / (BPS_DENOMINATOR as u128 * domain_activity.total_volume);
    
    let time_share = (total_fees as u128 * w_t as u128 * domain_activity.time_volume)
        / (BPS_DENOMINATOR as u128 * domain_activity.total_volume);
    
    let leverage_share = (total_fees as u128 * w_l as u128 * domain_activity.leverage_volume)
        / (BPS_DENOMINATOR as u128 * domain_activity.total_volume);
    
    let buffer_share = spot_share
        .saturating_add(time_share)
        .saturating_add(leverage_share)
        .min(u64::MAX as u128) as u64;
    
    Ok(buffer_share)
}

/// Domain activity tracking for fee distribution
#[derive(Debug, Default)]
pub struct DomainActivity {
    pub spot_volume: u128,
    pub time_volume: u128,
    pub leverage_volume: u128,
    pub total_volume: u128,
}

/// Build conservation proof with buffer participation
pub fn build_buffer_conservation_proof(
    base_weights: Vec<u32>,
    base_factors: Vec<u128>,
    buffer_participation_bps: u32,
    buffer_growth_factor: u128,
    buffer_fee_share: u64,
    buffer_rebate_share: u64,
    weighted_sum: i64,
) -> BufferConservationProof {
    // Add buffer to weights and factors
    let mut all_weights = base_weights.clone();
    let mut all_factors = base_factors.clone();
    
    // Buffer weight is derived from participation
    let buffer_weight = (buffer_participation_bps * 10000) / BPS_DENOMINATOR;
    all_weights.push(buffer_weight);
    all_factors.push(buffer_growth_factor);
    
    // Adjust other weights proportionally
    let total_weight: u32 = all_weights.iter().sum();
    if total_weight != 10000 {
        // Normalize weights
        for weight in all_weights.iter_mut() {
            *weight = (*weight * 10000) / total_weight;
        }
    }
    
    BufferConservationProof {
        base_proof: ConservationProof {
            growth_factors: all_factors,
            weights: all_weights,
            weighted_log_sum: weighted_sum,
            tolerance_bps: 1,
        },
        buffer_participation_bps,
        buffer_growth_factor,
        buffer_fee_share,
        buffer_rebate_share,
    }
}

#[cfg(test)]
mod buffer_tests {
    use super::*;
    
    #[test]
    fn test_buffer_conservation_proof() {
        let proof = build_buffer_conservation_proof(
            vec![4000, 4000], // Base weights
            vec![Q64 + 1000, Q64 - 1000], // Base factors
            2000, // 20% buffer participation
            Q64, // No buffer growth
            1000, // Fees collected
            1000, // Rebates paid
            0, // Perfect conservation
        );
        
        // Should have 3 weights after adding buffer
        assert_eq!(proof.base_proof.weights.len(), 3);
        assert_eq!(proof.base_proof.growth_factors.len(), 3);
        
        // Weights should sum to 10000
        let sum: u32 = proof.base_proof.weights.iter().sum();
        assert_eq!(sum, 10000);
    }
}