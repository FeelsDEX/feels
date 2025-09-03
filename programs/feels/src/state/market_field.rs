/// Market field commitment state for client-side optimal routing.
/// Provides minimal on-chain data enabling exact work computation via closed-form log work.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Market Field State
// ============================================================================

/// Minimal market state for field commitment strategy (Option A)
#[account]
#[derive(Default, Debug)]
#[allow(non_snake_case)]
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
    
    /// Token 0 weight within spot dimension (ω_a)
    /// Defaults to token weight if not specified
    pub omega_0: u32,
    
    /// Token 1 weight within spot dimension (ω_b)
    pub omega_1: u32,
    
    // ========== Risk Scalers (basis points) ==========
    
    /// Price volatility σ_price
    pub sigma_price: u64,
    
    /// Rate volatility σ_rate
    pub sigma_rate: u64,
    
    /// Leverage volatility σ_leverage
    pub sigma_leverage: u64,
    
    // ========== Internal TWAPs (Q64 in common numeraire) ==========
    
    /// Token 0 TWAP price
    pub twap_0: u128,
    
    /// Token 1 TWAP price
    pub twap_1: u128,
    
    // ========== Freshness & Validity ==========
    
    /// Snapshot timestamp
    pub snapshot_ts: i64,
    
    /// Maximum staleness before refresh required (seconds)
    pub max_staleness: i64,
    
    // ========== Commitment Hash ==========
    
    /// Deterministic hash of the field commitment payload
    pub commitment_hash: [u8; 32],
    
    // ========== Reserved ==========
    
    /// Reserved for future extensions (reduced by 32 bytes for hash)
    pub _reserved: [u8; 32],
}

impl MarketField {
    /// Check if field data is fresh
    pub fn is_fresh(&self, current_ts: i64) -> bool {
        current_ts - self.snapshot_ts <= self.max_staleness
    }
    
    /// Get the commitment hash
    pub fn get_commitment_hash(&self) -> [u8; 32] {
        self.commitment_hash
    }
    
    /// Validate field parameters are within bounds
    pub fn validate(&self) -> Result<()> {
        // Weights must sum to 10000
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
    
    /// Convert to MarketState for physics calculations
    pub fn to_market_state(&self) -> Result<crate::state::market_state::MarketState> {
        Ok(crate::state::market_state::MarketState {
            S: self.S,
            T: self.T,
            L: self.L,
            weights: crate::state::DomainWeights {
                w_s: self.w_s,
                w_t: self.w_t,
                w_l: self.w_l,
                w_tau: self.w_tau,
            },
        })
    }
}

// ============================================================================
// Work Calculation Helpers (for SDK)
// ============================================================================

/// Parameters for client-side work calculation
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
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

/// Calculate work using closed-form log work formula - DEPRECATED
/// This function is kept for backwards compatibility but should not be used directly.
/// Use market_field_work::calculate_work_for_market instead.
#[deprecated(note = "Use market_field_work module for proper work calculation")]
pub fn calculate_work_closed_form(params: &WorkCalculationParams) -> Result<i128> {
    msg!("Warning: calculate_work_closed_form is deprecated. Use market_field_work module.");
    
    // For backwards compatibility, call the deprecated calculate_ln_ratio
    // This will fail with InvalidOperation error
    let (w_hat_s, w_hat_t, w_hat_l) = params.field.get_hat_weights();
    
    // Calculate work components
    let mut work = 0i128;
    
    // Spot component
    if params.S_start != params.S_end && w_hat_s > 0 {
        #[allow(deprecated)]
        let ln_ratio = calculate_ln_ratio(params.S_end, params.S_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_s)?;
        work = work.saturating_sub(w_component);
    }
    
    // Time component
    if params.T_start != params.T_end && w_hat_t > 0 {
        #[allow(deprecated)]
        let ln_ratio = calculate_ln_ratio(params.T_end, params.T_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_t)?;
        work = work.saturating_sub(w_component);
    }
    
    // Leverage component
    if params.L_start != params.L_end && w_hat_l > 0 {
        #[allow(deprecated)]
        let ln_ratio = calculate_ln_ratio(params.L_end, params.L_start)?;
        let w_component = apply_weight(ln_ratio, w_hat_l)?;
        work = work.saturating_sub(w_component);
    }
    
    Ok(work)
}

/// Calculate ln(a/b) - DEPRECATED
/// This function is kept for backwards compatibility but should not be used.
/// Use market_field_work::calculate_work_for_market instead.
#[deprecated(note = "Use market_field_work module for proper work calculation")]
fn calculate_ln_ratio(_a: u128, _b: u128) -> Result<i128> {
    msg!("Warning: calculate_ln_ratio is deprecated. Use market_field_work module.");
    Err(FeelsProtocolError::InvalidOperation.into())
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
#[allow(non_snake_case)]
pub struct FieldUpdateParams {
    /// New market scalars
    pub S: u128,
    pub T: u128,
    pub L: u128,
    
    /// Updated TWAPs
    pub twap_0: u128,
    pub twap_1: u128,
    
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
            FeelsProtocolError::InvalidParameter
        );
        
        // TWAPs must be positive
        require!(
            self.twap_0 > 0 && self.twap_1 > 0,
            FeelsProtocolError::InvalidParameter
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
        32 +                      // commitment_hash
        32;                       // reserved
}