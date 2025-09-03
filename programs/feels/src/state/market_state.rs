/// Market state structures for physics calculations and market management.
/// Provides core data structures for 3D market physics model.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Domain Weights Structure
// ============================================================================

/// Domain weights for the 3D market physics model
/// Controls the relative importance of each dimension
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq)]
pub struct DomainWeights {
    /// Spot dimension weight (basis points)
    pub w_s: u32,
    
    /// Time dimension weight (basis points)
    pub w_t: u32,
    
    /// Leverage dimension weight (basis points)
    pub w_l: u32,
    
    /// Buffer dimension weight (basis points)
    pub w_tau: u32,
}

impl DomainWeights {
    /// Validate that weights sum to 10000 basis points
    pub fn validate(&self) -> Result<()> {
        require!(
            self.w_s + self.w_t + self.w_l + self.w_tau == 10000,
            FeelsProtocolError::ValidationError
        );
        Ok(())
    }
    
    /// Get normalized hat weights (excluding tau)
    pub fn get_hat_weights(&self) -> (u64, u64, u64) {
        let trade_total = (self.w_s + self.w_t + self.w_l) as u64;
        if trade_total == 0 {
            return (0, 0, 0);
        }
        
        let scale = 10000u64;
        let w_hat_s = (self.w_s as u64 * scale) / trade_total;
        let w_hat_t = (self.w_t as u64 * scale) / trade_total;
        let w_hat_l = (self.w_l as u64 * scale) / trade_total;
        
        (w_hat_s, w_hat_t, w_hat_l)
    }
}

// ============================================================================
// Market State Structure
// ============================================================================

/// Core market state for physics calculations
/// Contains the minimal state needed for 3D physics computations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct MarketState {
    /// Spot dimension scalar S (Q64 fixed-point)
    pub S: u128,
    
    /// Time dimension scalar T (Q64 fixed-point)
    pub T: u128,
    
    /// Leverage dimension scalar L (Q64 fixed-point)
    pub L: u128,
    
    /// Domain weights controlling physics behavior
    pub weights: DomainWeights,
}

impl MarketState {
    /// Validate market state parameters
    pub fn validate(&self) -> Result<()> {
        // Validate scalars are positive
        require!(
            self.S > 0 && self.T > 0 && self.L > 0,
            FeelsProtocolError::ValidationError
        );
        
        // Validate weights
        self.weights.validate()?;
        
        Ok(())
    }
    
    /// Create default market state
    pub fn default_state() -> Self {
        Self {
            S: 1u128 << 64, // 1.0 in Q64
            T: 1u128 << 64, // 1.0 in Q64
            L: 1u128 << 64, // 1.0 in Q64
            weights: DomainWeights {
                w_s: 4000,   // 40% spot
                w_t: 3000,   // 30% time
                w_l: 2000,   // 20% leverage
                w_tau: 1000, // 10% buffer
            },
        }
    }
}

// ============================================================================
// Pool State (Simplified)
// ============================================================================

/// Simplified pool state for compatibility with existing code
/// This is a transitional structure while migrating to market physics model
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PoolSimplified {
    /// Pool identifier
    pub id: Pubkey,
    
    /// Token 0 mint
    pub token_0: Pubkey,
    
    /// Token 1 mint
    pub token_1: Pubkey,
    
    /// Current tick
    pub current_tick: i32,
    
    /// Current sqrt price (Q64.64)
    pub current_sqrt_price: u128,
    
    /// Total liquidity
    pub liquidity: u128,
    
    /// Fee rate in basis points
    pub fee_rate: u16,
    
    /// Last update timestamp
    pub last_update_ts: i64,
}

impl PoolSimplified {
    /// Convert to market state approximation
    pub fn to_market_state(&self) -> MarketState {
        // Convert pool state to physics state (simplified)
        let spot_scalar = self.current_sqrt_price;
        
        MarketState {
            S: spot_scalar,
            T: 1u128 << 64, // Default time scalar
            L: 1u128 << 64, // Default leverage scalar
            weights: DomainWeights {
                w_s: 7000,   // 70% spot (traditional AMM focus)
                w_t: 1500,   // 15% time
                w_l: 1000,   // 10% leverage
                w_tau: 500,  // 5% buffer
            },
        }
    }
}