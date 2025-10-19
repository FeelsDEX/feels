//! Buffer (fee accumulation) data models

use super::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed buffer state for a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBuffer {
    pub address: Pubkey,
    pub market: Pubkey,
    
    // τ (tau) values for different dimensions
    pub tau_spot: u128,
    pub tau_time: u128,
    pub tau_leverage: u128,
    
    // Fee accumulation
    pub fees_token_0: u128,
    pub fees_token_1: u128,
    
    // Floor management
    pub floor_threshold: u64,
    
    // JIT liquidity funding
    pub jit_budget_used: u128,
    pub jit_budget_remaining: u128,
    
    // Performance tracking
    pub total_fees_collected: u128,
    pub total_jit_revenue: u128,
    pub total_floor_allocations: u128,
    
    pub last_updated: BlockInfo,
}

impl IndexedBuffer {
    /// Calculate total tau across all dimensions
    pub fn total_tau(&self) -> u128 {
        self.tau_spot + self.tau_time + self.tau_leverage
    }

    /// Calculate total fees across both tokens
    pub fn total_fees(&self) -> u128 {
        self.fees_token_0 + self.fees_token_1
    }

    /// Calculate JIT budget utilization percentage
    pub fn jit_utilization_percentage(&self) -> f64 {
        let total_budget = self.jit_budget_used + self.jit_budget_remaining;
        if total_budget == 0 {
            return 0.0;
        }
        (self.jit_budget_used as f64 / total_budget as f64) * 100.0
    }

    /// Check if buffer is healthy (has sufficient reserves)
    pub fn is_healthy(&self, min_threshold: u128) -> bool {
        self.total_tau() >= min_threshold
    }
}

/// Buffer fee allocation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferFeeAllocation {
    pub buffer: Pubkey,
    pub market: Pubkey,
    pub allocation_type: FeeAllocationType,
    pub amount_token_0: u64,
    pub amount_token_1: u64,
    pub tau_dimension: TauDimension,
    pub block_info: BlockInfo,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeeAllocationType {
    SwapFee,
    JitRevenue,
    FloorAllocation,
    CreatorBonus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TauDimension {
    Spot,
    Time,
    Leverage,
}

/// Buffer performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferMetrics {
    pub buffer: Pubkey,
    pub market: Pubkey,
    pub period_start: i64,
    pub period_end: i64,
    
    // Fee collection
    pub total_fees_collected: u128,
    pub avg_fee_per_swap: f64,
    pub fee_growth_rate: f64,
    
    // JIT performance
    pub jit_revenue: u128,
    pub jit_utilization_avg: f64,
    pub jit_roi_percentage: f64,
    
    // Allocations
    pub floor_allocations: u128,
    pub creator_bonuses: u128,
    pub allocation_efficiency: f64,
    
    // Health metrics
    pub min_tau_level: u128,
    pub avg_tau_level: u128,
    pub max_tau_level: u128,
    pub health_score: f64,
}
