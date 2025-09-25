use crate::prelude::*;

/// Route type for swaps
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    /// Direct swap with FeelsSOL (1 hop)
    Direct { from: Pubkey, to: Pubkey },
    /// Two-hop swap through FeelsSOL hub
    TwoHop {
        from: Pubkey,
        intermediate: Pubkey,
        to: Pubkey,
    },
}

/// Swap direction for concentrated liquidity
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwapDirection {
    /// Swapping token 0 for token 1 (price decreases)
    ZeroForOne,
    /// Swapping token 1 for token 0 (price increases)
    OneForZero,
}

/// Market state information
#[derive(Clone, Debug)]
pub struct MarketInfo {
    pub address: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub current_tick: i32,
    pub base_fee_bps: u16,
    pub tick_spacing: u16,
    pub is_paused: bool,
}

/// Position information
#[derive(Clone, Debug)]
pub struct PositionInfo {
    pub owner: Pubkey,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_growth_inside_0: u128,
    pub fee_growth_inside_1: u128,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

/// Fee estimate for a swap
#[derive(Clone, Debug)]
pub struct FeeEstimate {
    pub base_fee: u64,
    pub impact_fee: u64,
    pub total_fee: u64,
    pub fee_bps: u16,
    pub price_impact_bps: u16,
}

/// Swap simulation result
#[derive(Clone, Debug)]
pub struct SwapSimulation {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_paid: u64,
    pub end_sqrt_price: u128,
    pub end_tick: i32,
    pub ticks_crossed: u8,
}