//! Type definitions for SDK

use anchor_lang::prelude::*;

// InitializeMarketParams is now defined in instructions.rs

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

impl Route {
    /// Get the number of hops
    pub fn hop_count(&self) -> u8 {
        match self {
            Route::Direct { .. } => 1,
            Route::TwoHop { .. } => 2,
        }
    }
}

/// Swap result
#[derive(Clone, Debug)]
pub struct SwapQuote {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub fee_bps: u16,
    pub price_impact_bps: u16,
    pub route: Route,
}

/// Market info
#[derive(Clone, Debug)]
pub struct MarketInfo {
    pub address: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub base_fee_bps: u16,
    pub is_paused: bool,
}

/// Buffer info
#[derive(Clone, Debug)]
pub struct BufferInfo {
    pub address: Pubkey,
    pub market: Pubkey,
    pub tau_spot: u128,
    pub tau_time: u128,
    pub tau_leverage: u128,
    pub fees_token_0: u128,
    pub fees_token_1: u128,
    pub floor_threshold: u64,
}

/// Fee domain for attribution
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FeeDomain {
    Spot,
    Time,
    Leverage,
}
