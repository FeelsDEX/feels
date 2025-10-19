//! Swap transaction data models

use super::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed swap transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSwap {
    pub signature: String,
    pub market: Pubkey,
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub fee_bps: u16,
    
    // Price and liquidity state
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
    
    // JIT liquidity interaction
    pub jit_liquidity_used: bool,
    pub jit_amount_filled: Option<u64>,
    
    // Block information
    pub block_info: BlockInfo,
    
    // Additional context
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
        // Assuming token_0 < token_1 in pubkey ordering
        // Buy = token_0 in, token_1 out
        self.token_in < self.token_out
    }

    /// Get trade size in USD equivalent (requires price feed)
    pub fn trade_size_usd(&self, token_price_usd: f64) -> f64 {
        self.amount_in as f64 * token_price_usd
    }
}

/// Aggregated swap statistics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapStats {
    pub market: Pubkey,
    pub period_start: i64,
    pub period_end: i64,
    
    // Volume metrics
    pub total_swaps: u32,
    pub volume_token_0: u128,
    pub volume_token_1: u128,
    pub total_fees: u128,
    
    // Price metrics
    pub avg_price: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub price_volatility: f64,
    
    // User metrics
    pub unique_users: u32,
    pub avg_trade_size: f64,
    pub median_trade_size: f64,
    
    // JIT metrics
    pub jit_fill_rate: f32,
    pub jit_volume_percentage: f32,
}

/// Swap route information for multi-hop swaps
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
