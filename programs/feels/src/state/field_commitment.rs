/// Field commitment structure for keeper-provided market physics data.
/// Gradient gradient Hessian computation and posts compact keeper updates.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::utils::math::safe;

// ============================================================================
// Field Commitment Structure
// ============================================================================

/// Compact field commitment containing market physics state
/// Enables client-side analytical routing without dense gradient tables
#[account(zero_copy)]
#[derive(Debug)]
#[repr(C, packed)]
#[allow(non_snake_case)]
pub struct FieldCommitment {
    /// Pool this commitment belongs to
    pub pool: Pubkey,
    
    // ========== Minimal Scalar Snapshot (Option A) ==========
    
    /// Spot dimension scalar S
    pub S: u128,
    
    /// Time dimension scalar T  
    pub T: u128,
    
    /// Leverage dimension scalar L
    pub L: u128,
    
    /// Domain weights (basis points)
    pub w_s: u32,
    pub w_t: u32, 
    pub w_l: u32,
    pub w_tau: u32,
    
    /// Spot value weights (basis points)
    pub omega_0: u32,
    pub omega_1: u32,
    
    /// Risk scalers (basis points)
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
    
    /// Internal TWAPs for price mapping
    pub twap_0: u128,
    pub twap_1: u128,
    
    // ========== Timestamps and Freshness ==========
    
    /// Commitment snapshot timestamp
    pub snapshot_ts: i64,
    
    /// Maximum staleness before fallback (seconds)
    pub max_staleness: i64,
    
    // ========== Optional Local Quadratic Coefficients (Option B) ==========
    
    /// Local quadratic coefficients for improved accuracy (0 = not set)
    pub c0_s: i128, // Linear coefficient for spot
    pub c1_s: i128, // Quadratic coefficient for spot
    pub c0_t: i128, // Linear coefficient for time
    pub c1_t: i128, // Quadratic coefficient for time
    pub c0_l: i128, // Linear coefficient for leverage
    pub c1_l: i128, // Quadratic coefficient for leverage
    
    /// Validity window for coefficients (0 = not set)
    pub coeff_valid_until: i128,
    
    // ========== Optional Micro-Field Commitment ==========
    
    /// Commitment root for verifiable micro-fields (all zeros if not set)
    pub root: [u8; 32],
    
    /// Global Lipschitz constant for gradient bounds (0 if not set)
    pub lipschitz_L: u64,
    
    /// Curvature bounds (min, max) - (0, 0) if not set
    pub curvature_bounds_min: u64,
    pub curvature_bounds_max: u64,
    
    // ========== Oracle Metadata ==========
    
    /// Oracle that provided this commitment
    pub oracle: Pubkey,
    
    /// Commitment update sequence number
    pub sequence: u64,
    
    /// Oracle signature for verification
    pub signature: [u8; 64],
    
    // ========== Dynamic Fee Parameters ==========
    
    /// Current base fee in basis points (from hysteresis controller)
    pub base_fee_bps: u64,
    
    // ========== Reserved ==========
    
    /// Reserved for future extensions (reduced by 8 bytes for base_fee_bps)
    pub _reserved1: [u8; 32],
    pub _reserved2: [u8; 32],
    pub _reserved3: [u8; 32],
    pub _reserved4: [u8; 24],
}

impl Default for FieldCommitment {
    fn default() -> Self {
        Self {
            pool: Pubkey::default(),
            S: 0,
            T: 0,
            L: 0,
            w_s: 0,
            w_t: 0,
            w_l: 0,
            w_tau: 0,
            omega_0: 0,
            omega_1: 0,
            sigma_price: 0,
            sigma_rate: 0,
            sigma_leverage: 0,
            twap_0: 0,
            twap_1: 0,
            snapshot_ts: 0,
            max_staleness: 0,
            c0_s: 0,
            c1_s: 0,
            c0_t: 0,
            c1_t: 0,
            c0_l: 0,
            c1_l: 0,
            coeff_valid_until: 0,
            root: [0u8; 32],
            lipschitz_L: 0,
            curvature_bounds_min: 0,
            curvature_bounds_max: 0,
            oracle: Pubkey::default(),
            sequence: 0,
            signature: [0u8; 64],
            base_fee_bps: 0,
            _reserved1: [0u8; 32],
            _reserved2: [0u8; 32],
            _reserved3: [0u8; 32],
            _reserved4: [0u8; 24],
        }
    }
}

impl FieldCommitment {
    /// Size calculation for account allocation
    pub const SIZE: usize = 8 +  // discriminator
        32 +                      // pool
        16 * 3 +                  // S, T, L scalars
        4 * 4 +                   // domain weights
        4 * 2 +                   // omega weights
        8 * 3 +                   // risk scalers
        16 * 2 +                  // TWAPs
        8 * 2 +                   // timestamps
        (8 + 16) * 6 +           // optional coefficients
        8 +                       // coeff_valid_until
        (8 + 32) +               // optional root
        8 +                       // lipschitz_L
        (8 + 16) +               // curvature_bounds
        32 +                      // oracle
        8 +                       // sequence
        64 +                      // signature
        8 +                       // base_fee_bps
        120;                      // reserved

    /// Check if commitment is fresh enough for use
    pub fn is_fresh(&self, current_ts: i64) -> bool {
        current_ts - self.snapshot_ts <= self.max_staleness
    }
    
    /// Check if local coefficients are valid
    pub fn coeffs_valid(&self, current_ts: i64) -> bool {
        if self.coeff_valid_until == 0 {
            false
        } else {
            current_ts as i128 <= self.coeff_valid_until
        }
    }
    
    /// Validate commitment structure and values
    pub fn validate(&self) -> Result<()> {
        // Domain weights must sum to 10000
        require!(
            self.w_s + self.w_t + self.w_l + self.w_tau == 10000,
            FeelsProtocolError::InvalidWeights
        );
        
        // Spot weights must sum to 10000
        require!(
            self.omega_0 + self.omega_1 == 10000,
            FeelsProtocolError::InvalidWeights
        );
        
        // Scalars must be positive
        require!(
            self.S > 0 && self.T > 0 && self.L > 0,
            FeelsProtocolError::InvalidParameter
        );
        
        // TWAPs must be positive
        require!(
            self.twap_0 > 0 && self.twap_1 > 0,
            FeelsProtocolError::InvalidParameter
        );
        
        // Sequence number monotonicity (caller should check against previous)
        require!(
            self.sequence > 0,
            FeelsProtocolError::InvalidParameter
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
// Client-Side Route Computation Helpers
// ============================================================================

/// Parameters for client-side analytical route computation
#[derive(Clone, Debug)]
pub struct RouteComputationParams {
    /// Field commitment data
    pub field: FieldCommitment,
    
    /// Route segments (planned tick traversal)
    pub segments: Vec<RouteSegment>,
    
    /// Total route distance for scaling
    pub total_distance: u64,
}

/// Individual route segment for piecewise computation
#[derive(Clone, Debug)]
pub struct RouteSegment {
    /// Starting position in this segment
    pub start_s: u128,
    pub start_t: u128,
    pub start_l: u128,
    
    /// Ending position in this segment  
    pub end_s: u128,
    pub end_t: u128,
    pub end_l: u128,
    
    /// Segment length for integration
    pub length: u64,
}

impl RouteSegment {
    /// Calculate work for this segment
    /// NOTE: This is a placeholder - actual work calculation must be done off-chain
    /// The keeper/oracle provides pre-computed work values in the field commitment
    pub fn calculate_work(&self, _field: &FieldCommitment) -> Result<i128> {
        // On-chain programs cannot compute logarithms efficiently
        // Work values must come from field commitments or be computed off-chain
        Err(FeelsProtocolError::InvalidOperation.into())
    }
    
    /// Add optional quadratic correction if coefficients are available
    pub fn calculate_quadratic_correction(&self, field: &FieldCommitment) -> Result<i128> {
        if !field.coeffs_valid(0) { // Current timestamp check should be done by caller
            return Ok(0);
        }
        
        let mut correction = 0i128;
        
        // Spot quadratic: c0_s * dx + 0.5 * c1_s * dx^2
        if field.c0_s != 0 || field.c1_s != 0 {
            let dx = safe::sub_i128(self.end_s as i128, self.start_s as i128)?;
            let linear_term = safe::mul_i128(field.c0_s, dx)?;
            let quadratic_term = safe::div_i128(
                safe::mul_i128(field.c1_s, safe::mul_i128(dx, dx)?)?,
                2
            )?;
            correction = safe::add_i128(correction, safe::add_i128(linear_term, quadratic_term)?)?;
        }
        
        // Time quadratic: c0_t * dt + 0.5 * c1_t * dt^2
        if field.c0_t != 0 || field.c1_t != 0 {
            let dt = safe::sub_i128(self.end_t as i128, self.start_t as i128)?;
            let linear_term = safe::mul_i128(field.c0_t, dt)?;
            let quadratic_term = safe::div_i128(
                safe::mul_i128(field.c1_t, safe::mul_i128(dt, dt)?)?,
                2
            )?;
            correction = safe::add_i128(correction, safe::add_i128(linear_term, quadratic_term)?)?;
        }
        
        // Leverage quadratic: c0_l * dl + 0.5 * c1_l * dl^2
        if field.c0_l != 0 || field.c1_l != 0 {
            let dl = safe::sub_i128(self.end_l as i128, self.start_l as i128)?;
            let linear_term = safe::mul_i128(field.c0_l, dl)?;
            let quadratic_term = safe::div_i128(
                safe::mul_i128(field.c1_l, safe::mul_i128(dl, dl)?)?,
                2
            )?;
            correction = safe::add_i128(correction, safe::add_i128(linear_term, quadratic_term)?)?;
        }
        
        Ok(correction)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================
// Work Calculation Helpers
// ============================================================================
// NOTE: ln(a/b) calculation is done off-chain for precision
