//! API response types

use crate::database::{Market, Swap, Position};
use crate::models::{IndexedFloor, MarketStats};
use serde::{Deserialize, Serialize};
use rust_decimal::prelude::ToPrimitive;

// Frontend-compatible response types

/// Market format expected by frontend (IndexedMarket)
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedMarket {
    pub address: String,
    pub token_0: String,
    pub token_1: String,
    pub sqrt_price: String,
    pub liquidity: String,
    pub current_tick: i32,
    pub tick_spacing: i32,
    pub fee_bps: i32,
    pub is_paused: bool,
    pub phase: String,
    pub last_updated_slot: i64,
    pub last_updated_timestamp: i64,
}

/// Swap format expected by frontend (IndexedSwap)
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedSwap {
    pub signature: String,
    pub slot: i64,
    pub timestamp: i64,
    pub market: String,
    pub user: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: i64,
    pub amount_out: i64,
    pub fee_amount: i64,
    pub price_before: f64,
    pub price_after: f64,
    pub liquidity_before: String,
    pub liquidity_after: String,
}

impl From<Market> for IndexedMarket {
    fn from(market: Market) -> Self {
        IndexedMarket {
            address: market.address,
            token_0: market.token_0,
            token_1: market.token_1,
            sqrt_price: market.sqrt_price.to_string(),
            liquidity: market.liquidity.to_string(),
            current_tick: market.current_tick,
            tick_spacing: market.tick_spacing as i32,
            fee_bps: market.fee_bps as i32,
            is_paused: market.is_paused,
            phase: market.phase,
            last_updated_slot: market.last_updated_slot,
            // Convert updated_at timestamp to Unix timestamp
            last_updated_timestamp: market.updated_at.timestamp(),
        }
    }
}

impl From<Swap> for IndexedSwap {
    fn from(swap: Swap) -> Self {
        IndexedSwap {
            signature: swap.signature,
            slot: swap.slot,
            timestamp: swap.timestamp.timestamp(),
            market: swap.market_id.to_string(), // Convert UUID to string
            user: swap.trader,
            token_in: swap.token_in,
            token_out: swap.token_out,
            amount_in: swap.amount_in,
            amount_out: swap.amount_out,
            fee_amount: swap.fee_amount,
            price_before: swap.sqrt_price_before.to_f64().unwrap_or(0.0),
            price_after: swap.sqrt_price_after.to_f64().unwrap_or(0.0),
            liquidity_before: swap.liquidity.to_string(),
            liquidity_after: swap.liquidity.to_string(), // We don't track liquidity_after separately
        }
    }
}

/// Response for markets list
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketsResponse {
    pub markets: Vec<IndexedMarket>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Response for single market
#[derive(Debug, Serialize, Deserialize)]
pub struct MarketResponse {
    pub market: IndexedMarket,
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
    pub swaps: Vec<IndexedSwap>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Response for single swap
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap: IndexedSwap,
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
