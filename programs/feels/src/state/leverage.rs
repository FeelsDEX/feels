/// Leverage system for continuous leverage functionality including protection curves
/// and risk profiles that determine position parameters based on leverage levels.
/// Implements dynamic risk management through protection curves that shield users
/// from excessive losses during market stress while maintaining capital efficiency.

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

// ============================================================================
// Protection Curve Types
// ============================================================================

/// Protection curve type identifier
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ProtectionCurveType {
    pub curve_type: u8, // 0 = Linear, 1 = Exponential, 2 = Piecewise
    pub _padding: [u8; 7],
}

/// Protection curve data (union-like structure for zero-copy)
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct ProtectionCurveData {
    pub decay_rate: u64, // For exponential
    pub points: [[u64; 2]; 8], // For piecewise
}

// ============================================================================
// Leverage Parameters
// ============================================================================

/// Leverage parameters for continuous leverage system
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct LeverageParameters {
    /// Maximum leverage allowed in this pool (6 decimals, e.g., 3000000 = 3x)
    pub max_leverage: u64,
    /// Current dynamic ceiling based on market conditions
    pub current_ceiling: u64,
    /// Protection curve type
    pub protection_curve_type: ProtectionCurveType,
    /// Protection curve data
    pub protection_curve_data: ProtectionCurveData,
    /// Last time ceiling was updated
    pub last_ceiling_update: u64,
    /// Padding for alignment
    pub _padding: [u8; 8],
}

// ============================================================================
// Risk Profile System
// ============================================================================

/// Risk profile calculated from leverage
#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct RiskProfile {
    /// Input leverage (6 decimals)
    pub leverage: u64,
    /// Protection factor derived from curve (6 decimals)
    pub protection_factor: u64,
    /// Fee multiplier for leveraged positions (6 decimals)
    pub fee_multiplier: u64,
    /// Maximum loss percentage during redenomination (6 decimals)
    pub max_loss_percentage: u64,
    /// Required margin ratio (6 decimals)
    pub required_margin_ratio: u64,
}

impl RiskProfile {
    pub const LEVERAGE_SCALE: u64 = 1_000_000; // 6 decimals
    pub const PROTECTION_SCALE: u64 = 1_000_000; // 6 decimals
    pub const MAX_LEVERAGE_SCALE: u64 = 10_000_000; // 10x max leverage (10 * 1_000_000)

    /// Calculate risk profile from leverage and pool parameters
    pub fn from_leverage(leverage: u64, params: &LeverageParameters) -> Result<Self> {
        // Validate leverage
        crate::utils::ErrorHandling::validate_leverage(
            leverage,
            Self::LEVERAGE_SCALE,
            params.current_ceiling,
        )?;

        // Calculate protection factor based on curve
        let protection = match params.protection_curve_type.curve_type {
            0 => { // Linear
                // protection = 1 - (leverage - 1) / (max - 1)
                let leverage_ratio = leverage.saturating_sub(Self::LEVERAGE_SCALE);
                let max_ratio = params.max_leverage.saturating_sub(Self::LEVERAGE_SCALE);
                if max_ratio == 0 {
                    Self::PROTECTION_SCALE
                } else {
                    Self::PROTECTION_SCALE.saturating_sub(
                        leverage_ratio.saturating_mul(Self::PROTECTION_SCALE) / max_ratio,
                    )
                }
            }
            1 => { // Exponential
                // Simplified exponential approximation
                // protection ≈ 1 / (1 + k * (leverage - 1))
                let leverage_excess = leverage.saturating_sub(Self::LEVERAGE_SCALE);
                let decay_rate = params.protection_curve_data.decay_rate;
                let denominator = Self::LEVERAGE_SCALE
                    + (decay_rate.saturating_mul(leverage_excess) / Self::LEVERAGE_SCALE);
                Self::PROTECTION_SCALE.saturating_mul(Self::LEVERAGE_SCALE) / denominator
            }
            2 => { // Piecewise
                // Find the right segment in the piecewise function
                let mut protection = Self::PROTECTION_SCALE;
                for [lev, prot] in params.protection_curve_data.points.iter() {
                    if leverage <= *lev {
                        protection = *prot;
                        break;
                    }
                }
                protection
            }
            _ => Self::PROTECTION_SCALE, // Default to full protection
        };

        // Calculate fee multiplier: sqrt(leverage)
        let fee_multiplier = crate::utils::sqrt_u64(leverage).saturating_mul(1_000) / 1_000; // Adjust scale

        // Calculate max loss percentage
        let max_loss = Self::PROTECTION_SCALE.saturating_sub(protection);

        // Calculate required margin ratio
        let margin = Self::LEVERAGE_SCALE.saturating_mul(Self::LEVERAGE_SCALE) / leverage
            + Self::calculate_buffer(leverage);

        Ok(Self {
            leverage,
            protection_factor: protection,
            fee_multiplier,
            max_loss_percentage: max_loss,
            required_margin_ratio: margin,
        })
    }

    /// Calculate additional margin buffer based on leverage
    fn calculate_buffer(leverage: u64) -> u64 {
        // Higher leverage requires more buffer
        // buffer = 0.05 * (leverage - 1)
        (leverage.saturating_sub(Self::LEVERAGE_SCALE)).saturating_mul(50_000)
            / Self::LEVERAGE_SCALE
    }
    
    /// Convert leverage to tick for 3D encoding
    pub fn to_tick(&self) -> i16 {
        // Map leverage (1-10x) to tick space (0-63)
        let normalized = self.leverage.saturating_sub(Self::LEVERAGE_SCALE) / Self::LEVERAGE_SCALE;
        (normalized * 7).min(63) as i16
    }
    
    /// Create risk profile from leverage value with pool context
    pub fn from_leverage_with_pool(leverage: u64, pool: &crate::state::Pool) -> Result<Self> {
        // Get leverage parameters from pool
        let params = pool.leverage_params;
        
        Self::from_leverage(leverage, &params)
    }
}

// ============================================================================
// Leverage Statistics
// ============================================================================

/// Pool-wide leverage statistics for risk monitoring
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct LeverageStatistics {
    /// Total value at each leverage tier
    pub total_value_by_tier: [u64; 4], // None, Low, Medium, High
    
    /// Average leverage across all positions (LEVERAGE_SCALE units)
    pub average_leverage: u64,
    
    /// Maximum leverage currently in use
    pub max_leverage_in_use: u64,
    
    /// Total margin locked
    pub total_margin_locked: u64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Total base liquidity (without leverage)
    pub total_base_liquidity: u128,
    
    /// Total leveraged liquidity (with leverage applied)
    pub total_leveraged_liquidity: u128,
    
    /// Number of positions at each tier
    pub position_count_by_tier: [u32; 4],
    
    /// Positions at risk of liquidation
    pub positions_at_risk: u32,
    
    /// Leveraged position count
    pub leveraged_position_count: u32,
    
    /// Padding for alignment
    pub _padding: u64,
}