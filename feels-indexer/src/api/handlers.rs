//! API request handlers

use super::{ApiState, responses::*};
use crate::database::Market;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Query parameters for pagination
#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query parameters for time range
#[derive(Deserialize)]
pub struct TimeRangeQuery {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

/// List all markets
pub async fn list_markets(
    State(state): State<ApiState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<MarketsResponse>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get markets from PostgreSQL
    let markets = state.db_manager.postgres
        .get_markets_paginated(limit, offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get markets: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let total = state.db_manager.postgres
        .get_markets_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    // Convert to frontend format
    let indexed_markets: Vec<IndexedMarket> = markets.into_iter()
        .map(|m| m.into())
        .collect();
    
    Ok(Json(MarketsResponse {
        markets: indexed_markets,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get specific market
pub async fn get_market(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> Result<Json<MarketResponse>, StatusCode> {
    // Validate address
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Try Redis cache first
    let cache_key = format!("market:{}", address);
    if let Ok(Some(market)) = state.db_manager.redis
        .get_json::<Market>(&cache_key)
        .await {
        return Ok(Json(MarketResponse { market: market.into() }));
    }
    
    // Fallback to PostgreSQL
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get market: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    match market {
        Some(market) => Ok(Json(MarketResponse { market: market.into() })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get market statistics
pub async fn get_market_stats(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(time_range): Query<TimeRangeQuery>,
) -> Result<Json<MarketStatsResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Get market to validate it exists
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let market = match market {
        Some(m) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Get time range (default to 24h)
    let end_time = time_range.end_time.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let start_time = time_range.start_time.unwrap_or(end_time - 86400); // 24 hours ago
    
    // Get market stats from PostgreSQL
    let stats = state.db_manager.postgres
        .get_market_stats(market.id, start_time, end_time)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    use rust_decimal::prelude::ToPrimitive;
    
    Ok(Json(MarketStatsResponse {
        market_address: address,
        volume_24h: stats.volume_24h.to_f64().unwrap_or(0.0),
        fees_24h: stats.fees_24h.to_f64().unwrap_or(0.0),
        swaps_24h: stats.swaps_24h as u64,
        unique_traders_24h: stats.unique_traders_24h as u64,
        price_change_24h: stats.price_change_24h.to_f64().unwrap_or(0.0),
        liquidity_change_24h: stats.liquidity_change_24h.to_f64().unwrap_or(0.0),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Get market swaps
pub async fn get_market_swaps(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<SwapsResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get market by address first to get its ID
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let market = match market {
        Some(m) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Get swaps for this market
    let swaps = state.db_manager.postgres
        .get_swaps_by_market_id(market.id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_swaps_count_by_market_id(market.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    // Convert to frontend format
    let indexed_swaps: Vec<IndexedSwap> = swaps.into_iter()
        .map(|s| s.into())
        .collect();
    
    Ok(Json(SwapsResponse {
        swaps: indexed_swaps,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get market positions
pub async fn get_market_positions(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PositionsResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get market by address first to get its ID
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let market = match market {
        Some(m) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Get positions for this market
    let positions = state.db_manager.postgres
        .get_positions_by_market_id(market.id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_positions_count_by_market_id(market.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    Ok(Json(PositionsResponse {
        positions,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get market floor information
pub async fn get_market_floor(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> Result<Json<FloorResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Get market to validate it exists
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let market = match market {
        Some(m) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Get floor data from PostgreSQL
    let floor = state.db_manager.postgres
        .get_market_floor(market.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    use rust_decimal::prelude::ToPrimitive;
    
    Ok(Json(FloorResponse {
        market_address: address,
        current_floor_tick: floor.floor_tick,
        current_floor_price: floor.floor_price.to_f64().unwrap_or(0.0),
        jitosol_reserves: floor.jitosol_reserves.to_string(),
        circulating_supply: floor.circulating_supply.to_string(),
        last_update_slot: floor.last_update_slot as u64,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Get market OHLCV data
pub async fn get_market_ohlcv(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(time_range): Query<TimeRangeQuery>,
) -> Result<Json<OHLCVResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Get market to validate it exists
    let market = state.db_manager.postgres
        .get_market_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let market = match market {
        Some(m) => m,
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Get time range (default to 24h)
    let end_time = time_range.end_time.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let start_time = time_range.start_time.unwrap_or(end_time - 86400); // 24 hours ago
    
    // Get OHLCV data from PostgreSQL
    let candles = state.db_manager.postgres
        .get_market_ohlcv(market.id, start_time, end_time, "1h")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    use rust_decimal::prelude::ToPrimitive;
    
    let ohlcv_candles = candles.into_iter()
        .map(|c| OHLCVCandle {
            timestamp: c.timestamp,
            open: c.open.to_f64().unwrap_or(0.0),
            high: c.high.to_f64().unwrap_or(0.0),
            low: c.low.to_f64().unwrap_or(0.0),
            close: c.close.to_f64().unwrap_or(0.0),
            volume: c.volume.to_f64().unwrap_or(0.0),
        })
        .collect();
    
    Ok(Json(OHLCVResponse {
        market_address: address,
        candles: ohlcv_candles,
        interval: "1h".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// List swaps
pub async fn list_swaps(
    State(state): State<ApiState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<SwapsResponse>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get recent swaps
    let swaps = state.db_manager.postgres
        .get_recent_swaps_paginated(limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_swaps_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    // Convert to frontend format
    let indexed_swaps: Vec<IndexedSwap> = swaps.into_iter()
        .map(|s| s.into())
        .collect();
    
    Ok(Json(SwapsResponse {
        swaps: indexed_swaps,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get specific swap
pub async fn get_swap(
    State(state): State<ApiState>,
    Path(signature): Path<String>,
) -> Result<Json<SwapResponse>, StatusCode> {
    // Get swap by signature
    let swap = state.db_manager.postgres
        .get_swap_by_signature(&signature)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match swap {
        Some(swap) => Ok(Json(SwapResponse { swap: swap.into() })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get user swaps
pub async fn get_user_swaps(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<SwapsResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get swaps for this user
    let swaps = state.db_manager.postgres
        .get_swaps_by_user(&address, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_swaps_count_by_user(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    // Convert to frontend format
    let indexed_swaps: Vec<IndexedSwap> = swaps.into_iter()
        .map(|s| s.into())
        .collect();
    
    Ok(Json(SwapsResponse {
        swaps: indexed_swaps,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// List positions
pub async fn list_positions(
    State(state): State<ApiState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PositionsResponse>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get all positions
    let positions = state.db_manager.postgres
        .get_positions_paginated(limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_positions_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    Ok(Json(PositionsResponse {
        positions,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get specific position
pub async fn get_position(
    State(state): State<ApiState>,
    Path(address): Path<String>,
) -> Result<Json<PositionResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Get position by address
    let position = state.db_manager.postgres
        .get_position_by_address(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match position {
        Some(position) => Ok(Json(PositionResponse { position })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get user positions
pub async fn get_user_positions(
    State(state): State<ApiState>,
    Path(address): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PositionsResponse>, StatusCode> {
    let _pubkey = Pubkey::from_str(&address)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let limit = pagination.limit.unwrap_or(50).min(100) as i64;
    let offset = pagination.offset.unwrap_or(0) as i64;
    
    // Get positions for this user
    let positions = state.db_manager.postgres
        .get_positions_by_user(&address, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = state.db_manager.postgres
        .get_positions_count_by_user(&address)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as usize;
    
    Ok(Json(PositionsResponse {
        positions,
        total,
        limit: limit as usize,
        offset: offset as usize,
    }))
}

/// Get protocol statistics
pub async fn get_protocol_stats(
    State(state): State<ApiState>,
) -> Result<Json<ProtocolStatsResponse>, StatusCode> {
    // Get stats from PostgreSQL
    let total_markets = state.db_manager.postgres
        .get_markets_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as u64;
    
    let stats_24h = state.db_manager.postgres
        .get_protocol_stats_24h()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    use rust_decimal::prelude::ToPrimitive;
    
    Ok(Json(ProtocolStatsResponse {
        total_markets,
        total_volume_24h: stats_24h.total_volume_24h.to_f64().unwrap_or(0.0),
        total_fees_24h: stats_24h.total_fees_24h.to_f64().unwrap_or(0.0),
        total_liquidity: stats_24h.total_liquidity.to_f64().unwrap_or(0.0),
        active_traders_24h: stats_24h.active_traders_24h as u64,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}

/// Get protocol markets
pub async fn get_protocol_markets(
    State(state): State<ApiState>,
) -> Result<Json<MarketsResponse>, StatusCode> {
    // Reuse list_markets logic
    list_markets(State(state), Query(PaginationQuery { limit: None, offset: None })).await
}

/// Get protocol volume
pub async fn get_protocol_volume(
    State(state): State<ApiState>,
    Query(time_range): Query<TimeRangeQuery>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    // Get time range (default to 30 days)
    let end_time = time_range.end_time.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let start_time = time_range.start_time.unwrap_or(end_time - 2592000); // 30 days ago
    
    // Get volume data from PostgreSQL
    let volume_data = state.db_manager.postgres
        .get_protocol_volume_history(start_time, end_time)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    use rust_decimal::prelude::ToPrimitive;
    use chrono::{DateTime, Utc};
    
    let daily_volumes: Vec<DailyVolume> = volume_data.into_iter()
        .map(|v| DailyVolume {
            date: DateTime::<Utc>::from_timestamp(v.date, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            volume: v.volume.to_f64().unwrap_or(0.0),
            fees: v.fees.to_f64().unwrap_or(0.0),
            swap_count: v.swap_count as u64,
        })
        .collect();
    
    let total_volume = daily_volumes.iter().map(|v| v.volume).sum();
    let total_fees = daily_volumes.iter().map(|v| v.fees).sum();
    
    Ok(Json(VolumeResponse {
        daily_volumes,
        total_volume,
        total_fees,
        timestamp: chrono::Utc::now().timestamp(),
    }))
}
