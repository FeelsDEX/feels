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
    // Size breakdown for clarity and maintainability
    const DISCRIMINATOR_SIZE: usize = 8;
    const IDENTIFICATION_SIZE: usize = 32 * 3;  // pool, tick_position_mint, owner
    const RANGE_SIZE: usize = 4 * 2;  // tick_lower, tick_upper
    const LIQUIDITY_SIZE: usize = 16;  // liquidity (u128)
    const FEE_TRACKING_SIZE: usize = 32 * 2 + 8 * 2;  // fee_growth_inside_last + tokens_owed
    const RESERVED_SIZE: usize = 64;  // reserved for future upgrades
    
    pub const SIZE: usize = Self::DISCRIMINATOR_SIZE +
        Self::IDENTIFICATION_SIZE +
        Self::RANGE_SIZE +
        Self::LIQUIDITY_SIZE +
        Self::FEE_TRACKING_SIZE +
        Self::RESERVED_SIZE;  // Total: 246 bytes
}

// Parameter and result types have been moved to utils::types for better organization
// Import them as needed: use crate::utils::types::{SwapParams, SwapResult, LiquidityParams, etc.}