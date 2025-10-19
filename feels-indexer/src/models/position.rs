//! Position and liquidity data models

use super::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed liquidity position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedPosition {
    pub address: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    
    // Fee tracking
    pub fee_growth_inside_0_last: u128,
    pub fee_growth_inside_1_last: u128,
    pub fees_owed_0: u64,
    pub fees_owed_1: u64,
    
    // Position type
    pub position_type: PositionType,
    pub is_protocol_owned: bool,
    
    // Performance tracking
    pub total_fees_earned_0: u128,
    pub total_fees_earned_1: u128,
    pub impermanent_loss: f64,
    
    // Lifecycle
    pub created_at: BlockInfo,
    pub last_updated: BlockInfo,
    pub is_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionType {
    UserLP,
    FloorLiquidity,
    JitLiquidity,
    BondingCurve,
}

impl IndexedPosition {
    /// Calculate position value in token amounts
    pub fn calculate_amounts(&self, sqrt_price: u128) -> (u128, u128) {
        // Simplified calculation - would need full CLMM math in production
        let price = (sqrt_price as f64 / (1u128 << 64) as f64).powi(2);
        let tick_current = (price.ln() / 1.0001_f64.ln()) as i32;
        
        if tick_current < self.tick_lower {
            // All token 0
            (self.liquidity, 0)
        } else if tick_current >= self.tick_upper {
            // All token 1
            (0, self.liquidity)
        } else {
            // Mixed position - simplified calculation
            let ratio = (tick_current - self.tick_lower) as f64 / (self.tick_upper - self.tick_lower) as f64;
            let amount_1 = (self.liquidity as f64 * ratio) as u128;
            let amount_0 = self.liquidity - amount_1;
            (amount_0, amount_1)
        }
    }

    /// Calculate unclaimed fees
    pub fn unclaimed_fees(&self) -> (u64, u64) {
        (self.fees_owed_0, self.fees_owed_1)
    }

    /// Check if position is in range for current price
    pub fn is_in_range(&self, current_tick: i32) -> bool {
        current_tick >= self.tick_lower && current_tick < self.tick_upper
    }

    /// Calculate position width in ticks
    pub fn width_ticks(&self) -> u32 {
        (self.tick_upper - self.tick_lower) as u32
    }
}

/// Position lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionEvent {
    pub position: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub event_type: PositionEventType,
    pub liquidity_delta: i128,
    pub amount_0_delta: i128,
    pub amount_1_delta: i128,
    pub block_info: BlockInfo,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionEventType {
    Opened,
    IncreasedLiquidity,
    DecreasedLiquidity,
    CollectedFees,
    Closed,
}

/// Liquidity provider performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LPMetrics {
    pub owner: Pubkey,
    pub market: Option<Pubkey>, // None for aggregate across all markets
    pub period_start: i64,
    pub period_end: i64,
    
    // Position metrics
    pub active_positions: u32,
    pub total_liquidity_provided: u128,
    pub avg_position_size: f64,
    
    // Performance
    pub total_fees_earned_0: u128,
    pub total_fees_earned_1: u128,
    pub total_impermanent_loss: f64,
    pub net_pnl: f64,
    pub roi_percentage: f64,
    
    // Activity
    pub position_changes: u32,
    pub fee_collections: u32,
    pub avg_holding_period_hours: f64,
}
