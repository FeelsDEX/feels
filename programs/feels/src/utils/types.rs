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
    pub is_token_0_to_1: bool,
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
    pub amount_0_requested: u64,
    pub amount_1_requested: u64,
    pub collect_all: bool,
}

/// Results from fee collection operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct FeeCollectionResult {
    pub amount_0_collected: u64,
    pub amount_1_collected: u64,
    pub fees_remaining_0: u64,
    pub fees_remaining_1: u64,
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