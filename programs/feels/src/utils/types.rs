/// Common parameter and result types for protocol operations.
/// These structs provide type safety and self-documenting interfaces for complex operations.
/// They are designed for future use when standardizing function signatures across the protocol.
// use anchor_lang::prelude::*; // Unused import
// FixedPoint types are placeholders - actual calculations done off-chain
// use crate::utils::{FixedPoint, FixedPointExt};

// ============================================================================
// 3D Market Physics Types
// ============================================================================

/// 3D position in market physics space (Rate, Duration, Leverage)
/// NOTE: Uses u128 for Q64 fixed-point values until FixedPoint type is implemented
#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
pub struct Position3D {
    pub S: u128,  // Spot/Rate coordinate (Q64 fixed-point)
    pub T: u128,  // Time/Duration coordinate (Q64 fixed-point)
    pub L: u128,  // Leverage coordinate (Q64 fixed-point)
}

impl Position3D {
    pub fn new(s: u128, t: u128, l: u128) -> Self {
        Self { S: s, T: t, L: l }
    }
    
    // TODO: Create from raw values using library-based FixedPoint
    // NOTE: Commented out until FixedPoint implementation is available
    /*
    pub fn from_values(s: f64, t: f64, l: f64) -> Self {
        Self {
            S: FixedPoint::from_num(s),
            T: FixedPoint::from_num(t),
            L: FixedPoint::from_num(l),
        }
    }
    */
}

// TODO: Basic position delta for simple calculations
// NOTE: Commented out until FixedPoint implementation is available
/*
#[derive(Debug, Clone, Copy)]
pub struct PositionDelta3D {
    pub dS: FixedPoint,
    pub dT: FixedPoint,
    pub dL: FixedPoint,
}
*/

/*
impl PositionDelta3D {
    pub fn between(start: &Position3D, end: &Position3D) -> Result<Self> {
        Ok(Self {
            dS: end.S.sub(start.S)?,
            dT: end.T.sub(start.T)?,
            dL: end.L.sub(start.L)?,
        })
    }
}

/// Trade dimension enum for categorization
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeDimension {
    Spot,
    Time,
    Leverage,
    Mixed,
}

/// 3D cell index for spatial partitioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellIndex3D {
    pub s: i32,
    pub t: i32,
    pub l: i32,
}

impl CellIndex3D {
    pub fn new(s: i32, t: i32, l: i32) -> Self {
        Self { s, t, l }
    }
}

// ============================================================================
// Trading Parameter Types
// ============================================================================

/// Swap parameters for executing trades
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapParams {
    pub amount_in: u64,
    pub amount_out_minimum: u64,
    pub sqrt_price_limit: u128,
    pub is_token_a_to_b: bool,
}

/// Results from a swap operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapResult {
    pub amount_out: u64,
    pub fee_amount: u64,
    pub new_sqrt_price: u128,
    pub new_tick: i32,
    pub new_liquidity: u128,
}

// ============================================================================
// Liquidity Operation Types
// ============================================================================

/// Liquidity operation parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LiquidityParams {
    pub liquidity_amount: u128,
    pub amount_0_max: u64,
    pub amount_1_max: u64,
    pub amount_0_min: u64,
    pub amount_1_min: u64,
}

/// Results from liquidity operations (add/remove)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LiquidityResult {
    pub amount_0: u64,
    pub amount_1: u64,
    pub liquidity_delta: i128,
    pub tick_lower: i32,
    pub tick_upper: i32,
}

// ============================================================================
// Fee Collection Types
// ============================================================================

/// Parameters for fee collection operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FeeCollectionParams {
    pub amount_a_requested: u64,
    pub amount_b_requested: u64,
    pub collect_all: bool,
}

/// Results from fee collection operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FeeCollectionResult {
    pub amount_a_collected: u64,
    pub amount_b_collected: u64,
    pub fees_remaining_a: u64,
    pub fees_remaining_b: u64,
}

// ============================================================================
// Event and Hook Types
// ============================================================================

/// Hook data for event emission
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct HookData {
    pub pre_swap_price: u128,
    pub post_swap_price: u128,
    pub price_impact_bps: u16,
    pub volume: u64,
}

/// Risk profile parameters for advanced operations (Phase 2+)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RiskProfile {
    pub max_slippage_bps: u16,
    pub max_price_impact_bps: u16,
    pub require_atomic: bool,
    pub timeout_seconds: u32,
}

/// Swap route information for multi-hop trades (Phase 2+)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapRoute {
    pub pools: Vec<Pubkey>,
    pub token_path: Vec<Pubkey>,
    pub fee_tiers: Vec<u16>,
    pub estimated_amount_out: u64,
}

// ============================================================================
// U256 Wrapper Type
// ============================================================================

use crate::utils::U256;
use crate::utils::math::big_int::{mul_div, Rounding};

/// Wrapper around U256 for fee growth calculations
#[derive(Clone, Copy, Debug)]
pub struct U256Wrapper(U256);

impl U256Wrapper {
    pub fn from_u64(value: u64) -> Self {
        U256Wrapper(U256::from(value))
    }

    pub fn from_u128(value: u128) -> Self {
        U256Wrapper(U256::from(value))
    }

    pub fn from_u64_array(array: [u64; 4]) -> Self {
        U256Wrapper(U256::from_limbs(array))
    }

    pub fn checked_add(&self, other: U256Wrapper) -> Option<U256Wrapper> {
        self.0.checked_add(other.0).map(U256Wrapper)
    }

    pub fn checked_mul(&self, other: U256Wrapper) -> Option<U256Wrapper> {
        self.0.checked_mul(other.0).map(U256Wrapper)
    }

    pub fn checked_div(&self, other: U256Wrapper) -> Option<U256Wrapper> {
        self.0.checked_div(other.0).map(U256Wrapper)
    }

    pub fn as_u64_array(&self) -> [u64; 4] {
        let bytes: [u8; 32] = self.0.to_le_bytes();
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            limbs[i] = u64::from_le_bytes([
                bytes[i * 8], bytes[i * 8 + 1], bytes[i * 8 + 2], bytes[i * 8 + 3],
                bytes[i * 8 + 4], bytes[i * 8 + 5], bytes[i * 8 + 6], bytes[i * 8 + 7],
            ]);
        }
        limbs
    }
    
    /// Multiply by numerator and divide by denominator using the consolidated math module
    pub fn mul_div(&self, numerator: U256Wrapper, denominator: U256Wrapper) -> Option<U256Wrapper> {
        mul_div(self.0, numerator.0, denominator.0, Rounding::Down)
            .map(U256Wrapper)
    }
    
    /// Multiply by numerator and divide by denominator, rounding up
    pub fn mul_div_round_up(&self, numerator: U256Wrapper, denominator: U256Wrapper) -> Option<U256Wrapper> {
        mul_div(self.0, numerator.0, denominator.0, Rounding::Up)
            .map(U256Wrapper)
    }
}

// ============================================================================
// Complex Operation Types
// ============================================================================

/// Complex operation parameters for advanced instructions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ComplexOperationParams {
    pub operation_type: u8,
    pub amount: u64,
    pub additional_data: [u8; 32],
}
*/
