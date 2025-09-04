//! # Position State - Consolidated Position and Leverage Management
//! 
//! This module consolidates all position-related state structures:
//! - NFT-based liquidity positions with concentrated liquidity
//! - Leverage parameters and protection curves
//! - Risk profiles and position tracking
//! - Fee accumulation and duration dimensions
//! 
//! ## Position Model
//! 
//! Positions in Feels are NFT-represented liquidity contributions that:
//! - Define a tick range for concentrated liquidity
//! - Track accumulated fees proportional to in-range time
//! - Support continuous leverage with protection curves
//! - Include time dimensions for the 3D thermodynamic model

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use crate::error::FeelsProtocolError;
use crate::state::rebase::RebaseCheckpoint;
use crate::utils::math::safe;

// ============================================================================
// Leverage System - Protection Curves and Risk Management
// ============================================================================

/// Protection curve type identifier
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ProtectionCurveType {
    pub curve_type: u8, // 0 = Linear, 1 = Exponential, 2 = Piecewise
    pub _padding: [u8; 7],
}

impl ProtectionCurveType {
    pub const LINEAR: u8 = 0;
    pub const EXPONENTIAL: u8 = 1;
    pub const PIECEWISE: u8 = 2;
}

/// Protection curve data (union-like structure for zero-copy)
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct ProtectionCurveData {
    pub decay_rate: u64, // For exponential
    pub points: [[u64; 2]; 8], // For piecewise (leverage, protection) pairs
}

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

impl LeverageParameters {
    /// Calculate protection level for given leverage
    pub fn calculate_protection(&self, leverage: u64) -> Result<u64> {
        match self.protection_curve_type.curve_type {
            ProtectionCurveType::LINEAR => {
                // Linear decay: protection = max_protection * (1 - leverage/max_leverage)
                let max_protection = 10000; // 100% in basis points
                if leverage >= self.max_leverage {
                    return Ok(0);
                }
                let ratio = (leverage * 10000) / self.max_leverage;
                Ok(max_protection.saturating_sub(ratio))
            }
            ProtectionCurveType::EXPONENTIAL => {
                // Exponential decay using decay_rate
                // protection = max_protection * exp(-decay_rate * leverage)
                // Simplified approximation for on-chain
                let decay = (leverage * self.protection_curve_data.decay_rate) / 1_000_000;
                let protection = 10000_u64.saturating_sub(decay.min(10000));
                Ok(protection)
            }
            ProtectionCurveType::PIECEWISE => {
                // Interpolate between points
                let points = &self.protection_curve_data.points;
                for i in 0..7 {
                    if leverage <= points[i][0] {
                        if i == 0 {
                            return Ok(points[0][1]);
                        }
                        // Linear interpolation between points[i-1] and points[i]
                        let x0 = points[i-1][0];
                        let y0 = points[i-1][1];
                        let x1 = points[i][0];
                        let y1 = points[i][1];
                        
                        if x1 == x0 {
                            return Ok(y1);
                        }
                        
                        let interpolated = y0 + ((leverage - x0) * (y1 - y0)) / (x1 - x0);
                        return Ok(interpolated);
                    }
                }
                Ok(points[7][1])
            }
            _ => Err(FeelsProtocolError::InvalidProtectionCurve.into()),
        }
    }
    
    /// Check if leverage is within allowed bounds
    pub fn is_leverage_allowed(&self, leverage: u64) -> bool {
        leverage <= self.current_ceiling && leverage <= self.max_leverage
    }
}

// ============================================================================
// Duration Tracking for Time Dimension
// ============================================================================

/// Duration categories for position time commitments
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum DurationType {
    Flash,      // Single transaction
    Swap,       // Single block
    Daily,      // 1 day commitment
    Weekly,     // 7 day commitment
    Monthly,    // 30 day commitment
    Perpetual,  // No maturity
}

impl Default for DurationType {
    fn default() -> Self {
        DurationType::Perpetual
    }
}

/// Duration tracking for time-weighted operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Debug)]
pub struct Duration {
    /// Duration type
    pub duration_type: DurationType,
    
    /// Start slot
    pub start_slot: u64,
    
    /// End slot (0 for perpetual)
    pub end_slot: u64,
}

impl Duration {
    /// Check if duration has expired
    pub fn is_expired(&self, current_slot: u64) -> bool {
        self.end_slot > 0 && current_slot > self.end_slot
    }
    
    /// Get remaining slots
    pub fn remaining_slots(&self, current_slot: u64) -> u64 {
        if self.end_slot == 0 || current_slot >= self.end_slot {
            0
        } else {
            self.end_slot - current_slot
        }
    }
}

// ============================================================================
// Tick Position NFT Structure
// ============================================================================

/// **TickPositionMetadata - NFT-based Concentrated Liquidity Position**
/// 
/// Each position is represented as an NFT that tracks:
/// - Liquidity provided within a tick range
/// - Accumulated fees from in-range trading
/// - Leverage parameters for capital efficiency
/// - Time dimensions for the 3D model
/// 
/// The NFT representation enables:
/// - Transfer of positions between wallets
/// - Composability with other DeFi protocols
/// - Automated position management via programs
#[account]
pub struct TickPositionMetadata {
    // ========== Position Identification ==========
    
    /// Market this position belongs to
    pub market: Pubkey,
    
    /// Pool identifier (alias for market for compatibility)
    pub pool: Pubkey,
    
    /// NFT mint representing this position
    pub tick_position_mint: Pubkey,
    
    /// Current owner of the position
    pub owner: Pubkey,

    // ========== Range Definition ==========
    
    /// Lower tick boundary
    pub tick_lower: i32,
    
    /// Upper tick boundary
    pub tick_upper: i32,

    // ========== Liquidity Tracking ==========
    
    /// Base liquidity amount (before leverage)
    pub liquidity: u128,

    // ========== Fee Tracking ==========
    
    /// Fee growth inside position at last update (token 0)
    pub fee_growth_inside_last_0: [u64; 4], // u256 as [u64; 4]
    
    /// Fee growth inside position at last update (token 1)
    pub fee_growth_inside_last_1: [u64; 4], // u256 as [u64; 4]
    
    /// Uncollected fees owed (token 0)
    pub tokens_owed_0: u64,
    
    /// Uncollected fees owed (token 1)
    pub tokens_owed_1: u64,

    // ========== Leverage Support ==========
    
    /// Leverage multiplier (6 decimals: 1_000_000 = 1x, 3_000_000 = 3x)
    pub leverage: u64,
    
    /// Hash of risk profile parameters for verification
    pub risk_profile_hash: [u8; 8],
    
    // ========== Time Dimension ==========
    
    /// Time commitment for this position
    pub duration: Duration,
    
    /// Slot when position was created
    pub creation_slot: u64,
    
    /// Slot when position matures (0 for perpetual)
    pub maturity_slot: u64,

    // ========== Rebasing Support ==========
    
    /// Virtual rebasing checkpoint for fee calculations
    pub rebase_checkpoint: RebaseCheckpoint,

    // ========== Reserved Space ==========
    
    /// Reserved for future extensions
    pub _reserved: [u8; 31],
}

impl TickPositionMetadata {
    // Size constants for account allocation
    pub const SIZE: usize = 8 +     // discriminator
        32 * 4 +                     // pubkeys (market, pool, mint, owner)
        4 * 2 +                      // tick range
        16 +                         // liquidity
        32 * 2 + 8 * 2 +            // fee tracking
        8 + 8 +                      // leverage data
        1 + 8 + 8 +                  // duration enum + slots
        8 + 8 +                      // time tracking
        16 + 16 + 16 + 8 +          // rebase checkpoint
        31;                          // reserved
    
    /// Calculate hash for risk profile verification
    pub fn calculate_risk_profile_hash(leverage: u64, protection_factor: u64) -> [u8; 8] {
        use anchor_lang::solana_program::hash::hash;
        let data = [leverage.to_le_bytes(), protection_factor.to_le_bytes()].concat();
        let full_hash = hash(&data);
        let mut hash_bytes = [0u8; 8];
        hash_bytes.copy_from_slice(&full_hash.to_bytes()[..8]);
        hash_bytes
    }

    /// Check if position has leverage enabled
    pub fn is_leveraged(&self) -> bool {
        self.leverage > 1_000_000 // Greater than 1x
    }

    /// Get effective liquidity considering leverage
    pub fn effective_liquidity(&self) -> Result<u128> {
        if !self.is_leveraged() {
            return Ok(self.liquidity);
        }
        
        // Calculate leveraged liquidity safely
        safe::mul_div_u128(
            self.liquidity,
            self.leverage as u128,
            1_000_000 // Leverage decimals
        )
    }

    /// Check if position is in range
    pub fn is_in_range(&self, current_tick: i32) -> bool {
        current_tick >= self.tick_lower && current_tick < self.tick_upper
    }

    /// Check if position has matured
    pub fn has_matured(&self, current_slot: u64) -> bool {
        self.maturity_slot > 0 && current_slot >= self.maturity_slot
    }

    /// Calculate position value including uncollected fees
    pub fn total_value(&self, token_0_price: u64, token_1_price: u64) -> Result<u128> {
        let fee_value_0 = safe::mul_u64(self.tokens_owed_0, token_0_price)?;
        let fee_value_1 = safe::mul_u64(self.tokens_owed_1, token_1_price)?;
        
        // Note: Would also include liquidity value calculation
        // based on current tick and price
        Ok((fee_value_0 + fee_value_1) as u128)
    }
}

// ============================================================================
// Position Manager State
// ============================================================================

/// Global position tracking for a market
#[account]
pub struct PositionManager {
    /// Market this manages positions for
    pub market: Pubkey,
    
    /// Total positions created
    pub total_positions: u64,
    
    /// Total active positions
    pub active_positions: u64,
    
    /// Total liquidity across all positions
    pub total_liquidity: u128,
    
    /// Total leveraged liquidity
    pub total_leveraged_liquidity: u128,
    
    /// Leverage parameters for this market
    pub leverage_params: LeverageParameters,
    
    /// Fee tier for position creation
    pub position_creation_fee: u64,
    
    /// Authority for parameter updates
    pub authority: Pubkey,
    
    /// Reserved space
    pub _reserved: [u8; 128],
}

impl PositionManager {
    pub const LEN: usize = 8 +      // discriminator
        32 +                         // market
        8 + 8 +                      // position counts
        16 + 16 +                    // liquidity totals
        88 +                         // leverage params
        8 +                          // creation fee
        32 +                         // authority
        128;                         // reserved
    
    /// Update leverage ceiling based on market conditions
    pub fn update_leverage_ceiling(&mut self, new_ceiling: u64, current_time: u64) -> Result<()> {
        require!(
            new_ceiling <= self.leverage_params.max_leverage,
            FeelsProtocolError::LeverageTooHigh
        );
        
        self.leverage_params.current_ceiling = new_ceiling;
        self.leverage_params.last_ceiling_update = current_time;
        
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate fee growth inside a position's tick range
pub fn calculate_fee_growth_inside(
    tick_lower: i32,
    tick_upper: i32,
    current_tick: i32,
    fee_growth_global: [u64; 4],
    fee_growth_outside_lower: [u64; 4],
    fee_growth_outside_upper: [u64; 4],
) -> [u64; 4] {
    // Implementation would handle the complex fee growth calculation
    // based on whether current tick is inside/outside the range
    // This is a placeholder
    fee_growth_global
}

/// Validate position parameters
pub fn validate_position(
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    leverage: u64,
    leverage_params: &LeverageParameters,
) -> Result<()> {
    // Validate tick range
    require!(
        tick_lower < tick_upper,
        FeelsProtocolError::InvalidTickRange
    );
    
    // Validate liquidity
    require!(
        liquidity > 0,
        FeelsProtocolError::ZeroLiquidity
    );
    
    // Validate leverage
    if leverage > 1_000_000 {
        require!(
            leverage_params.is_leverage_allowed(leverage),
            FeelsProtocolError::LeverageTooHigh
        );
    }
    
    Ok(())
}