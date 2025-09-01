/// Market field commitment state for client-side optimal routing.
/// Provides minimal on-chain data enabling exact work computation via closed-form log work.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Market Field State
// ============================================================================

/// Minimal market state for field commitment strategy (Option A)
#[account(zero_copy)]
#[derive(Default)]
#[repr(C, packed)]
pub struct MarketField {
    /// Pool this field belongs to
    pub pool: Pubkey,
    
    // ========== Market Scalars (Fixed-Point Q64) ==========
    
    /// Spot dimension scalar S
    pub S: u128,
    
    /// Time dimension scalar T
    pub T: u128,
    
    /// Leverage dimension scalar L
    pub L: u128,
    
    // ========== Domain Weights (basis points) ==========
    
    /// Spot weight w_s
    pub w_s: u32,
    
    /// Time weight w_t
    pub w_t: u32,
    
    /// Leverage weight w_l
    pub w_l: u32,
    
    /// Buffer weight w_τ
    pub w_tau: u32,
    
    // ========== Spot Value Weights ==========
    
    /// Token A weight within spot dimension (ω_a)
    /// Defaults to token weight if not specified
    pub omega_a: u32,
    
    /// Token B weight within spot dimension (ω_b)
    pub omega_b: u32,
    
    // ========== Risk Scalers (basis points) ==========
    
    /// Price volatility σ_price
    pub sigma_price: u64,
    
    /// Rate volatility σ_rate
    pub sigma_rate: u64,
    
    /// Leverage volatility σ_leverage
    pub sigma_leverage: u64,
    
    // ========== Internal TWAPs (Q64 in common numeraire) ==========
    
    /// Token A TWAP price
    pub twap_a: u128,
    
    /// Token B TWAP price
    pub twap_b: u128,
    
    // ========== Freshness & Validity ==========
    
    /// Snapshot timestamp
    pub snapshot_ts: i64,
    
    /// Maximum staleness before refresh required (seconds)
    pub max_staleness: i64,
    
    // ========== Reserved ==========
    
    /// Reserved for future extensions
    pub _reserved: [u8; 64],
}

impl MarketField {
    /// Check if field data is fresh
    pub fn is_fresh(&self, current_ts: i64) -> bool {
        current_ts - self.snapshot_ts <= self.max_staleness
    }
    
    /// Validate field parameters are within bounds
    pub fn validate(&self) -> Result<()> {
        // Weights must sum to 10000
        require!(
            self.w_s + self.w_t + self.w_l + self.w_tau == 10000,
            FeelsProtocolError::InvalidWeights { 
                description: "Domain weights must sum to 10000".to_string() 
            }
        );
        
        // Spot weights must sum to 10000
        require!(
            self.omega_a + self.omega_b == 10000,
            FeelsProtocolError::InvalidWeights { 
                description: "Spot weights must sum to 10000".to_string() 
            }
        );
        
        // Scalars must be positive
        require!(
            self.S > 0 && self.T > 0 && self.L > 0,
            FeelsProtocolError::InvalidParameter { 
                param: "Market scalars".to_string(),
                reason: "Must be positive".to_string()
            }
        );
        
        // TWAPs must be positive
        require!(
            self.twap_a > 0 && self.twap_b > 0,
            FeelsProtocolError::InvalidParameter { 
                param: "TWAPs".to_string(),
                reason: "Must be positive".to_string()
            }
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
// Work Calculation Helpers (for SDK)
// ============================================================================

/// Parameters for client-side work calculation
#[derive(Clone, Debug)]
pub struct WorkCalculationParams {
    /// Start position scalars
    pub S_start: u128,
    pub T_start: u128,
    pub L_start: u128,
    
    /// End position scalars
    pub S_end: u128,
    pub T_end: u128,
    pub L_end: u128,
    
    /// Market field data
    pub field: MarketField,
}

/// Calculate work using closed-form log work formula
/// W = -ŵ_s · ln(S_end/S_start) - ŵ_t · ln(T_end/T_start) - ŵ_l · ln(L_end/L_start)
pub fn calculate_work_closed_form(params: &WorkCalculationParams) -> Result<i128> {
    let (w_hat_s, w_hat_t, w_hat_l) = params.field.get_hat_weights();
    
    // Calculate work components
    let mut work = 0i128;
    
    // Spot component
    if params.S_start != params.S_end && w_hat_s > 0 {
        let ln_ratio = calculate_ln_ratio(params.S_end, params.S_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_s)?;
        work = work.saturating_sub(w_component);
    }
    
    // Time component
    if params.T_start != params.T_end && w_hat_t > 0 {
        let ln_ratio = calculate_ln_ratio(params.T_end, params.T_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_t)?;
        work = work.saturating_sub(w_component);
    }
    
    // Leverage component
    if params.L_start != params.L_end && w_hat_l > 0 {
        let ln_ratio = calculate_ln_ratio(params.L_end, params.L_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_l)?;
        work = work.saturating_sub(w_component);
    }
    
    Ok(work)
}

/// Calculate ln(a/b) in fixed point
fn calculate_ln_ratio(a: u128, b: u128) -> Result<i128> {
    use crate::logic::market_physics::potential::{ln_fixed, FixedPoint};
    
    // ln(a/b) = ln(a) - ln(b)
    let ln_a = ln_fixed(a)?.value;
    let ln_b = ln_fixed(b)?.value;
    
    Ok(ln_a.saturating_sub(ln_b))
}

/// Apply weight to value
fn apply_weight(value: i128, weight: u64) -> Result<i128> {
    // weight is in basis points, convert to fixed point
    let weight_fp = (weight as i128 * (1i128 << 64)) / 10000;
    
    // Multiply and scale back
    let result = (value.saturating_mul(weight_fp)) >> 64;
    
    Ok(result)
}

// ============================================================================
// Field Update Parameters
// ============================================================================

/// Parameters for updating market field data
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FieldUpdateParams {
    /// New market scalars
    pub S: u128,
    pub T: u128,
    pub L: u128,
    
    /// Updated TWAPs
    pub twap_a: u128,
    pub twap_b: u128,
    
    /// Updated risk scalers (optional, 0 means no change)
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
}

impl FieldUpdateParams {
    /// Validate update parameters
    pub fn validate(&self) -> Result<()> {
        // Scalars must be positive
        require!(
            self.S > 0 && self.T > 0 && self.L > 0,
            FeelsProtocolError::InvalidParameter { 
                param: "Market scalars".to_string(),
                reason: "Must be positive".to_string()
            }
        );
        
        // TWAPs must be positive
        require!(
            self.twap_a > 0 && self.twap_b > 0,
            FeelsProtocolError::InvalidParameter { 
                param: "TWAPs".to_string(),
                reason: "Must be positive".to_string()
            }
        );
        
        Ok(())
    }
}

// ============================================================================
// Size Constants
// ============================================================================

impl MarketField {
    pub const SIZE: usize = 8 +  // discriminator
        32 +                      // pool pubkey
        16 * 3 +                  // S, T, L scalars
        4 * 4 +                   // weights
        4 * 2 +                   // omega weights
        8 * 3 +                   // risk scalers
        16 * 2 +                  // TWAPs
        8 * 2 +                   // timestamps
        64;                       // reserved
}