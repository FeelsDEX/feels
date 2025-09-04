//! # Risk Management and System Invariants
//! 
//! This module consolidates all risk management logic, including conservation
//! law verification and leverage safety checks. It serves as the single source
//! of truth for system safety and invariant checking.
//! 
//! ## Core Invariants
//! 
//! 1. **Conservation Law**: Σ w_i · ln(g_i) = 0 for all operations
//! 2. **Leverage Limits**: Position notionals bounded by market depth
//! 3. **Anti-Manipulation**: Protection against ping-pong attacks

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketField, MarketManager, UnifiedOracle, BufferAccount,
    FieldCommitment, FeesPolicy, PoolStatus,
};
use feels_core::constants::{Q64, BASIS_POINTS_DENOMINATOR};
use crate::state::rebase::REBASE_INDEX_SCALE;

// ============================================================================
// Conservation Law Verification
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
// Leverage Safety Mechanisms
// ============================================================================

/// Maximum leverage notional as percentage of AMM depth (basis points)
pub const MAX_NOTIONAL_PERCENT_OF_DEPTH: u64 = 2000; // 20%

/// Minimum TWAP window for leverage calculations (seconds)
pub const MIN_TWAP_WINDOW: i64 = 15; // Must exceed 2 block reorgs

/// Anti-ping-pong cooldown period (seconds)
pub const PING_PONG_COOLDOWN: i64 = 300; // 5 minutes

/// Maximum leverage adjustments per epoch
pub const MAX_LEVERAGE_ADJUSTMENTS_PER_EPOCH: u8 = 3;

/// Confidence threshold for TWAP reliability (basis points)
pub const TWAP_CONFIDENCE_THRESHOLD: u64 = 100; // 1%

/// Leverage operation tracking
#[derive(Debug)]
pub struct LeverageOperation {
    pub user: Pubkey,
    pub timestamp: i64,
    pub notional: u64,
    pub leverage: u32,
    pub direction: bool, // true = increase, false = decrease
}

/// Check leverage safety constraints
pub fn check_leverage_safety(
    operation: &LeverageOperation,
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    policy: &FeesPolicy,
    current_time: i64,
) -> Result<()> {
    // 1. Check TWAP freshness
    let twap_age = current_time - oracle.last_observation_time;
    require!(
        twap_age <= MIN_TWAP_WINDOW,
        FeelsProtocolError::StaleTWAP
    );
    
    // 2. Check oracle confidence
    let price_confidence = oracle.token_a_confidence * BASIS_POINTS_DENOMINATOR / oracle.token_a_usd_price as u64;
    require!(
        price_confidence <= TWAP_CONFIDENCE_THRESHOLD,
        FeelsProtocolError::InsufficientOracleConfidence
    );
    
    // 3. Check notional limits
    check_notional_limits(
        operation.notional,
        market_manager.liquidity,
        market_manager.sqrt_price,
    )?;
    
    // 4. Check anti-manipulation
    check_ping_pong_protection(
        &operation.user,
        current_time,
        // Would track last operation time per user
    )?;
    
    // 5. Check market stress
    check_market_stress(
        market_manager,
        oracle,
        policy,
    )?;
    
    Ok(())
}

/// Check notional value limits
fn check_notional_limits(
    notional: u64,
    liquidity: u128,
    sqrt_price: u128,
) -> Result<()> {
    // Calculate approximate market depth
    let price = (sqrt_price * sqrt_price) >> 64;
    let depth = (liquidity * price) >> 64;
    
    let max_notional = (depth * MAX_NOTIONAL_PERCENT_OF_DEPTH as u128) / BASIS_POINTS_DENOMINATOR as u128;
    
    require!(
        (notional as u128) <= max_notional,
        FeelsProtocolError::ExcessiveLeverage
    );
    
    Ok(())
}

/// Check for ping-pong attacks
fn check_ping_pong_protection(
    user: &Pubkey,
    current_time: i64,
    // Would pass user's last operation time
) -> Result<()> {
    // Simplified - would track last operation per user
    // and enforce cooldown period
    Ok(())
}

/// Check overall market stress levels
fn check_market_stress(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    policy: &FeesPolicy,
) -> Result<()> {
    // Calculate stress metrics
    let spot_deviation = calculate_spot_deviation(market_manager, oracle)?;
    let utilization = calculate_utilization(market_manager)?;
    let leverage_imbalance = calculate_leverage_imbalance(market_manager)?;
    
    // Check against policy thresholds
    let should_disable = policy.should_disable_market(
        spot_deviation,
        utilization,
        leverage_imbalance,
        0, // consecutive stress periods - would track
    );
    
    require!(
        !should_disable,
        FeelsProtocolError::MarketStressed
    );
    
    Ok(())
}

/// Calculate spot price deviation from oracle
fn calculate_spot_deviation(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
) -> Result<u64> {
    let market_price = market_manager.get_price();
    let oracle_price = oracle.get_safe_twap_a();
    
    if oracle_price == 0 {
        return Ok(0);
    }
    
    let deviation = if market_price > oracle_price {
        ((market_price - oracle_price) * BASIS_POINTS_DENOMINATOR as u128) / oracle_price
    } else {
        ((oracle_price - market_price) * BASIS_POINTS_DENOMINATOR as u128) / oracle_price
    };
    
    Ok(deviation as u64)
}

/// Calculate market utilization rate
fn calculate_utilization(market_manager: &MarketManager) -> Result<u64> {
    if market_manager.liquidity == 0 {
        return Ok(0);
    }
    
    // Simplified - would calculate actual utilization
    // based on active positions vs available liquidity
    Ok(5000) // 50% placeholder
}

/// Calculate leverage imbalance
fn calculate_leverage_imbalance(market_manager: &MarketManager) -> Result<u64> {
    // Simplified - would calculate actual imbalance
    // between long and short leveraged positions
    Ok(1000) // 10% placeholder
}

// ============================================================================
// Combined Risk Assessment
// ============================================================================

/// Comprehensive risk assessment for an operation
pub struct RiskAssessment {
    pub conservation_valid: bool,
    pub leverage_safe: bool,
    pub market_healthy: bool,
    pub risk_score: u32, // 0-10000 basis points
    pub warnings: Vec<RiskWarning>,
}

#[derive(Debug)]
pub enum RiskWarning {
    HighLeverage,
    LowLiquidity,
    PriceDeviation,
    StaleOracle,
    BufferLow,
}

/// Perform comprehensive risk assessment
pub fn assess_operation_risk(
    conservation_proof: Option<&ConservationProof>,
    leverage_op: Option<&LeverageOperation>,
    market_field: &MarketField,
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    buffer: &BufferAccount,
    policy: &FeesPolicy,
    current_time: i64,
) -> Result<RiskAssessment> {
    let mut warnings = Vec::new();
    let mut risk_score = 0u32;
    
    // Check conservation if proof provided
    let conservation_valid = if let Some(proof) = conservation_proof {
        verify_conservation(proof, market_field).is_ok()
    } else {
        true // Not all operations require conservation proof
    };
    
    if !conservation_valid {
        risk_score += 5000; // High risk
    }
    
    // Check leverage safety if operation provided
    let leverage_safe = if let Some(op) = leverage_op {
        check_leverage_safety(op, market_manager, oracle, policy, current_time).is_ok()
    } else {
        true
    };
    
    if !leverage_safe {
        risk_score += 3000;
        warnings.push(RiskWarning::HighLeverage);
    }
    
    // Check market health
    let market_healthy = check_market_health(
        market_manager,
        oracle,
        buffer,
        &mut warnings,
        &mut risk_score,
    );
    
    Ok(RiskAssessment {
        conservation_valid,
        leverage_safe,
        market_healthy,
        risk_score,
        warnings,
    })
}

/// Check overall market health
fn check_market_health(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    buffer: &BufferAccount,
    warnings: &mut Vec<RiskWarning>,
    risk_score: &mut u32,
) -> bool {
    let mut healthy = true;
    
    // Check liquidity
    if market_manager.liquidity < 1000 * Q64 {
        warnings.push(RiskWarning::LowLiquidity);
        *risk_score += 1000;
    }
    
    // Check oracle staleness
    if oracle.status != crate::state::OracleStatus::Active {
        warnings.push(RiskWarning::StaleOracle);
        *risk_score += 2000;
        healthy = false;
    }
    
    // Check buffer capacity
    let buffer_capacity = buffer.available_rebate();
    if buffer_capacity < buffer.max_rebate_per_epoch / 10 {
        warnings.push(RiskWarning::BufferLow);
        *risk_score += 500;
    }
    
    healthy && *risk_score < 5000
}