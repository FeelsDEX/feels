/// Fallback mode manager for handling stale field commitments and degraded conditions.
/// Ensures the protocol can continue operating safely when keeper updates are delayed.

use anchor_lang::prelude::*;
use crate::state::{
    FeelsProtocolError, FeesPolicy, FieldCommitment, MarketField,
    BufferAccount, TwapOracle, VolatilityObservation,
};
use crate::logic::instantaneous_fee::calculate_fallback_fees;
use crate::constant::{Q64, BASIS_POINTS_DENOMINATOR as BPS_DENOMINATOR};

// ============================================================================
// Fallback State
// ============================================================================

/// Current operational mode of the protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalMode {
    /// Normal operation with fresh field commitments
    Normal,
    /// Degraded operation with stale field commitments
    Fallback,
    /// Emergency mode with severe staleness
    Emergency,
}

/// Fallback mode context containing all necessary state
pub struct FallbackContext<'a> {
    pub field_commitment: &'a FieldCommitment,
    pub market_field: &'a MarketField,
    pub fees_policy: &'a FeesPolicy,
    pub buffer: &'a BufferAccount,
    pub twap_oracle: &'a TwapOracle,
    pub volatility_oracle: Option<&'a crate::state::VolatilityOracle>,
    pub current_time: i64,
}

/// Result of fallback mode evaluation
#[derive(Debug)]
pub struct FallbackEvaluation {
    pub mode: OperationalMode,
    pub staleness_seconds: i64,
    pub base_fee_bps: u64,
    pub volatility_multiplier: u64,
    pub confidence_score: u64, // 0-10000 bps
    pub reasons: Vec<&'static str>,
}

// ============================================================================
// Fallback Mode Manager
// ============================================================================

pub struct FallbackModeManager;

impl FallbackModeManager {
    /// Evaluate current operational mode based on staleness and market conditions
    pub fn evaluate_mode(ctx: &FallbackContext) -> Result<FallbackEvaluation> {
        let staleness = ctx.current_time - ctx.field_commitment.snapshot_ts;
        let max_staleness = ctx.fees_policy.max_commitment_staleness;
        
        let mut reasons = Vec::new();
        let mut confidence_score = 10000u64; // Start at 100%
        
        // Determine operational mode
        let mode = if staleness <= max_staleness {
            OperationalMode::Normal
        } else if staleness <= max_staleness * 3 {
            // Up to 3x staleness threshold
            reasons.push("Field commitment moderately stale");
            confidence_score = 7500; // 75% confidence
            OperationalMode::Fallback
        } else {
            // Severe staleness
            reasons.push("Field commitment severely stale");
            confidence_score = 2500; // 25% confidence
            OperationalMode::Emergency
        };
        
        // Additional checks for degradation
        if !Self::validate_twap_freshness(ctx)? {
            reasons.push("TWAP data stale");
            confidence_score = confidence_score.saturating_sub(2000);
        }
        
        if !Self::validate_buffer_health(ctx)? {
            reasons.push("Buffer unhealthy");
            confidence_score = confidence_score.saturating_sub(1000);
        }
        
        // Calculate fee parameters based on mode
        let (base_fee_bps, volatility_multiplier) = match mode {
            OperationalMode::Normal => {
                (ctx.field_commitment.base_fee_bps, 10000) // 1x
            }
            OperationalMode::Fallback => {
                let fee = ctx.fees_policy.fallback_fee_bps;
                let vol_mult = Self::calculate_fallback_volatility(ctx)?;
                (fee, vol_mult)
            }
            OperationalMode::Emergency => {
                // Use maximum fees in emergency
                let fee = ctx.fees_policy.max_base_fee_bps;
                (fee, 30000) // 3x multiplier
            }
        };
        
        Ok(FallbackEvaluation {
            mode,
            staleness_seconds: staleness,
            base_fee_bps,
            volatility_multiplier,
            confidence_score,
            reasons,
        })
    }
    
    /// Calculate dynamic fee for fallback mode
    pub fn calculate_dynamic_fee(
        amount_in: u64,
        evaluation: &FallbackEvaluation,
        ctx: &FallbackContext,
    ) -> Result<u64> {
        match evaluation.mode {
            OperationalMode::Normal => {
                // Use normal instantaneous fee calculation
                Ok(0) // Caller should use normal path
            }
            OperationalMode::Fallback => {
                // Use enhanced fallback calculation
                let volatility_bps = Self::estimate_current_volatility(ctx)?;
                calculate_fallback_fees(
                    amount_in,
                    ctx.market_field,
                    ctx.fees_policy,
                    Some(volatility_bps),
                )
            }
            OperationalMode::Emergency => {
                // Simple high fee in emergency
                let fee = (amount_in as u128)
                    .saturating_mul(evaluation.base_fee_bps as u128)
                    .saturating_div(BPS_DENOMINATOR as u128)
                    .min(u64::MAX as u128) as u64;
                Ok(fee)
            }
        }
    }
    
    /// Validate TWAP oracle freshness
    fn validate_twap_freshness(ctx: &FallbackContext) -> Result<bool> {
        let twap_age = ctx.current_time - ctx.twap_oracle.last_update;
        let max_twap_age = ctx.fees_policy.max_commitment_staleness * 2; // 2x field staleness
        Ok(twap_age <= max_twap_age)
    }
    
    /// Validate buffer health
    fn validate_buffer_health(ctx: &FallbackContext) -> Result<bool> {
        // Check if buffer has sufficient tau
        let tau_balance = ctx.buffer.get_available_tau()?;
        let min_tau = ctx.buffer.rebate_cap_epoch / 10; // At least 10% of epoch cap
        
        // Check if fees are being collected
        let fee_growth = ctx.buffer.total_fees_collected > 0;
        
        Ok(tau_balance >= min_tau && fee_growth)
    }
    
    /// Calculate volatility multiplier for fallback mode
    fn calculate_fallback_volatility(ctx: &FallbackContext) -> Result<u64> {
        // Try to get recent volatility from oracle
        if let Some(vol_obs) = Self::get_recent_volatility(ctx)? {
            // Get volatility from log return squared
            // Convert log return squared to basis points approximation
            let volatility_bps = (vol_obs.log_return_squared as u64).saturating_mul(100);
            // Scale volatility to multiplier (1000 bps = 1x, 5000 bps = 2x)
            let multiplier = 10000 + volatility_bps.saturating_sub(1000);
            Ok(multiplier.min(30000)) // Cap at 3x
        } else {
            // Use field commitment sigma as fallback
            let sigma_bps = ((ctx.market_field.sigma_price as u128 * BPS_DENOMINATOR as u128) / Q64) as u64;
            let multiplier = 10000u64 + sigma_bps.saturating_sub(1000);
            Ok(multiplier.min(25000)) // Cap at 2.5x without fresh data
        }
    }
    
    /// Estimate current volatility from available data
    fn estimate_current_volatility(ctx: &FallbackContext) -> Result<u64> {
        // Check for recent volatility observation
        if let Some(vol_obs) = Self::get_recent_volatility(ctx)? {
            // Convert log return squared to basis points approximation
            let volatility_bps = (vol_obs.log_return_squared as u64).saturating_mul(100);
            return Ok(volatility_bps.min(10000)); // Cap at 100%
        }
        
        // Estimate from TWAP price movements
        let twap_spread = ctx.twap_oracle.twap_1_per_0
            .abs_diff(ctx.field_commitment.twap_1)
            .saturating_mul(BPS_DENOMINATOR as u128)
            .saturating_div(ctx.twap_oracle.twap_1_per_0.max(1));
        
        // Convert spread to volatility estimate (rough approximation)
        let volatility_estimate = (twap_spread as u64).saturating_mul(2);
        
        Ok(volatility_estimate.min(10000)) // Cap at 100%
    }
    
    /// Get recent volatility observation if available
    fn get_recent_volatility(ctx: &FallbackContext) -> Result<Option<VolatilityObservation>> {
        // Check if volatility oracle is available and fresh
        if let Some(vol_oracle) = ctx.volatility_oracle {
            // Check if oracle data is fresh (max 1 hour old)
            if vol_oracle.is_fresh(ctx.current_time, 3600) {
                // Convert oracle data to VolatilityObservation
                return Ok(Some(vol_oracle.to_observation()));
            } else {
                msg!("Volatility oracle data is stale, using fallback calculation");
            }
        } else {
            msg!("No volatility oracle available, using fallback calculation");
        }
        
        // Return None to use fallback calculation
        Ok(None)
    }
}

// ============================================================================
// Integration Helpers
// ============================================================================

/// Check if protocol should enter fallback mode
pub fn should_use_fallback_mode(
    field_commitment: &FieldCommitment,
    fees_policy: &FeesPolicy,
    current_time: i64,
) -> bool {
    let staleness = current_time - field_commitment.snapshot_ts;
    staleness > fees_policy.max_commitment_staleness
}

/// Get appropriate fee parameters for current conditions
pub fn get_fee_parameters(
    evaluation: &FallbackEvaluation,
    amount_in: u64,
) -> Result<(u64, u64)> { // (base_fee_bps, fee_amount)
    let base_fee = evaluation.base_fee_bps;
    
    // Apply confidence-based adjustment
    let confidence_mult = evaluation.confidence_score.max(2500); // Min 25%
    let adjusted_fee_bps = (base_fee as u128)
        .saturating_mul(10000)
        .saturating_div(confidence_mult as u128)
        .min(10000) as u64; // Cap at 100%
    
    // Calculate fee amount
    let fee_amount = (amount_in as u128)
        .saturating_mul(adjusted_fee_bps as u128)
        .saturating_mul(evaluation.volatility_multiplier as u128)
        .saturating_div(BPS_DENOMINATOR as u128)
        .saturating_div(10000) // Volatility scale
        .min(u64::MAX as u128) as u64;
    
    Ok((adjusted_fee_bps, fee_amount))
}

/// Log fallback mode status for monitoring
pub fn log_fallback_status(evaluation: &FallbackEvaluation) {
    msg!(
        "Fallback mode: {:?}, staleness={}s, confidence={}%, fee={} bps",
        evaluation.mode,
        evaluation.staleness_seconds,
        evaluation.confidence_score / 100,
        evaluation.base_fee_bps
    );
    
    for reason in &evaluation.reasons {
        msg!("  - {}", reason);
    }
}

// ============================================================================
// Emergency Mode Actions
// ============================================================================

/// Actions to take in emergency mode
pub struct EmergencyActions;

impl EmergencyActions {
    /// Pause non-essential operations
    pub fn pause_operations(emergency_flags: &mut crate::state::EmergencyFlags, timestamp: i64) -> Result<()> {
        msg!("EMERGENCY: Pausing non-essential operations");
        
        // Set flags to disable features
        emergency_flags.activate_emergency("Market conditions require emergency pause", timestamp);
        
        Ok(())
    }
    
    /// Increase all fees to maximum
    pub fn maximize_fees(fees_policy: &FeesPolicy) -> u64 {
        msg!("EMERGENCY: Setting maximum fees");
        fees_policy.max_base_fee_bps
    }
    
    /// Disable rebates
    pub fn disable_rebates() -> Result<()> {
        msg!("EMERGENCY: Disabling rebates");
        // Rebate disabling is handled by EmergencyFlags.pause_rebates
        Ok(())
    }
    
    /// Alert keepers
    pub fn alert_keepers(market: Pubkey, staleness: i64) -> Result<()> {
        msg!(
            "EMERGENCY: Market {} has stale data for {} seconds",
            market,
            staleness
        );
        // Emergency events are handled by EmergencyFlags activation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operational_mode_detection() {
        // Test normal mode
        let eval = FallbackEvaluation {
            mode: OperationalMode::Normal,
            staleness_seconds: 300,
            base_fee_bps: 25,
            volatility_multiplier: 10000,
            confidence_score: 10000,
            reasons: vec![],
        };
        assert_eq!(eval.mode, OperationalMode::Normal);
        assert_eq!(eval.confidence_score, 10000);
        
        // Test fallback mode
        let eval_fallback = FallbackEvaluation {
            mode: OperationalMode::Fallback,
            staleness_seconds: 3600,
            base_fee_bps: 100,
            volatility_multiplier: 15000,
            confidence_score: 7500,
            reasons: vec!["Field commitment moderately stale"],
        };
        assert_eq!(eval_fallback.mode, OperationalMode::Fallback);
        assert!(eval_fallback.confidence_score < 10000);
    }

    #[test]
    fn test_fee_calculation() {
        let eval = FallbackEvaluation {
            mode: OperationalMode::Fallback,
            staleness_seconds: 3600,
            base_fee_bps: 100,
            volatility_multiplier: 20000, // 2x
            confidence_score: 5000, // 50%
            reasons: vec![],
        };
        
        let (fee_bps, fee_amount) = get_fee_parameters(&eval, 1000000).unwrap();
        
        // Base fee adjusted for confidence: 100 * 10000 / 5000 = 200 bps
        assert_eq!(fee_bps, 200);
        
        // Fee amount: 1000000 * 200 * 20000 / 10000 / 10000 = 4000
        assert_eq!(fee_amount, 4000);
    }
}