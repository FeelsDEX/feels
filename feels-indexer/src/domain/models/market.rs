//! Market domain models

use crate::core::{BlockInfo, types::UpdateType};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Pool phase enum matching the protocol
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PoolPhase {
    PriceDiscovery,
    SteadyState,
}

/// Indexed market state - core domain model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexedMarket {
    // Identity
    pub address: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    
    // Core state
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub current_tick: i32,
    pub tick_spacing: u16,
    pub fee_bps: u16,
    pub is_paused: bool,
    pub phase: PoolPhase,
    
    // Tick bounds
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    
    // Fee tracking
    pub fee_growth_global_0: u128,
    pub fee_growth_global_1: u128,
    
    // Metadata
    pub last_updated: BlockInfo,
    
    // Analytics (denormalized for performance)
    pub total_volume_0: u128,
    pub total_volume_1: u128,
    pub total_fees_0: u128,
    pub total_fees_1: u128,
    pub swap_count: u64,
    pub unique_traders: u64,
}

impl IndexedMarket {
    /// Calculate current price from sqrt_price
    pub fn current_price(&self) -> f64 {
        let sqrt_price = self.sqrt_price as f64;
        (sqrt_price / (1u128 << 64) as f64).powi(2)
    }

    /// Calculate price from tick
    pub fn tick_to_price(tick: i32) -> f64 {
        1.0001_f64.powi(tick)
    }

    /// Calculate tick from price
    pub fn price_to_tick(price: f64) -> i32 {
        (price.ln() / 1.0001_f64.ln()).round() as i32
    }

    /// Get market identifier string
    pub fn market_id(&self) -> String {
        format!("{}_{}", self.token_0, self.token_1)
    }
    
    /// Check if market is in steady state
    pub fn is_steady_state(&self) -> bool {
        self.phase == PoolPhase::SteadyState
    }
}

/// Market statistics for analytics queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStats {
    pub market: Pubkey,
    pub period_start: i64,
    pub period_end: i64,
    
    // OHLCV data
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume_0: u128,
    pub volume_1: u128,
    
    // Additional metrics
    pub swap_count: u32,
    pub unique_traders: u32,
    pub avg_trade_size_0: f64,
    pub avg_trade_size_1: f64,
    pub price_change_percent: f64,
    pub volatility: f64,
}

/// Market creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCreated {
    pub market: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub creator: Pubkey,
    pub initial_sqrt_price: u128,
    pub tick_spacing: u16,
    pub fee_bps: u16,
    pub block_info: BlockInfo,
    pub signature: String,
}

/// Market update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    pub market: Pubkey,
    pub update_type: UpdateType,
    pub block_info: BlockInfo,
    pub signature: Option<String>,
}

