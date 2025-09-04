//! # Field-related Types
//! 
//! Types for market field state and commitments.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::constants::*;

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Trade dimension identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub enum TradeDimension {
    /// Spot trading dimension (S)
    Spot,
    /// Time/lending dimension (T)
    Time,
    /// Leverage dimension (L)
    Leverage,
    /// Mixed/coupled dimension transitions
    Mixed,
}

/// Field commitment data posted by keeper
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
#[allow(non_snake_case)]
pub struct FieldCommitmentData {
    /// Market scalars (computed from 3D potential field)
    pub S: u128,  // Spot dimension scalar
    pub T: u128,  // Time dimension scalar  
    pub L: u128,  // Leverage dimension scalar
    
    /// Domain weights (basis points, sum to 10000)
    pub w_s: u32,    // Spot weight
    pub w_t: u32,    // Time weight
    pub w_l: u32,    // Leverage weight
    pub w_tau: u32,  // Tau (protocol token) weight
    
    /// Spot value weights (basis points, sum to 10000)
    pub omega_0: u32,  // Token 0 weight
    pub omega_1: u32,  // Token 1 weight
    
    /// Risk parameters (basis points)
    pub sigma_price: u64,    // Price volatility
    pub sigma_rate: u64,     // Rate volatility  
    pub sigma_leverage: u64, // Leverage volatility
    
    /// TWAP prices for spot value calculation
    pub twap_0: u128,
    pub twap_1: u128,
    
    /// Metadata
    pub snapshot_ts: i64,     // Timestamp when computed
    pub max_staleness: i64,   // Maximum age before refresh needed
    pub sequence: u64,        // Sequence number for ordering
    
    /// Dynamic base fee from hysteresis controller (basis points)
    pub base_fee_bps: u64,
    
    /// Optional local quadratic coefficients for enhanced precision
    pub local_coefficients: Option<LocalCoefficients>,
    
    /// Optional commitment hash for verification
    pub commitment_hash: Option<[u8; 32]>,
    
    /// Lipschitz constant for global bounds
    pub lipschitz_L: Option<u64>,
    
    /// Optimality gap in basis points
    pub gap_bps: Option<u64>,
}

/// Local quadratic approximation coefficients
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
#[allow(non_snake_case)]
pub struct LocalCoefficients {
    /// Linear coefficients
    pub c0_s: i128,  // ∂V/∂S at current position
    pub c0_t: i128,  // ∂V/∂T at current position
    pub c0_l: i128,  // ∂V/∂L at current position
    
    /// Quadratic coefficients  
    pub c1_s: i128,  // ∂²V/∂S² at current position
    pub c1_t: i128,  // ∂²V/∂T² at current position
    pub c1_l: i128,  // ∂²V/∂L² at current position
    
    /// Cross derivatives (coupling terms)
    pub c_st: Option<i128>,  // ∂²V/∂S∂T
    pub c_sl: Option<i128>,  // ∂²V/∂S∂L
    pub c_tl: Option<i128>,  // ∂²V/∂T∂L
    
    /// Validity window for local approximation
    pub valid_until: i64,
    
    /// Local bounds for validity region
    pub bounds_s: Option<(u128, u128)>,  // (min_S, max_S)
    pub bounds_t: Option<(u128, u128)>,  // (min_T, max_T)
    pub bounds_l: Option<(u128, u128)>,  // (min_L, max_L)
}

/// Quadratic coefficients for local approximations (simplified version)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct QuadraticCoefficients {
    /// Quadratic coefficient (a in ax² + bx + c)
    pub a: i64,
    /// Linear coefficient
    pub b: i64,
    /// Constant term
    pub c: i64,
    /// Validity range
    pub valid_range: (u128, u128),
}

impl FieldCommitmentData {
    /// Create a new field commitment with basic validation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        s: u128, t: u128, l: u128,
        w_s: u32, w_t: u32, w_l: u32, w_tau: u32,
        omega_0: u32, omega_1: u32,
        sigma_price: u64, sigma_rate: u64, sigma_leverage: u64,
        twap_0: u128, twap_1: u128,
        base_fee_bps: u64,
    ) -> CoreResult<Self> {
        // Validate domain weights sum to 10000
        if w_s + w_t + w_l + w_tau != BPS_DENOMINATOR as u32 {
            return Err(FeelsCoreError::InvalidWeightSum);
        }
        
        // Validate spot weights sum to 10000  
        if omega_0 + omega_1 != BPS_DENOMINATOR as u32 {
            return Err(FeelsCoreError::InvalidWeightSum);
        }
        
        // Validate volatility parameters
        if sigma_price > MAX_VOLATILITY_BPS || sigma_rate > MAX_VOLATILITY_BPS || sigma_leverage > MAX_VOLATILITY_BPS {
            return Err(FeelsCoreError::OutOfBounds);
        }
        
        Ok(Self {
            S: s, T: t, L: l,
            w_s, w_t, w_l, w_tau,
            omega_0, omega_1,
            sigma_price, sigma_rate, sigma_leverage,
            twap_0, twap_1,
            snapshot_ts: 0, // Will be set when created
            max_staleness: DEFAULT_COMMITMENT_STALENESS,
            sequence: 0,    // Will be set when submitted
            base_fee_bps,
            local_coefficients: None,
            commitment_hash: None,
            lipschitz_L: None,
            gap_bps: None,
        })
    }
    
    /// Check if the field commitment is stale
    pub fn is_stale(&self, current_time: i64) -> bool {
        current_time - self.snapshot_ts > self.max_staleness
    }
    
    /// Get normalized trading weights (excluding tau)
    pub fn get_trading_weights(&self) -> (f64, f64, f64) {
        let total = (self.w_s + self.w_t + self.w_l) as f64;
        if total == 0.0 {
            return (1.0/3.0, 1.0/3.0, 1.0/3.0);
        }
        (
            self.w_s as f64 / total,
            self.w_t as f64 / total,
            self.w_l as f64 / total,
        )
    }
    
    /// Get spot value weights as fractions
    pub fn get_spot_weights(&self) -> (f64, f64) {
        (
            self.omega_0 as f64 / BPS_DENOMINATOR as f64,
            self.omega_1 as f64 / BPS_DENOMINATOR as f64,
        )
    }
    
    /// Get dimension weight
    pub fn get_dimension_weight(&self, dimension: TradeDimension) -> CoreResult<u64> {
        match dimension {
            TradeDimension::Spot => Ok(self.w_s as u64),
            TradeDimension::Time => Ok(self.w_t as u64),
            TradeDimension::Leverage => Ok(self.w_l as u64),
            TradeDimension::Mixed => {
                // For mixed, use average of involved dimensions
                Ok(((self.w_s + self.w_t + self.w_l) / 3) as u64)
            }
        }
    }
    
    /// Validate that weights sum to 10000 (100%)
    pub fn validate_weights(&self) -> CoreResult<()> {
        let domain_sum = self.w_s + self.w_t + self.w_l + self.w_tau;
        if domain_sum != BPS_DENOMINATOR as u32 {
            return Err(FeelsCoreError::InvalidWeightSum);
        }
        
        let spot_sum = self.omega_0 + self.omega_1;
        if spot_sum != BPS_DENOMINATOR as u32 {
            return Err(FeelsCoreError::InvalidWeightSum);
        }
        
        Ok(())
    }
}

/// Domain weights structure
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct DomainWeights {
    /// Spot dimension weight (basis points)
    pub w_s: u32,
    /// Time dimension weight (basis points)
    pub w_t: u32,
    /// Leverage dimension weight (basis points)
    pub w_l: u32,
    /// Protocol token weight (basis points)
    pub w_tau: u32,
}

impl DomainWeights {
    /// Get normalized weights (excluding tau)
    pub fn get_hat_weights(&self) -> (u64, u64, u64) {
        let total = self.w_s + self.w_t + self.w_l;
        if total == 0 {
            return (3333, 3333, 3334); // Equal weights
        }
        
        let w_hat_s = ((self.w_s as u64) * BPS_DENOMINATOR) / (total as u64);
        let w_hat_t = ((self.w_t as u64) * BPS_DENOMINATOR) / (total as u64);
        let w_hat_l = ((self.w_l as u64) * BPS_DENOMINATOR) / (total as u64);
        
        (w_hat_s, w_hat_t, w_hat_l)
    }
}

/// Market field state
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct MarketField {
    /// Current domain values
    pub s: u128,
    pub t: u128,
    pub l: u128,
    
    /// Current domain weights
    pub weights: DomainWeights,
    
    /// Total value locked
    pub tvl: u128,
    
    /// Last update timestamp
    pub last_update: i64,
}

/// Risk parameters for a market field
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct FieldRiskParams {
    /// Spot risk (volatility-based)
    pub rho_spot: u64,
    
    /// Time risk per duration bucket
    pub rho_time: Vec<u64>,
    
    /// Leverage risk (skew-based)
    pub rho_leverage: u64,
    
    /// Maximum allowed leverage
    pub max_leverage: u64,
    
    /// Minimum liquidity threshold
    pub min_liquidity: u128,
}

// ============================================================================
// Advanced Field Types (for off-chain use)
// ============================================================================

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    /// Types of field updates
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
    pub enum FieldUpdateType {
        /// Keeper-provided commitment with off-chain computation
        KeeperCommitment,
        /// Oracle-derived update from external price feeds
        OracleUpdate,
        /// Pool-derived update from current on-chain state
        PoolDerived,
        /// Administrative configuration update
        AdminConfig,
    }
    
    /// Source of field commitment data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum FieldSource {
        /// Computed by authorized keeper
        Keeper {
            authority: [u8; 32],  // Pubkey
            computation_method: ComputationMethod,
        },
        /// Derived from oracle feeds
        Oracle {
            oracle_program: [u8; 32],  // Pubkey
            feed_accounts: Vec<[u8; 32]>,  // Vec<Pubkey>
        },
        /// Computed from current pool state
        Pool {
            derivation_method: PoolDerivationMethod,
        },
    }
    
    /// Method used for field computation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ComputationMethod {
        /// Standard 3D physics simulation
        Physics3D,
        /// Machine learning model
        ML { model_id: String, version: u32 },
        /// Hybrid physics + ML approach
        Hybrid { physics_weight: f64 },
    }
    
    /// Method for deriving field from pool state
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PoolDerivationMethod {
        /// Use current reserves and prices
        CurrentState,
        /// Use moving averages
        MovingAverage { window_size: u32 },
        /// Use liquidity-weighted calculations
        LiquidityWeighted,
    }
    
    /// Position in 3D market physics space - floating point version
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct PositionF64 {
        pub s: f64,  // Spot coordinate
        pub t: f64,  // Time coordinate
        pub l: f64,  // Leverage coordinate
    }
    
    /// 3D gradient vector (first derivatives) - floating point version
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct GradientF64 {
        pub ds: f64,  // ∂V/∂S
        pub dt: f64,  // ∂V/∂T  
        pub dl: f64,  // ∂V/∂L
    }
    
    /// 3D Hessian matrix (second derivatives)
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct Hessian3D {
        /// Diagonal elements (pure second derivatives)
        pub d2s: f64,   // ∂²V/∂S²
        pub d2t: f64,   // ∂²V/∂T²
        pub d2l: f64,   // ∂²V/∂L²
        
        /// Off-diagonal elements (cross derivatives)
        pub dst: f64,   // ∂²V/∂S∂T
        pub dsl: f64,   // ∂²V/∂S∂L
        pub dtl: f64,   // ∂²V/∂T∂L
    }
    
    impl PositionF64 {
        /// Create new 3D position
        pub fn new(s: f64, t: f64, l: f64) -> Self {
            Self { s, t, l }
        }
        
        /// Calculate distance to another position
        pub fn distance_to(&self, other: &PositionF64) -> f64 {
            let ds = self.s - other.s;
            let dt = self.t - other.t;
            let dl = self.l - other.l;
            (ds * ds + dt * dt + dl * dl).sqrt()
        }
        
        /// Check if position is within bounds
        pub fn is_in_bounds(&self, min: f64, max: f64) -> bool {
            self.s >= min && self.s <= max &&
            self.t >= min && self.t <= max &&
            self.l >= min && self.l <= max
        }
    }
}

// Re-export advanced types for convenience
#[cfg(feature = "advanced")]
pub use advanced::*;