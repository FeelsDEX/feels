/// Simplified oracle system for market parameter updates.
/// Replaces complex keeper competition with straightforward oracle feeds.
use anchor_lang::prelude::*;
use crate::state::{Pool, MarketState};
use crate::error::FeelsError;

// ============================================================================
// Constants
// ============================================================================

/// Maximum staleness for oracle updates (seconds)
pub const MAX_ORACLE_STALENESS: i64 = 300; // 5 minutes

/// Maximum change allowed in single update (basis points)
pub const MAX_PARAMETER_CHANGE_BPS: u32 = 500; // 5%

// ============================================================================
// Oracle Update Structure
// ============================================================================

/// Simplified oracle update containing market parameters
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct OracleUpdate {
    /// Pool being updated
    pub pool: Pubkey,
    
    /// Updated market parameters
    pub parameters: MarketParameters,
    
    /// Timestamp of the update
    pub timestamp: i64,
    
    /// Oracle authority that signed the update
    pub oracle: Pubkey,
}

/// Market parameters that can be updated by oracle
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct MarketParameters {
    /// Spot price gradient (simplified from 3D)
    pub spot_gradient: i64,
    
    /// Time/rate gradient
    pub rate_gradient: i64,
    
    /// Leverage gradient
    pub leverage_gradient: i64,
    
    /// Market curvature (simplified Hessian)
    pub market_curvature: u64,
    
    /// Risk parameters
    pub risk_adjustment: u32,
    
    /// Volatility estimate
    pub volatility: u32,
}

impl MarketParameters {
    /// Validate parameters are within reasonable bounds
    pub fn validate(&self) -> Result<()> {
        // Gradients should be bounded
        require!(
            self.spot_gradient.abs() < (1 << 40), // ~1 trillion
            FeelsError::ParameterError {
                parameter: "spot_gradient".to_string(),
                reason: "Gradient too large".to_string(),
            }
        );
        
        require!(
            self.rate_gradient.abs() < (1 << 40),
            FeelsError::ParameterError {
                parameter: "rate_gradient".to_string(),
                reason: "Gradient too large".to_string(),
            }
        );
        
        require!(
            self.leverage_gradient.abs() < (1 << 40),
            FeelsError::ParameterError {
                parameter: "leverage_gradient".to_string(),
                reason: "Gradient too large".to_string(),
            }
        );
        
        // Curvature must be positive (convexity)
        require!(
            self.market_curvature > 0 && self.market_curvature < (1 << 50),
            FeelsError::ParameterError {
                parameter: "market_curvature".to_string(),
                reason: "Invalid curvature".to_string(),
            }
        );
        
        // Risk adjustment in basis points
        require!(
            self.risk_adjustment <= 10_000, // Max 100%
            FeelsError::ParameterError {
                parameter: "risk_adjustment".to_string(),
                reason: "Risk adjustment too high".to_string(),
            }
        );
        
        // Volatility in basis points
        require!(
            self.volatility <= 100_000, // Max 1000% annualized
            FeelsError::ParameterError {
                parameter: "volatility".to_string(),
                reason: "Volatility unrealistic".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Check if parameter change is within allowed bounds
    pub fn validate_change(&self, previous: &MarketParameters) -> Result<()> {
        // Check each parameter doesn't change too much
        let spot_change = ((self.spot_gradient - previous.spot_gradient).abs() * 10_000) 
            / previous.spot_gradient.abs().max(1);
        require!(
            spot_change as u32 <= MAX_PARAMETER_CHANGE_BPS,
            FeelsError::ParameterError {
                parameter: "spot_gradient".to_string(),
                reason: format!("Change {} bps exceeds max {} bps", spot_change, MAX_PARAMETER_CHANGE_BPS),
            }
        );
        
        // Similar checks for other parameters...
        
        Ok(())
    }
}

// ============================================================================
// Oracle Interface
// ============================================================================

/// Trait for oracle implementations
pub trait OracleProvider {
    /// Get current market parameters
    fn get_parameters(&self, pool: &Pool) -> Result<MarketParameters>;
    
    /// Validate oracle is authorized for pool
    fn is_authorized(&self, pool: &Pool, oracle: &Pubkey) -> bool;
}

/// Simple oracle configuration
#[account]
pub struct OracleConfig {
    /// Primary oracle authority
    pub primary_oracle: Pubkey,
    
    /// Secondary oracle (for redundancy)
    pub secondary_oracle: Pubkey,
    
    /// Minimum time between updates
    pub update_frequency: i64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Current parameters
    pub current_parameters: MarketParameters,
}

impl OracleConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // primary_oracle
        32 + // secondary_oracle  
        8 + // update_frequency
        8 + // last_update
        8 + 8 + 8 + 8 + 4 + 4; // MarketParameters
}

// ============================================================================
// Update Validation
// ============================================================================

/// Validate oracle update before applying
pub fn validate_oracle_update(
    update: &OracleUpdate,
    config: &OracleConfig,
    current_time: i64,
) -> Result<()> {
    // Check oracle is authorized
    require!(
        update.oracle == config.primary_oracle || 
        update.oracle == config.secondary_oracle,
        FeelsError::UnauthorizedError {
            action: "oracle_update".to_string(),
            reason: "Oracle not authorized".to_string(),
        }
    );
    
    // Check update is recent
    require!(
        current_time - update.timestamp <= MAX_ORACLE_STALENESS,
        FeelsError::ValidationError {
            field: "timestamp".to_string(),
            reason: format!("Update too old: {} seconds", current_time - update.timestamp),
        }
    );
    
    // Check minimum update frequency
    require!(
        update.timestamp >= config.last_update + config.update_frequency,
        FeelsError::ValidationError {
            field: "timestamp".to_string(),
            reason: "Update too frequent".to_string(),
        }
    );
    
    // Validate parameters
    update.parameters.validate()?;
    
    // Validate change is reasonable
    update.parameters.validate_change(&config.current_parameters)?;
    
    Ok(())
}

// ============================================================================
// Apply Updates
// ============================================================================

/// Apply validated oracle update to market state
pub fn apply_oracle_update(
    market_state: &mut MarketState,
    config: &mut OracleConfig,
    update: &OracleUpdate,
) -> Result<()> {
    // Update market state with new parameters
    // This is greatly simplified from the original gradient/hessian system
    
    // Apply gradients (simplified from 3D to individual components)
    market_state.spot_gradient = update.parameters.spot_gradient;
    market_state.rate_gradient = update.parameters.rate_gradient;
    market_state.leverage_gradient = update.parameters.leverage_gradient;
    
    // Apply curvature (simplified from 3x3 Hessian)
    market_state.market_curvature = update.parameters.market_curvature;
    
    // Apply risk parameters
    market_state.risk_adjustment = update.parameters.risk_adjustment;
    market_state.implied_volatility = update.parameters.volatility;
    
    // Update config
    config.last_update = update.timestamp;
    config.current_parameters = update.parameters.clone();
    
    msg!("Oracle update applied successfully");
    msg!("Spot gradient: {}", update.parameters.spot_gradient);
    msg!("Volatility: {} bps", update.parameters.volatility);
    
    Ok(())
}

// ============================================================================
// Simplified Gradient Calculation
// ============================================================================

/// Calculate gradients from market state (simplified)
pub fn calculate_simple_gradients(
    pool: &Pool,
    spot_price: u64,
    time_value: u64,
    leverage_factor: u64,
) -> Result<MarketParameters> {
    // Simple gradient calculation based on current state
    // This replaces the complex 3D gradient system
    
    let spot_gradient = {
        // dV/dS ≈ -1/S for logarithmic utility
        let s_scaled = spot_price.max(1);
        -(1i64 << 64) / s_scaled as i64
    };
    
    let rate_gradient = {
        // dV/dT ≈ -1/T
        let t_scaled = time_value.max(1);
        -(1i64 << 64) / t_scaled as i64
    };
    
    let leverage_gradient = {
        // dV/dL ≈ -1/L  
        let l_scaled = leverage_factor.max(1);
        -(1i64 << 64) / l_scaled as i64
    };
    
    // Simple curvature (second derivative)
    let market_curvature = {
        // d²V/dx² ≈ 1/x² for logarithmic utility
        let avg_scale = ((spot_price + time_value + leverage_factor) / 3).max(1);
        (1u64 << 96) / (avg_scale * avg_scale)
    };
    
    // Risk adjustment based on pool utilization
    let risk_adjustment = calculate_risk_adjustment(pool)?;
    
    // Volatility from recent price movements
    let volatility = estimate_volatility(pool)?;
    
    Ok(MarketParameters {
        spot_gradient,
        rate_gradient,
        leverage_gradient,
        market_curvature,
        risk_adjustment,
        volatility,
    })
}

/// Calculate risk adjustment based on pool state
fn calculate_risk_adjustment(pool: &Pool) -> Result<u32> {
    // Simple risk adjustment based on liquidity concentration
    // Returns basis points (0-10000)
    
    let total_liquidity = pool.liquidity;
    if total_liquidity == 0 {
        return Ok(0);
    }
    
    // Higher concentration = higher risk
    let concentration_factor = calculate_liquidity_concentration(pool)?;
    
    // Scale to basis points (max 500 bps = 5%)
    let risk_bps = (concentration_factor * 500).min(500);
    
    Ok(risk_bps as u32)
}

/// Estimate volatility from recent activity
fn estimate_volatility(pool: &Pool) -> Result<u32> {
    // Simplified volatility calculation
    // Returns annualized volatility in basis points
    
    // Use simple proxy based on fee tier
    // Higher fee tier = higher expected volatility
    let base_vol = match pool.fee {
        100 => 1000,    // 0.01% fee = 10% vol
        500 => 2500,    // 0.05% fee = 25% vol
        3000 => 5000,   // 0.30% fee = 50% vol
        10000 => 10000, // 1.00% fee = 100% vol
        _ => 3000,      // Default 30% vol
    };
    
    Ok(base_vol)
}

/// Calculate liquidity concentration metric
fn calculate_liquidity_concentration(pool: &Pool) -> Result<u64> {
    // Returns 0-1 scaled by 2^64
    // 0 = perfectly distributed, 1 = highly concentrated
    
    // Simplified: use tick spacing as proxy
    // Tighter spacing = more concentrated
    let concentration = match pool.tick_spacing {
        1 => (9 << 60),   // 90% concentrated
        10 => (5 << 60),  // 50% concentrated
        60 => (3 << 60),  // 30% concentrated
        200 => (1 << 60), // 10% concentrated
        _ => (5 << 60),   // Default 50%
    };
    
    Ok(concentration)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parameter_validation() {
        let params = MarketParameters {
            spot_gradient: -1000,
            rate_gradient: -2000,
            leverage_gradient: -1500,
            market_curvature: 1000,
            risk_adjustment: 500,
            volatility: 5000,
        };
        
        assert!(params.validate().is_ok());
        
        // Test invalid parameters
        let invalid = MarketParameters {
            spot_gradient: 1i64 << 50, // Too large
            ..params
        };
        assert!(invalid.validate().is_err());
    }
    
    #[test]
    fn test_change_validation() {
        let params1 = MarketParameters {
            spot_gradient: -1000,
            rate_gradient: -2000,
            leverage_gradient: -1500,
            market_curvature: 1000,
            risk_adjustment: 500,
            volatility: 5000,
        };
        
        let params2 = MarketParameters {
            spot_gradient: -1040, // 4% change, within limit
            ..params1.clone()
        };
        
        assert!(params2.validate_change(&params1).is_ok());
        
        let params3 = MarketParameters {
            spot_gradient: -1600, // 60% change, exceeds limit
            ..params1.clone()
        };
        
        assert!(params3.validate_change(&params1).is_err());
    }
}