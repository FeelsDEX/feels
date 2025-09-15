//! Event definitions

use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;

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
    pub base_fee_paid: u64,
    pub sqrt_price_after: u128,
    pub timestamp: i64,
    pub version: u8,
}

/// Event emitted with fee breakdown (MVP: base + impact, post-swap applied on output)
#[event]
pub struct FeeSplitApplied {
    pub market: Pubkey,
    pub base_fee_bps: u16,
    pub impact_fee_bps: u16,
    pub total_fee_bps: u16,
    pub fee_denom_mint: Pubkey, // Mint of the token the fee was taken in (output token)
    pub fee_amount: u64,        // Amount of fee taken in fee_denom_mint
    pub to_buffer_amount: u64,  // Amount routed to Buffer (τ)
    pub to_treasury_amount: u64, // Amount routed to Protocol Treasury
    pub to_creator_amount: u64, // Amount accrued to Creator (protocol tokens only)
    pub jit_consumed_quote: u64, // Quote units reserved by JIT v0 in this swap
    pub timestamp: i64,
}

/// Event emitted on protocol oracle update
#[event]
pub struct OracleUpdatedProtocol {
    pub native_q64: u128,
    pub dex_twap_q64: u128,
    pub min_rate_q64: u128,
    pub div_bps: u16,
    pub threshold_bps: u16,
    pub window_secs: u32,
    pub paused: bool,
    pub timestamp: i64,
}

#[event]
pub struct CircuitBreakerActivated {
    pub threshold_bps: u16,
    pub window_secs: u32,
}

#[event]
pub struct RedemptionsPaused {
    pub timestamp: i64,
}

#[event]
pub struct RedemptionsResumed {
    pub timestamp: i64,
}

#[event]
pub struct SafetyPaused {
    pub timestamp: i64,
}

#[event]
pub struct SafetyResumed {
    pub timestamp: i64,
}

/// Event emitted when a rate limit is triggered
#[event]
pub struct RateLimitTriggered {
    /// 0 = mint, 1 = redeem
    pub scope: u8,
    pub amount: u64,
    pub cap: u64,
    pub slot: u64,
    pub timestamp: i64,
}
/// Event emitted when protocol params are updated (snapshot)
#[event]
pub struct ProtocolParamsUpdated {
    pub authority: Pubkey,
    pub depeg_threshold_bps: u16,
    pub depeg_required_obs: u8,
    pub clear_required_obs: u8,
    pub dex_twap_window_secs: u32,
    pub dex_twap_stale_age_secs: u32,
    pub dex_twap_updater: Pubkey,
}

#[event]
pub struct FloorRatcheted {
    pub market: Pubkey,
    pub old_floor_tick: i32,
    pub new_floor_tick: i32,
    pub timestamp: i64,
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

/// Event emitted when an expired token is destroyed
#[event]
pub struct TokenDestroyed {
    pub token_mint: Pubkey,
    pub destroyer: Pubkey,
    pub created_at: i64,
    pub destroyed_at: i64,
    pub mint_fee_returned: u64,
    pub destroyer_reward: u64,
    pub treasury_amount: u64,
}
