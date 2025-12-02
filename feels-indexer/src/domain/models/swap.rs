//! Swap domain models

use crate::core::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed swap transaction - core domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSwap {
    // Identity
    pub signature: String,
    pub market: Pubkey,
    pub user: Pubkey,
    
    // Swap details
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub fee_bps: u16,
    
    // State changes
    pub sqrt_price_before: u128,
    pub sqrt_price_after: u128,
    pub tick_before: i32,
    pub tick_after: i32,
    pub liquidity_before: u128,
    pub liquidity_after: u128,
    
    // Derived metrics
    pub price_before: f64,
    pub price_after: f64,
    pub price_impact_bps: u16,
    pub effective_price: f64,
    
    // JIT liquidity
    pub jit_liquidity_used: bool,
    pub jit_amount_filled: Option<u64>,
    
    // Context
    pub block_info: BlockInfo,
    pub instruction_index: u8,
    pub inner_instruction_index: Option<u8>,
}

impl IndexedSwap {
    /// Calculate price impact in basis points
    pub fn calculate_price_impact(&self) -> u16 {
        if self.price_before == 0.0 {
            return 0;
        }
        
        let impact = ((self.price_after - self.price_before) / self.price_before).abs();
        (impact * 10000.0) as u16
    }

    /// Calculate effective price (amount_out / amount_in)
    pub fn calculate_effective_price(&self) -> f64 {
        if self.amount_in == 0 {
            return 0.0;
        }
        self.amount_out as f64 / self.amount_in as f64
    }

    /// Get swap direction (true = buy token_1, false = sell token_1)
    pub fn is_buy(&self) -> bool {
        self.token_in < self.token_out
    }
}

/// Swap route for multi-hop swaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub hops: Vec<SwapHop>,
    pub total_amount_in: u64,
    pub total_amount_out: u64,
    pub total_fee: u64,
    pub route_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHop {
    pub market: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub hop_index: u8,
}

