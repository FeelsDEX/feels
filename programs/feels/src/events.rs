//! Event definitions

use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;

/// Event emitted when a swap is executed
#[event]
pub struct SwapExecuted {
    pub market: Pubkey,
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_paid: u64,
    pub sqrt_price_after: u128,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when floor liquidity is placed
#[event]
pub struct FloorLiquidityPlaced {
    pub market: Pubkey,
    pub buffer: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_added: u128,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when epoch is bumped
#[event]
pub struct EpochBumped {
    pub market: Pubkey,
    pub old_epoch: u64,
    pub new_epoch: u64,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when FeelsSOL is minted (entry)
#[event]
pub struct FeelsSOLMinted {
    pub user: Pubkey,
    pub jitosol_amount: u64,
    pub feelssol_amount: u64,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when FeelsSOL is burned (exit)
#[event]
pub struct FeelsSOLBurned {
    pub user: Pubkey,
    pub feelssol_amount: u64,
    pub jitosol_amount: u64,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when a market is initialized
#[event]
pub struct MarketInitialized {
    pub market: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub feelssol_mint: Pubkey,
    pub buffer: Pubkey,
    pub base_fee_bps: u16,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when buffer is initialized
#[event]
pub struct BufferInitialized {
    pub buffer: Pubkey,
    pub market: Pubkey,
    pub floor_threshold: u64,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted when liquidity is added (generic path)
#[event]
pub struct LiquidityAdded {
    pub market: Pubkey,
    pub provider: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
    pub liquidity_minted: u128,
    pub sqrt_price: u128,
    pub timestamp: i64,
}

/// Event emitted when liquidity is removed (generic path)
#[event]
pub struct LiquidityRemoved {
    pub market: Pubkey,
    pub provider: Pubkey,
    pub liquidity_removed: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub sqrt_price: u128,
    pub timestamp: i64,
}


/// Event emitted when a new token is minted
#[event]
pub struct TokenMinted {
    pub token_mint: Pubkey,
    pub creator: Pubkey,
    pub ticker: String,
    pub name: String,
    pub total_supply: u64,
    pub buffer_amount: u64,
    pub creator_amount: u64,
    pub buffer_account: Pubkey,
    pub timestamp: i64,
}

/// Event emitted when a token is launched with bonding curve
#[event]  
pub struct TokenLaunched {
    pub market: Pubkey,
    pub buffer: Pubkey,
    pub launcher: Pubkey,
    pub token_mint: Pubkey,
    pub total_tokens_deployed: u64,
    pub total_feelssol_deployed: u64,
    pub num_tranches: u8,
    pub initial_price: u64,
    pub timestamp: i64,
}

/// Event emitted when a position NFT is minted
#[event]
pub struct PositionMinted {
    pub position_nft: Pubkey,
    pub position_account: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub timestamp: i64,
}

/// Event emitted when a position NFT is burned
#[event]
pub struct PositionBurned {
    pub position_nft: Pubkey,
    pub position_account: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}

/// Event emitted when oracle is observed
#[event]
pub struct OracleObserved {
    pub market: Pubkey,
    pub seconds_ago: u32,
    pub timestamp: i64,
    pub tick_cumulative: i128,
    pub liquidity_cumulative: u128,
}

/// Unified event for all position updates (open, close, collect fees)
#[event]
pub struct PositionUpdated {
    pub position: Pubkey,
    pub position_mint: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub fees_collected_0: u64,
    pub fees_collected_1: u64,
    pub operation: PositionOperation,
    pub timestamp: i64,
}

/// Type of position operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum PositionOperation {
    Open,
    Close,
    CollectFees,
    AddLiquidity,
    RemoveLiquidity,
}

/// Type of market operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum MarketOperation {
    Initialize,
    Pause,
    Unpause,
    UpdateFees,
}


