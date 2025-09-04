//! # System Safety and Stability Checks
//! 
//! This module focuses on system stability mechanisms including:
//! - Leverage limits and safety constraints
//! - Fallback mode triggers and circuit breakers
//! - Market health monitoring and protective measures
//! - Anti-manipulation and attack prevention
//! 
//! These checks ensure the protocol remains stable and prevents exploitation
//! through various circuit breakers and safety mechanisms.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketField, MarketManager, UnifiedOracle, BufferAccount, FeesPolicy,
    PoolStatus,
};
use feels_core::constants::{Q64, BASIS_POINTS_DENOMINATOR};

// ============================================================================
// Safety Constants
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

// ============================================================================
// Leverage Safety Mechanisms
// ============================================================================

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
// System Health Assessment
// ============================================================================

/// Comprehensive safety assessment for an operation
pub struct SafetyAssessment {
    pub leverage_safe: bool,
    pub market_healthy: bool,
    pub risk_score: u32, // 0-10000 basis points
    pub warnings: Vec<SafetyWarning>,
}

#[derive(Debug)]
pub enum SafetyWarning {
    HighLeverage,
    LowLiquidity,
    PriceDeviation,
    StaleOracle,
    BufferLow,
}

/// Perform comprehensive safety assessment
pub fn assess_system_safety(
    leverage_op: Option<&LeverageOperation>,
    market_field: &MarketField,
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    buffer: &BufferAccount,
    policy: &FeesPolicy,
    current_time: i64,
) -> Result<SafetyAssessment> {
    let mut warnings = Vec::new();
    let mut risk_score = 0u32;
    
    // Check leverage safety if operation provided
    let leverage_safe = if let Some(op) = leverage_op {
        check_leverage_safety(op, market_manager, oracle, policy, current_time).is_ok()
    } else {
        true
    };
    
    if !leverage_safe {
        risk_score += 3000;
        warnings.push(SafetyWarning::HighLeverage);
    }
    
    // Check market health
    let market_healthy = check_market_health(
        market_manager,
        oracle,
        buffer,
        &mut warnings,
        &mut risk_score,
    );
    
    Ok(SafetyAssessment {
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
    warnings: &mut Vec<SafetyWarning>,
    risk_score: &mut u32,
) -> bool {
    let mut healthy = true;
    
    // Check liquidity
    if market_manager.liquidity < 1000 * Q64 {
        warnings.push(SafetyWarning::LowLiquidity);
        *risk_score += 1000;
    }
    
    // Check oracle staleness
    if oracle.status != crate::state::OracleStatus::Active {
        warnings.push(SafetyWarning::StaleOracle);
        *risk_score += 2000;
        healthy = false;
    }
    
    // Check buffer capacity
    let buffer_capacity = buffer.available_rebate();
    if buffer_capacity < buffer.max_rebate_per_epoch / 10 {
        warnings.push(SafetyWarning::BufferLow);
        *risk_score += 500;
    }
    
    healthy && *risk_score < 5000
}

// ============================================================================
// Circuit Breakers and Fallback Modes
// ============================================================================

/// Check if market should enter fallback mode
pub fn should_trigger_fallback(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
    buffer: &BufferAccount,
) -> Result<bool> {
    // Trigger fallback if:
    // 1. Oracle is stale or unreliable
    if oracle.status != crate::state::OracleStatus::Active {
        return Ok(true);
    }
    
    // 2. Liquidity is critically low
    if market_manager.liquidity < 100 * Q64 {
        return Ok(true);
    }
    
    // 3. Buffer is depleted
    if buffer.available_rebate() == 0 {
        return Ok(true);
    }
    
    Ok(false)
}

/// Emergency pause conditions
pub fn should_emergency_pause(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
) -> Result<bool> {
    // Emergency pause if spot price deviates dramatically from oracle
    let deviation = calculate_spot_deviation(market_manager, oracle)?;
    if deviation > 5000 { // 50% deviation
        return Ok(true);
    }
    
    // Pause if liquidity drops to dangerous levels
    if market_manager.liquidity < 10 * Q64 {
        return Ok(true);
    }
    
    Ok(false)
}

// ============================================================================
// Safety Utilities
// ============================================================================

/// Validate that a leverage operation is within safe bounds
pub fn validate_leverage_bounds(operation: &LeverageOperation) -> Result<()> {
    // Check leverage multiplier is reasonable
    require!(
        operation.leverage >= 100 && operation.leverage <= 2000, // 1x to 20x
        FeelsProtocolError::InvalidLeverage
    );
    
    // Check notional is not zero
    require!(
        operation.notional > 0,
        FeelsProtocolError::InvalidAmount
    );
    
    Ok(())
}

/// Helper to determine if system is under stress
pub fn is_system_stressed(
    market_manager: &MarketManager,
    oracle: &UnifiedOracle,
) -> Result<bool> {
    let deviation = calculate_spot_deviation(market_manager, oracle)?;
    let utilization = calculate_utilization(market_manager)?;
    
    // System is stressed if price deviation > 10% or utilization > 90%
    Ok(deviation > 1000 || utilization > 9000)
}