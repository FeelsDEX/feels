/// Common parameter and result types for protocol operations.
/// These structs provide type safety and self-documenting interfaces for complex operations.
/// They are designed for future use when standardizing function signatures across the protocol.
use anchor_lang::prelude::*;

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
    pub amount_a_max: u64,
    pub amount_b_max: u64,
    pub amount_a_min: u64,
    pub amount_b_min: u64,
}

/// Results from liquidity operations (add/remove)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LiquidityResult {
    pub amount_a: u64,
    pub amount_b: u64,
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
        U256Wrapper(U256(array))
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
        self.0.0
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
