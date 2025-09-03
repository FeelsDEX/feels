/// Field commitment and related types for the 3D AMM physics model

use anchor_lang::prelude::*;
use serde::{Deserialize, Serialize};
use crate::constants::*;

// ============================================================================
// Field Commitment Data
// ============================================================================

/// Field commitment data computed off-chain and submitted by keepers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

// ============================================================================
// 3D Position and Derivatives
// ============================================================================

/// Position in 3D market physics space
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position3D {
    pub s: f64,  // Spot coordinate
    pub t: f64,  // Time coordinate
    pub l: f64,  // Leverage coordinate
}

/// 3D gradient vector (first derivatives)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gradient3D {
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

// ============================================================================
// Field Update Types
// ============================================================================

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldSource {
    /// Computed by authorized keeper
    Keeper {
        authority: Pubkey,
        computation_method: ComputationMethod,
    },
    /// Derived from oracle feeds
    Oracle {
        oracle_program: Pubkey,
        feed_accounts: Vec<Pubkey>,
    },
    /// Computed from current pool state
    Pool {
        derivation_method: PoolDerivationMethod,
    },
}

/// Method used for field computation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComputationMethod {
    /// Standard 3D physics simulation
    Physics3D,
    /// Machine learning model
    ML { model_id: String, version: u32 },
    /// Hybrid physics + ML approach
    Hybrid { physics_weight: f64 },
}

/// Method for deriving field from pool state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolDerivationMethod {
    /// Use current reserves and prices
    CurrentState,
    /// Use moving averages
    MovingAverage { window_size: u32 },
    /// Use liquidity-weighted calculations
    LiquidityWeighted,
}

// ============================================================================
// Verification Types
// ============================================================================

/// Proof data for field commitment verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldVerificationProof {
    /// Convex bound verification points
    pub convex_bound_points: Vec<Position3D>,
    
    /// Lipschitz inequality samples
    pub lipschitz_samples: Vec<LipschitzSample>,
    
    /// Merkle proof for coefficient inclusion
    pub merkle_proof: Option<MerkleProof>,
    
    /// Optimality gap verification
    pub gap_certification: Option<GapCertification>,
}

/// Sample point for Lipschitz verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LipschitzSample {
    pub position1: Position3D,
    pub position2: Position3D,
    pub value1: f64,
    pub value2: f64,
    pub distance: f64,
    pub lipschitz_bound: f64,
}

/// Merkle inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MerkleProof {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
    pub root: [u8; 32],
    pub index: u32,
}

/// Optimality gap certification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GapCertification {
    pub gap_bps: u64,
    pub certification_method: String,
    pub sample_points: Vec<Position3D>,
    pub optimal_values: Vec<f64>,
    pub approximate_values: Vec<f64>,
}

// ============================================================================
// Implementation
// ============================================================================

impl FieldCommitmentData {
    /// Create a new field commitment with basic validation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        S: u128, T: u128, L: u128,
        w_s: u32, w_t: u32, w_l: u32, w_tau: u32,
        omega_0: u32, omega_1: u32,
        sigma_price: u64, sigma_rate: u64, sigma_leverage: u64,
        twap_0: u128, twap_1: u128,
        base_fee_bps: u64,
    ) -> std::result::Result<Self, &'static str> {
        // Validate domain weights sum to 10000
        if w_s + w_t + w_l + w_tau != BPS_DENOMINATOR as u32 {
            return Err("Domain weights must sum to 10000 basis points");
        }
        
        // Validate spot weights sum to 10000  
        if omega_0 + omega_1 != BPS_DENOMINATOR as u32 {
            return Err("Spot weights must sum to 10000 basis points");
        }
        
        // Validate volatility parameters
        if sigma_price > MAX_VOLATILITY_BPS || sigma_rate > MAX_VOLATILITY_BPS || sigma_leverage > MAX_VOLATILITY_BPS {
            return Err("Volatility parameters exceed maximum");
        }
        
        Ok(Self {
            S, T, L,
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
    
    /// Calculate commitment hash for verification
    pub fn calculate_hash(&self) -> [u8; 32] {
        use anchor_lang::solana_program::hash::{hash, Hash};
        
        let mut data = Vec::new();
        
        // Add all fields in deterministic order
        data.extend_from_slice(&self.S.to_le_bytes());
        data.extend_from_slice(&self.T.to_le_bytes());
        data.extend_from_slice(&self.L.to_le_bytes());
        data.extend_from_slice(&self.w_s.to_le_bytes());
        data.extend_from_slice(&self.w_t.to_le_bytes());
        data.extend_from_slice(&self.w_l.to_le_bytes());
        data.extend_from_slice(&self.w_tau.to_le_bytes());
        data.extend_from_slice(&self.omega_0.to_le_bytes());
        data.extend_from_slice(&self.omega_1.to_le_bytes());
        data.extend_from_slice(&self.sigma_price.to_le_bytes());
        data.extend_from_slice(&self.sigma_rate.to_le_bytes());
        data.extend_from_slice(&self.sigma_leverage.to_le_bytes());
        data.extend_from_slice(&self.twap_0.to_le_bytes());
        data.extend_from_slice(&self.twap_1.to_le_bytes());
        data.extend_from_slice(&self.sequence.to_le_bytes());
        
        // Add local coefficients if present
        if let Some(coeffs) = &self.local_coefficients {
            data.extend_from_slice(&coeffs.c0_s.to_le_bytes());
            data.extend_from_slice(&coeffs.c0_t.to_le_bytes());
            data.extend_from_slice(&coeffs.c0_l.to_le_bytes());
            data.extend_from_slice(&coeffs.c1_s.to_le_bytes());
            data.extend_from_slice(&coeffs.c1_t.to_le_bytes());
            data.extend_from_slice(&coeffs.c1_l.to_le_bytes());
            data.extend_from_slice(&coeffs.valid_until.to_le_bytes());
        }
        
        hash(&data).to_bytes()
    }
}

impl Position3D {
    /// Create new 3D position
    pub fn new(s: f64, t: f64, l: f64) -> Self {
        Self { s, t, l }
    }
    
    /// Calculate distance to another position
    pub fn distance_to(&self, other: &Position3D) -> f64 {
        let ds = self.s - other.s;
        let dt = self.t - other.t;
        let dl = self.l - other.l;
        (ds*ds + dt*dt + dl*dl).sqrt()
    }
    
    /// Check if position is within bounds
    pub fn is_within_bounds(&self, min: &Position3D, max: &Position3D) -> bool {
        self.s >= min.s && self.s <= max.s &&
        self.t >= min.t && self.t <= max.t &&
        self.l >= min.l && self.l <= max.l
    }
}

impl Gradient3D {
    /// Create new gradient
    pub fn new(ds: f64, dt: f64, dl: f64) -> Self {
        Self { ds, dt, dl }
    }
    
    /// Calculate gradient magnitude
    pub fn magnitude(&self) -> f64 {
        (self.ds*self.ds + self.dt*self.dt + self.dl*self.dl).sqrt()
    }
    
    /// Normalize gradient to unit length
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            self.clone()
        } else {
            Self {
                ds: self.ds / mag,
                dt: self.dt / mag,
                dl: self.dl / mag,
            }
        }
    }
}

impl Hessian3D {
    /// Create new Hessian matrix
    pub fn new(d2s: f64, d2t: f64, d2l: f64, dst: f64, dsl: f64, dtl: f64) -> Self {
        Self { d2s, d2t, d2l, dst, dsl, dtl }
    }
    
    /// Calculate determinant of Hessian
    pub fn determinant(&self) -> f64 {
        self.d2s * (self.d2t * self.d2l - self.dtl * self.dtl) -
        self.dst * (self.dst * self.d2l - self.dtl * self.dsl) +
        self.dsl * (self.dst * self.dtl - self.d2t * self.dsl)
    }
    
    /// Calculate trace of Hessian
    pub fn trace(&self) -> f64 {
        self.d2s + self.d2t + self.d2l
    }
    
    /// Check if Hessian is positive definite (convex)
    pub fn is_positive_definite(&self) -> bool {
        // For 3x3 symmetric matrix, check all leading principal minors
        let m1 = self.d2s;
        let m2 = self.d2s * self.d2t - self.dst * self.dst;
        let m3 = self.determinant();
        
        m1 > 0.0 && m2 > 0.0 && m3 > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_commitment_creation() {
        let field = FieldCommitmentData::new(
            Q64, Q64, Q64,  // S, T, L
            3333, 3333, 3334, 0,  // domain weights
            5000, 5000,  // spot weights
            1000, 500, 1500,  // volatilities
            Q64, Q64,  // TWAPs
            25,  // base_fee_bps
        ).unwrap();
        
        assert_eq!(field.w_s + field.w_t + field.w_l + field.w_tau, 10000);
        assert_eq!(field.omega_0 + field.omega_1, 10000);
    }

    #[test]
    fn test_field_commitment_validation() {
        // Invalid domain weights
        assert!(FieldCommitmentData::new(
            Q64, Q64, Q64, 5000, 5000, 5000, 0, 5000, 5000, 1000, 500, 1500, Q64, Q64, 25
        ).is_err());
        
        // Invalid spot weights
        assert!(FieldCommitmentData::new(
            Q64, Q64, Q64, 3333, 3333, 3334, 0, 6000, 5000, 1000, 500, 1500, Q64, Q64, 25
        ).is_err());
    }

    #[test]
    fn test_position_operations() {
        let p1 = Position3D::new(1.0, 2.0, 3.0);
        let p2 = Position3D::new(4.0, 6.0, 8.0);
        
        let distance = p1.distance_to(&p2);
        let expected = ((3.0*3.0 + 4.0*4.0 + 5.0*5.0) as f64).sqrt();
        assert!((distance - expected).abs() < 1e-10);
    }

    #[test]
    fn test_gradient_operations() {
        let grad = Gradient3D::new(3.0, 4.0, 0.0);
        assert_eq!(grad.magnitude(), 5.0);
        
        let normalized = grad.normalize();
        assert!((normalized.magnitude() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_hessian_properties() {
        // Positive definite matrix
        let hessian = Hessian3D::new(2.0, 1.0, 3.0, 0.5, 0.1, 0.2);
        assert!(hessian.is_positive_definite());
        assert_eq!(hessian.trace(), 6.0);
    }
}