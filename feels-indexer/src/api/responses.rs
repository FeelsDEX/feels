//! API response types

use crate::database::{Market, Swap, Position};
use crate::models::{IndexedFloor, MarketStats};
use serde::{Deserialize, Serialize};

/// Response for markets list
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketsResponse {
    pub markets: Vec<Market>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Response for single market
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketResponse {
    pub market: Market,
}

/// Response for market statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketStatsResponse {
    pub market_address: String,
    pub volume_24h: f64,
    pub fees_24h: f64,
    pub swaps_24h: u64,
    pub unique_traders_24h: u64,
    pub price_change_24h: f64,
    pub liquidity_change_24h: f64,
    pub timestamp: i64,
}

/// Response for swaps list
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapsResponse {
    pub swaps: Vec<Swap>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Response for single swap
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap: Swap,
}

/// Response for positions list
#[derive(Debug, Serialize, Deserialize)]
pub struct PositionsResponse {
    pub positions: Vec<Position>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Response for single position
#[derive(Debug, Serialize, Deserialize)]
pub struct PositionResponse {
    pub position: Position,
}

/// Response for floor information
#[derive(Debug, Serialize, Deserialize)]
pub struct FloorResponse {
    pub market_address: String,
    pub current_floor_tick: i32,
    pub current_floor_price: f64,
    pub jitosol_reserves: String,
    pub circulating_supply: String,
    pub last_update_slot: u64,
    pub timestamp: i64,
}

/// Response for OHLCV data
#[derive(Debug, Serialize, Deserialize)]
pub struct OHLCVResponse {
    pub market_address: String,
    pub candles: Vec<OHLCVCandle>,
    pub interval: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OHLCVCandle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Response for protocol statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolStatsResponse {
    pub total_markets: u64,
    pub total_volume_24h: f64,
    pub total_fees_24h: f64,
    pub total_liquidity: f64,
    pub active_traders_24h: u64,
    pub timestamp: i64,
}

/// Response for volume data
#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub daily_volumes: Vec<DailyVolume>,
    pub total_volume: f64,
    pub total_fees: f64,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyVolume {
    pub date: String,
    pub volume: f64,
    pub fees: f64,
    pub swap_count: u64,
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
    pub timestamp: i64,
}
