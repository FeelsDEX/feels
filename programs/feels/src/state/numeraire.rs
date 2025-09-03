/// Protocol numeraire system for unified value measurement across all dimensions.
/// All values in the market physics model are measured in a single numeraire N
/// using internal protocol TWAPs, avoiding external oracle dependencies.
use anchor_lang::prelude::*;
// use std::collections::BTreeMap; // Unused import
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Default TWAP window (15 minutes)
pub const DEFAULT_TWAP_WINDOW: i64 = 900;

/// Minimum observations for valid TWAP
pub const MIN_TWAP_OBSERVATIONS: u32 = 10;

/// Maximum staleness for cached rates (5 minutes)
pub const MAX_RATE_STALENESS: i64 = 300;

/// Fixed-point precision for rates (Q64)
pub const RATE_PRECISION: u128 = 1 << 64;

// ============================================================================
// Conversion Rate Structure
// ============================================================================

/// Cached conversion rate with confidence metrics
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConversionRate {
    /// Exchange rate to numeraire (Q64 fixed-point)
    pub rate: u128,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Confidence level (basis points, 10000 = 100%)
    pub confidence: u32,
    
    /// Number of observations in TWAP
    pub observations: u32,
    
    /// Geometric mean TWAP price
    pub twap_price: u128,
}

impl ConversionRate {
    /// Check if rate is stale
    pub fn is_stale(&self, current_time: i64) -> bool {
        current_time - self.last_update > MAX_RATE_STALENESS
    }
    
    /// Check if rate has sufficient confidence
    pub fn has_sufficient_confidence(&self, min_confidence: u32) -> bool {
        self.confidence >= min_confidence && self.observations >= MIN_TWAP_OBSERVATIONS
    }
}

// ============================================================================
// Protocol Numeraire Account
// ============================================================================

/// Protocol numeraire configuration and state
#[account]
pub struct ProtocolNumeraire {
    /// Authority who can update numeraire settings
    pub authority: Pubkey,
    
    /// Base asset used as numeraire (e.g., FeelsSOL, USDC)
    pub base_asset: Pubkey,
    
    /// TWAP window duration in seconds
    pub twap_window: i64,
    
    /// Minimum observations required for valid TWAP
    pub min_observations: u32,
    
    /// Minimum confidence level required (basis points)
    pub min_confidence: u32,
    
    /// Whether to use geometric mean (true) or arithmetic mean (false)
    pub use_geometric_mean: bool,
    
    /// Maximum allowed price deviation from TWAP (basis points)
    pub max_deviation_bps: u16,
    
    /// Emergency fallback mode enabled
    pub fallback_enabled: bool,
    
    /// Fallback fixed rates if TWAP unavailable
    pub fallback_rates: Vec<FallbackRate>,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

/// Fallback rate for emergency scenarios
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FallbackRate {
    /// Token to convert
    pub token: Pubkey,
    
    /// Fixed conversion rate to numeraire
    pub rate: u128,
}

// ============================================================================
// Numeraire Cache Account
// ============================================================================

/// Cached conversion rates for efficient access
#[account]
pub struct NumeraireCache {
    /// Protocol numeraire reference
    pub numeraire: Pubkey,
    
    /// Market field this cache belongs to
    pub market_field: Pubkey,
    
    /// Token 0 conversion rate
    pub rate_0: ConversionRate,
    
    /// Token 1 conversion rate  
    pub rate_1: ConversionRate,
    
    /// Additional token rates (for multi-asset pools)
    pub additional_rates: Vec<(Pubkey, ConversionRate)>,
    
    /// Last full cache update
    pub last_update: i64,
    
    /// Cache validity period
    pub validity_period: i64,
}

impl NumeraireCache {
    /// Convert token amount to numeraire value
    pub fn to_numeraire(&self, token: &Pubkey, amount: u64) -> Result<u128> {
        let rate = if token == &self.market_field {
            // Assuming token 0 for now, should check actual token
            &self.rate_0
        } else {
            // Check additional rates
            self.additional_rates
                .iter()
                .find(|(t, _)| t == token)
                .map(|(_, r)| r)
                .ok_or(FeelsProtocolError::TokenNotFound)?
        };
        
        // Check staleness
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            !rate.is_stale(current_time),
            FeelsProtocolError::StalePrice
        );
        
        // Convert: amount * rate / RATE_PRECISION
        (amount as u128)
            .checked_mul(rate.rate)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(RATE_PRECISION)
            .ok_or(FeelsProtocolError::DivisionByZero.into())
    }
    
    /// Convert numeraire value to token amount
    pub fn from_numeraire(&self, token: &Pubkey, numeraire_value: u128) -> Result<u64> {
        let rate = if token == &self.market_field {
            &self.rate_0
        } else {
            self.additional_rates
                .iter()
                .find(|(t, _)| t == token)
                .map(|(_, r)| r)
                .ok_or(FeelsProtocolError::TokenNotFound)?
        };
        
        // Check staleness
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            !rate.is_stale(current_time),
            FeelsProtocolError::StalePrice
        );
        
        // Convert: numeraire_value * RATE_PRECISION / rate
        let amount = numeraire_value
            .checked_mul(RATE_PRECISION)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(rate.rate)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        // Check overflow
        require!(
            amount <= u64::MAX as u128,
            FeelsProtocolError::MathOverflow
        );
        
        Ok(amount as u64)
    }
}

// ============================================================================
// TWAP Calculation
// ============================================================================

/// Price observation for TWAP calculation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PriceObservation {
    /// Observed price (Q64)
    pub price: u128,
    
    /// Observation timestamp
    pub timestamp: i64,
    
    /// Liquidity at observation
    pub liquidity: u128,
}

/// Calculate geometric mean TWAP from observations - OFF-CHAIN ONLY
/// This function is provided as a reference but CANNOT be executed on-chain.
/// Keepers must compute TWAP off-chain and submit results via numeraire_twap module.
#[cfg(feature = "off-chain-only")]
pub fn calculate_geometric_twap(
    observations: &[PriceObservation],
    window_start: i64,
    window_end: i64,
) -> Result<u128> {
    require!(
        observations.len() >= MIN_TWAP_OBSERVATIONS as usize,
        FeelsProtocolError::InsufficientObservations
    );
    
    // Filter observations within window
    let mut filtered: Vec<&PriceObservation> = observations
        .iter()
        .filter(|obs| obs.timestamp >= window_start && obs.timestamp <= window_end)
        .collect();
    
    require!(
        filtered.len() >= MIN_TWAP_OBSERVATIONS as usize,
        FeelsProtocolError::InsufficientObservations
    );
    
    // Sort by timestamp
    filtered.sort_by_key(|obs| obs.timestamp);
    
    // Calculate time-weighted geometric mean
    let mut log_price_sum: i128 = 0;
    let mut total_weight: i64 = 0;
    
    for i in 0..filtered.len() - 1 {
        let current = filtered[i];
        let next = filtered[i + 1];
        
        // Time weight
        let time_delta = next.timestamp - current.timestamp;
        if time_delta <= 0 {
            continue;
        }
        
        // Log price (approximation for small deviations)
        let ln_price = calculate_ln_approx(current.price)?;
        
        // Weighted sum
        log_price_sum = log_price_sum
            .checked_add(
                ln_price
                    .checked_mul(time_delta as i128)
                    .ok_or(FeelsProtocolError::MathOverflow)?
            )
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        total_weight += time_delta;
    }
    
    require!(total_weight > 0, FeelsProtocolError::InvalidInput);
    
    // Average log price
    let avg_ln_price = log_price_sum
        .checked_div(total_weight as i128)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    // Convert back from log space
    calculate_exp_approx(avg_ln_price)
}

/// Calculate natural log - OFF-CHAIN ONLY
/// This function is feature-gated and only available for off-chain use.
/// On-chain code must use numeraire_twap module for TWAP submissions.
#[cfg(feature = "off-chain-only")]
fn calculate_ln_approx(x: u128) -> Result<i128> {
    // This would use high-precision off-chain math libraries
    // For example, using f64 or arbitrary precision arithmetic
    unimplemented!("Use off-chain math libraries for ln calculation")
}

/// Calculate exponential - OFF-CHAIN ONLY
/// This function is feature-gated and only available for off-chain use.
/// On-chain code must use numeraire_twap module for TWAP submissions.
#[cfg(feature = "off-chain-only")]
fn calculate_exp_approx(x: i128) -> Result<u128> {
    // This would use high-precision off-chain math libraries
    // For example, using f64 or arbitrary precision arithmetic
    unimplemented!("Use off-chain math libraries for exp calculation")
}

/// Guard functions to prevent accidental on-chain usage
#[cfg(not(feature = "off-chain-only"))]
fn calculate_ln_approx(_x: u128) -> Result<i128> {
    msg!("Error: ln calculation cannot be performed on-chain");
    msg!("Use numeraire_twap module to submit pre-calculated TWAP");
    Err(FeelsProtocolError::InvalidOperation.into())
}

#[cfg(not(feature = "off-chain-only"))]
fn calculate_exp_approx(_x: i128) -> Result<u128> {
    msg!("Error: exp calculation cannot be performed on-chain");
    msg!("Use numeraire_twap module to submit pre-calculated TWAP");
    Err(FeelsProtocolError::InvalidOperation.into())
}

// ============================================================================
// Instructions Context
// ============================================================================

#[derive(Accounts)]
pub struct InitializeNumeraire<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8 + 4 + 4 + 1 + 2 + 1 + 64 + 256, // Approximate size
        seeds = [b"numeraire", authority.key().as_ref()],
        bump
    )]
    pub numeraire: Account<'info, ProtocolNumeraire>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateNumeraireCache<'info> {
    #[account(mut)]
    pub cache: Account<'info, NumeraireCache>,
    
    pub numeraire: Account<'info, ProtocolNumeraire>,
    
    pub market_field: Account<'info, crate::state::MarketField>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
}