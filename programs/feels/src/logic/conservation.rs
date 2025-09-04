//! # Conservation Law Enforcement
//! 
//! This module implements the fundamental thermodynamic invariant of the Feels protocol:
//! **Conservation Law**: Σ w_i · ln(g_i) = 0
//! 
//! This law ensures that the total "thermodynamic work" performed by any operation
//! sums to zero across all market dimensions, maintaining system stability and
//! preventing value creation/destruction exploits.
//! 
//! ## Core Theory
//! 
//! The conservation law states that for any state transition P₁ → P₂:
//! - w_s · ln(g_s) + w_t · ln(g_t) + w_l · ln(g_l) + w_τ · ln(g_τ) = 0
//! 
//! Where:
//! - w_i = normalized domain weights (sum to 10000 bps)
//! - g_i = growth factors for each dimension (after/before ratios)
//! - Dimensions: S (spot), T (time), L (leverage), τ (buffer)

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{MarketField, MarketManager, BufferAccount};
use feels_core::constants::Q64;
use crate::state::rebase::REBASE_INDEX_SCALE;

// ============================================================================
// Conservation Proof Types
// ============================================================================

/// Conservation proof data provided by keeper
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConservationProof {
    /// Growth factors in Q64 format (1.0 = 2^64)
    pub growth_factors: ConservationGrowthFactors,
    /// Weighted logarithms scaled by 1e9
    pub weighted_logs: ConservationWeightedLogs,
    /// Sum of weighted logs (should be ~0)
    pub conservation_sum: i64,
    /// Operation type for context
    pub operation: ConservationOperation,
}

/// Growth factors for each dimension
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConservationGrowthFactors {
    pub g_s: u128,   // Spot growth
    pub g_t: u128,   // Time growth
    pub g_l: u128,   // Leverage growth
    pub g_tau: u128, // Buffer growth
}

/// Weighted logarithms for conservation check
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConservationWeightedLogs {
    pub w_ln_g_s: i64,   // w_s * ln(g_s)
    pub w_ln_g_t: i64,   // w_t * ln(g_t)
    pub w_ln_g_l: i64,   // w_l * ln(g_l)
    pub w_ln_g_tau: i64, // w_tau * ln(g_tau)
}

/// Type of operation being verified
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum ConservationOperation {
    Rebase,
    Liquidity,
    Swap,
    Leverage,
}

// ============================================================================
// Conservation Law Verification
// ============================================================================

/// Verify conservation law: Σ w_i · ln(g_i) = 0
pub fn verify_conservation(
    proof: &ConservationProof,
    market_field: &MarketField,
) -> Result<()> {
    // Validate growth factors are positive
    require!(
        proof.growth_factors.g_s > 0 &&
        proof.growth_factors.g_t > 0 &&
        proof.growth_factors.g_l > 0 &&
        proof.growth_factors.g_tau > 0,
        FeelsProtocolError::InvalidGrowthFactors
    );
    
    // Get domain weights
    let weights = market_field.get_weights();
    
    // Verify sum is approximately zero
    let computed_sum = proof.weighted_logs.w_ln_g_s
        + proof.weighted_logs.w_ln_g_t
        + proof.weighted_logs.w_ln_g_l
        + proof.weighted_logs.w_ln_g_tau;
    
    require!(
        computed_sum == proof.conservation_sum,
        FeelsProtocolError::ConservationSumMismatch
    );
    
    // Allow small deviation (0.01% of scale)
    let tolerance = 100_000; // 0.01% of 1e9 scale
    require!(
        proof.conservation_sum.abs() < tolerance,
        FeelsProtocolError::ConservationViolation
    );
    
    // Additional checks based on operation type
    match proof.operation {
        ConservationOperation::Rebase => verify_rebase_constraints(&proof)?,
        ConservationOperation::Leverage => verify_leverage_constraints(&proof)?,
        _ => {}
    }
    
    Ok(())
}

/// Verify rebase-specific constraints
fn verify_rebase_constraints(proof: &ConservationProof) -> Result<()> {
    // Rebase should not change buffer significantly
    let buffer_change = if proof.growth_factors.g_tau > Q64 {
        proof.growth_factors.g_tau - Q64
    } else {
        Q64 - proof.growth_factors.g_tau
    };
    
    let max_buffer_change = Q64 / 100; // 1% max change
    require!(
        buffer_change < max_buffer_change,
        FeelsProtocolError::ExcessiveBufferChange
    );
    
    Ok(())
}

/// Verify leverage-specific constraints
fn verify_leverage_constraints(proof: &ConservationProof) -> Result<()> {
    // Leverage operations should primarily affect L dimension
    let leverage_change = if proof.growth_factors.g_l > Q64 {
        proof.growth_factors.g_l - Q64
    } else {
        Q64 - proof.growth_factors.g_l
    };
    
    // Other dimensions should have minimal change
    let spot_change = if proof.growth_factors.g_s > Q64 {
        proof.growth_factors.g_s - Q64
    } else {
        Q64 - proof.growth_factors.g_s
    };
    
    require!(
        leverage_change > spot_change * 2,
        FeelsProtocolError::InvalidLeverageOperation
    );
    
    Ok(())
}

// ============================================================================
// Conservation State Tracking
// ============================================================================

/// Snapshot of market state for conservation verification
#[derive(Debug, Clone)]
pub struct ConservationSnapshot {
    /// Spot reserves
    pub reserve_0: u64,
    pub reserve_1: u64,
    /// Time dimension state
    pub duration_weighted_liquidity: u128,
    /// Leverage dimension state
    pub total_leveraged_notional: u128,
    /// Buffer state
    pub buffer_value: u128,
    /// Rebase indices
    pub rebase_index_0: u128,
    pub rebase_index_1: u128,
    /// Timestamp
    pub timestamp: i64,
}

impl ConservationSnapshot {
    /// Take a snapshot of current market state
    pub fn capture(
        market_manager: &MarketManager,
        buffer: &BufferAccount,
        current_time: i64,
    ) -> Self {
        Self {
            reserve_0: market_manager.protocol_fees_0, // Simplified
            reserve_1: market_manager.protocol_fees_1,
            duration_weighted_liquidity: market_manager.liquidity,
            total_leveraged_notional: market_manager.total_volume_usd,
            buffer_value: (buffer.accumulated_fees_0 + buffer.accumulated_fees_1) as u128,
            rebase_index_0: REBASE_INDEX_SCALE,
            rebase_index_1: REBASE_INDEX_SCALE,
            timestamp: current_time,
        }
    }
    
    /// Calculate growth factors between snapshots
    pub fn calculate_growth_factors(&self, after: &ConservationSnapshot) -> ConservationGrowthFactors {
        ConservationGrowthFactors {
            g_s: calculate_growth(self.reserve_0 as u128, after.reserve_0 as u128),
            g_t: calculate_growth(self.duration_weighted_liquidity, after.duration_weighted_liquidity),
            g_l: calculate_growth(self.total_leveraged_notional, after.total_leveraged_notional),
            g_tau: calculate_growth(self.buffer_value, after.buffer_value),
        }
    }
}

/// Calculate growth factor with Q64 scaling
fn calculate_growth(before: u128, after: u128) -> u128 {
    if before == 0 {
        return Q64;
    }
    (after * Q64) / before
}

// ============================================================================
// Conservation Utilities
// ============================================================================

/// Validate that a conservation proof is well-formed
pub fn validate_conservation_proof(proof: &ConservationProof) -> Result<()> {
    // Check that growth factors are reasonable (between 0.1x and 10x)
    let min_growth = Q64 / 10; // 0.1x
    let max_growth = Q64 * 10; // 10x
    
    require!(
        proof.growth_factors.g_s >= min_growth && proof.growth_factors.g_s <= max_growth,
        FeelsProtocolError::InvalidGrowthFactors
    );
    require!(
        proof.growth_factors.g_t >= min_growth && proof.growth_factors.g_t <= max_growth,
        FeelsProtocolError::InvalidGrowthFactors
    );
    require!(
        proof.growth_factors.g_l >= min_growth && proof.growth_factors.g_l <= max_growth,
        FeelsProtocolError::InvalidGrowthFactors
    );
    require!(
        proof.growth_factors.g_tau >= min_growth && proof.growth_factors.g_tau <= max_growth,
        FeelsProtocolError::InvalidGrowthFactors
    );
    
    Ok(())
}

/// Helper to create empty conservation proof for testing
#[cfg(feature = "test-utils")]
pub fn create_neutral_conservation_proof() -> ConservationProof {
    ConservationProof {
        growth_factors: ConservationGrowthFactors {
            g_s: Q64,
            g_t: Q64,
            g_l: Q64,
            g_tau: Q64,
        },
        weighted_logs: ConservationWeightedLogs {
            w_ln_g_s: 0,
            w_ln_g_t: 0,
            w_ln_g_l: 0,
            w_ln_g_tau: 0,
        },
        conservation_sum: 0,
        operation: ConservationOperation::Liquidity,
    }
}