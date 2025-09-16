//! Liquidity position state
//!
//! Represents an NFT-tokenized liquidity position in a market

use anchor_lang::prelude::*;

#[account]
pub struct Position {
    /// Position NFT mint (Metaplex Core asset ID)
    pub nft_mint: Pubkey,

    /// Market this position belongs to
    pub market: Pubkey,

    /// Owner of the position
    pub owner: Pubkey,

    /// Tick range
    pub tick_lower: i32,
    pub tick_upper: i32,

    /// Liquidity amount
    pub liquidity: u128,

    /// Fee growth inside the position at last update (Q64 fixed point)
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,

    /// Tokens owed to position (collected fees + removed liquidity)
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,

    /// Canonical bump for position PDA
    /// Storing prevents recomputation when minting/burning
    pub position_bump: u8,
    
    /// Whether this is a POMM position
    pub is_pomm: bool,
    
    /// Last slot this position was updated
    pub last_updated_slot: u64,

    /// Fee growth inside at last action (for proper accounting)
    pub fee_growth_inside_0_last: u128,
    pub fee_growth_inside_1_last: u128,
    
    /// Accumulated fees owed
    pub fees_owed_0: u64,
    pub fees_owed_1: u64,
}

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // nft_mint
        32 + // market
        32 + // owner
        4 + // tick_lower
        4 + // tick_upper
        16 + // liquidity
        16 + // fee_growth_inside_0_last_x64
        16 + // fee_growth_inside_1_last_x64
        8 + // tokens_owed_0
        8 + // tokens_owed_1
        1 + // position_bump
        1 + // is_pomm
        8 + // last_updated_slot
        16 + // fee_growth_inside_0_last
        16 + // fee_growth_inside_1_last
        8 + // fees_owed_0
        8 + // fees_owed_1
        6 + // padding for alignment
        8; // Additional Rust compiler padding

    pub const SIZE: usize = Self::LEN;
}