//! Floor liquidity data models

use super::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed floor state for a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFloor {
    pub market: Pubkey,
    pub current_floor_tick: i32,
    pub current_floor_price: f64,
    pub jitosol_reserves: u128,
    pub circulating_supply: u128,
    pub last_ratchet_slot: u64,
    pub floor_buffer: i32,
    pub last_updated: BlockInfo,
    
    // Historical tracking
    pub total_ratchets: u32,
    pub cumulative_appreciation: f64,
    pub initial_floor_price: f64,
    pub highest_floor_price: f64,
}

impl IndexedFloor {
    /// Calculate floor price from reserves and supply
    pub fn calculate_floor_price(reserves: u128, supply: u128) -> f64 {
        if supply == 0 {
            return 0.0;
        }
        reserves as f64 / supply as f64
    }

    /// Calculate appreciation since inception
    pub fn appreciation_percentage(&self) -> f64 {
        if self.initial_floor_price == 0.0 {
            return 0.0;
        }
        ((self.current_floor_price - self.initial_floor_price) / self.initial_floor_price) * 100.0
    }

    /// Check if floor can ratchet (based on cooldown)
    pub fn can_ratchet(&self, current_slot: u64, cooldown_slots: u64) -> bool {
        current_slot >= self.last_ratchet_slot + cooldown_slots
    }

    /// Get safe ask tick (floor + buffer)
    pub fn safe_ask_tick(&self) -> i32 {
        self.current_floor_tick + self.floor_buffer
    }
}

/// Floor ratchet event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorRatchet {
    pub market: Pubkey,
    pub old_floor_tick: i32,
    pub new_floor_tick: i32,
    pub old_floor_price: f64,
    pub new_floor_price: f64,
    pub reserves_at_ratchet: u128,
    pub supply_at_ratchet: u128,
    pub appreciation_bps: u16,
    pub block_info: BlockInfo,
    pub signature: String,
}

/// Floor liquidity position update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLiquidityUpdate {
    pub market: Pubkey,
    pub position_id: Pubkey,
    pub old_tick_lower: Option<i32>,
    pub old_tick_upper: Option<i32>,
    pub new_tick_lower: i32,
    pub new_tick_upper: i32,
    pub liquidity_delta: i128,
    pub amount_0_delta: i128,
    pub amount_1_delta: i128,
    pub block_info: BlockInfo,
    pub signature: String,
}

/// Floor performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorMetrics {
    pub market: Pubkey,
    pub period_start: i64,
    pub period_end: i64,
    
    // Ratchet metrics
    pub ratchet_count: u32,
    pub total_appreciation_bps: u32,
    pub avg_ratchet_size_bps: u32,
    pub max_ratchet_size_bps: u32,
    
    // Solvency metrics
    pub min_solvency_ratio: f64,
    pub avg_solvency_ratio: f64,
    pub max_solvency_ratio: f64,
    
    // Reserve growth
    pub reserve_growth_percentage: f64,
    pub supply_change_percentage: f64,
    
    // Performance vs market
    pub floor_vs_market_performance: f64,
}

/// Historical floor price point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorPricePoint {
    pub market: Pubkey,
    pub timestamp: i64,
    pub slot: u64,
    pub floor_tick: i32,
    pub floor_price: f64,
    pub reserves: u128,
    pub supply: u128,
    pub solvency_ratio: f64,
}
