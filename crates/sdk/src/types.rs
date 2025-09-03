use anchor_lang::prelude::*;
use solana_sdk::signature::Signature;

/// Result of a pool creation operation
#[derive(Debug, Clone)]
pub struct PoolCreationResult {
    pub pool_pubkey: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub signature: Signature,
}

/// Result of a pool creation operation (alias)
pub type CreatePoolResult = PoolCreationResult;

/// Result of a liquidity addition operation (alias)
pub type AddLiquidityResult = LiquidityResult;

/// Result of a liquidity operation
#[derive(Debug, Clone)]
pub struct LiquidityResult {
    pub position_pubkey: Pubkey,
    pub position_mint: Pubkey,
    pub liquidity_amount: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub signature: Signature,
}

/// Result of a swap operation
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_after: u128,
    pub signature: Signature,
}

/// Position information
#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub owner: Pubkey,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_growth_0_checkpoint: u128,
    pub fee_growth_1_checkpoint: u128,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

/// Pool information
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pubkey: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub current_tick: i32,
    pub tick_spacing: i32,
}

/// Token account information
#[derive(Debug, Clone)]
pub struct TokenAccountInfo {
    pub pubkey: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
}
