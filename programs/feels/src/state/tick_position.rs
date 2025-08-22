/// Represents individual liquidity positions as NFTs with concentrated liquidity metadata.
/// Tracks position boundaries (tick range), liquidity amount, accumulated fees, and ownership.
/// Each position earns fees proportional to its share of in-range liquidity during swaps.
/// NFT representation enables positions to be transferred, composed, and integrated with DeFi.
use anchor_lang::prelude::*;

// ============================================================================
// Tick Position NFT Structure
// ============================================================================

#[account]
pub struct TickPositionMetadata {
    // Tick Position identification
    pub pool: Pubkey,
    pub tick_position_mint: Pubkey,
    pub owner: Pubkey,
    
    // Range definition
    pub tick_lower: i32,
    pub tick_upper: i32,
    
    // Liquidity tracking
    pub liquidity: u128,
    
    // Fee tracking (using [u64; 4] to represent u256)
    pub fee_growth_inside_last_0: [u64; 4],
    pub fee_growth_inside_last_1: [u64; 4],
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
    
    // Reserved for future extensions
    pub _reserved: [u8; 64],
}

impl TickPositionMetadata {
    pub const SIZE: usize = 8 + // discriminator
        32 * 3 +                // pool, tick_position_mint, owner
        4 * 2 +                 // tick_lower, tick_upper
        16 +                    // liquidity
        32 * 2 +                // fee_growth_inside_last
        8 * 2 +                 // tokens_owed
        64;                     // reserved
}

// ============================================================================
// Trading Parameter Types
// ============================================================================

/// Swap parameters for executing trades
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapParams {
    pub amount_in: u64,
    pub amount_out_minimum: u64,
    pub sqrt_price_limit: u128,
    pub is_token_0_to_1: bool, // Updated to use 0/1 naming
}

/// Results from a swap operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapResult {
    pub amount_out: u64,
    pub fee_amount: u64,
    pub new_sqrt_price: u128,
    pub new_tick: i32,
    pub new_liquidity: u128,
}

// ============================================================================
// Event and Hook Types
// ============================================================================

/// Hook data for event emission
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct HookData {
    pub pre_swap_price: u128,
    pub post_swap_price: u128,
    pub price_impact_bps: u16,
    pub volume: u64,
}

// ============================================================================
// Liquidity Operation Types
// ============================================================================

/// Liquidity operation parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct LiquidityParams {
    pub liquidity_amount: u128,
    pub amount_0_max: u64,
    pub amount_1_max: u64,
    pub amount_0_min: u64,
    pub amount_1_min: u64,
}