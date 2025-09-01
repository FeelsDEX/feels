/// Unified market data source interface for field updates.
/// Provides a single interface for market data updates from various sources.
use anchor_lang::prelude::*;
use crate::state::{MarketField, FieldUpdateParams};
use crate::error::FeelsProtocolError;

// ============================================================================
// Market Data Source
// ============================================================================

/// Unified market data source feeding field updates
#[account]
pub struct MarketDataSource {
    /// Pool this source provides data for
    pub pool: Pubkey,
    
    /// Primary data provider
    pub primary_provider: Pubkey,
    
    /// Secondary provider (for redundancy)
    pub secondary_provider: Pubkey,
    
    /// Minimum time between updates (seconds)
    pub update_frequency: i64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Update counter
    pub update_count: u64,
    
    /// Whether this source is active
    pub is_active: bool,
}

impl MarketDataSource {
    pub const SIZE: usize = 8 + // discriminator
        32 +                     // pool
        32 +                     // primary_provider
        32 +                     // secondary_provider
        8 +                      // update_frequency
        8 +                      // last_update
        8 +                      // update_count
        1;                       // is_active
    
    /// Validate provider is authorized
    pub fn is_authorized(&self, provider: &Pubkey) -> bool {
        self.is_active && (
            provider == &self.primary_provider ||
            provider == &self.secondary_provider
        )
    }
    
    /// Check if update is allowed based on frequency
    pub fn can_update(&self, current_time: i64) -> bool {
        current_time >= self.last_update + self.update_frequency
    }
}

// ============================================================================
// Market Update Types
// ============================================================================

/// Unified market update that can be either simple parameters or commitment
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum MarketUpdate {
    /// Simple parameter snapshot (Option A - default)
    ParamSnapshot(ParamSnapshot),
    
    /// Gradient commitment (Option B - future extension)
    /// Not implemented in current version
    GradientCommitment(GradientCommitment),
}

/// Simple parameter snapshot for field updates
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ParamSnapshot {
    /// Market scalars
    pub S: u128,
    pub T: u128,
    pub L: u128,
    
    /// Updated TWAPs
    pub twap_a: u128,
    pub twap_b: u128,
    
    /// Risk scalers (optional updates)
    pub sigma_price: Option<u64>,
    pub sigma_rate: Option<u64>,
    pub sigma_leverage: Option<u64>,
    
    /// Update timestamp
    pub timestamp: i64,
    
    /// Provider that generated this update
    pub provider: Pubkey,
}

impl ParamSnapshot {
    /// Convert to field update params
    pub fn to_field_update_params(&self) -> FieldUpdateParams {
        FieldUpdateParams {
            S: self.S,
            T: self.T,
            L: self.L,
            twap_a: self.twap_a,
            twap_b: self.twap_b,
            sigma_price: self.sigma_price.unwrap_or(0),
            sigma_rate: self.sigma_rate.unwrap_or(0),
            sigma_leverage: self.sigma_leverage.unwrap_or(0),
        }
    }
    
    /// Validate snapshot parameters
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
        
        // Risk scalers must be reasonable if provided
        if let Some(sigma) = self.sigma_price {
            require!(
                sigma <= 100_000, // Max 1000%
                FeelsProtocolError::InvalidParameter {
                    param: "sigma_price".to_string(),
                    reason: "Too high".to_string()
                }
            );
        }
        
        Ok(())
    }
}

/// Placeholder for gradient commitment (not implemented)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GradientCommitment {
    /// Commitment root
    pub root: [u8; 32],
    
    /// Bounds
    pub lipschitz_constant: u64,
    pub curvature_bound: u64,
    
    /// Optimality gap
    pub gap_bps: u16,
    
    /// Timestamp
    pub timestamp: i64,
}

// ============================================================================
// Verification
// ============================================================================

/// Verify market update before applying
pub fn verify_market_update(
    update: &MarketUpdate,
    source: &MarketDataSource,
    field: &MarketField,
    current_time: i64,
) -> Result<()> {
    match update {
        MarketUpdate::ParamSnapshot(snapshot) => {
            // Verify provider is authorized
            require!(
                source.is_authorized(&snapshot.provider),
                FeelsProtocolError::UnauthorizedAccess {
                    action: "market_update".to_string()
                }
            );
            
            // Verify update frequency
            require!(
                source.can_update(current_time),
                FeelsProtocolError::UpdateTooFrequent {
                    min_interval: source.update_frequency,
                    elapsed: current_time - source.last_update,
                }
            );
            
            // Verify freshness
            require!(
                current_time - snapshot.timestamp <= field.max_staleness,
                FeelsProtocolError::StaleData {
                    max_age: field.max_staleness,
                    actual_age: current_time - snapshot.timestamp,
                }
            );
            
            // Validate parameters
            snapshot.validate()?;
            
            // Verify rate of change caps
            verify_rate_of_change(snapshot, field)?;
            
            Ok(())
        }
        
        MarketUpdate::GradientCommitment(_) => {
            // Not implemented in current version
            Err(FeelsProtocolError::NotImplemented {
                feature: "GradientCommitment".to_string()
            }.into())
        }
    }
}

/// Verify rate of change is within acceptable bounds
fn verify_rate_of_change(
    snapshot: &ParamSnapshot,
    current_field: &MarketField,
) -> Result<()> {
    // Maximum allowed change per update (2%)
    const MAX_CHANGE_BPS: u32 = 200;
    
    // Check S change
    let s_change = calculate_change_bps(current_field.S, snapshot.S);
    require!(
        s_change <= MAX_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange {
            field: "S".to_string(),
            change_bps: s_change,
            max_bps: MAX_CHANGE_BPS,
        }
    );
    
    // Check T change
    let t_change = calculate_change_bps(current_field.T, snapshot.T);
    require!(
        t_change <= MAX_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange {
            field: "T".to_string(),
            change_bps: t_change,
            max_bps: MAX_CHANGE_BPS,
        }
    );
    
    // Check L change
    let l_change = calculate_change_bps(current_field.L, snapshot.L);
    require!(
        l_change <= MAX_CHANGE_BPS,
        FeelsProtocolError::ExcessiveChange {
            field: "L".to_string(),
            change_bps: l_change,
            max_bps: MAX_CHANGE_BPS,
        }
    );
    
    Ok(())
}

/// Calculate change in basis points
fn calculate_change_bps(old: u128, new: u128) -> u32 {
    if old == 0 {
        return if new == 0 { 0 } else { 10000 };
    }
    
    let change = if new > old {
        new - old
    } else {
        old - new
    };
    
    ((change * 10000) / old).min(10000) as u32
}