//! API integration tests

use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use feels_indexer::api::{ApiState, create_router};
use feels_indexer::config::RocksDBConfig;
use feels_indexer::database::DatabaseManager;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt; // for `oneshot`

/// Create a test API setup with database
async fn create_test_api() -> Result<(Router, Arc<DatabaseManager>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    let db_manager = Arc::new(DatabaseManager::new_rocksdb_only(config).await?);
    let state = ApiState {
        db_manager: db_manager.clone(),
    };
    
    let app = create_router(state);
    
    Ok((app, db_manager, temp_dir))
}

/// Create and store a test market
async fn create_and_store_test_market(db_manager: &DatabaseManager, address: &str) -> Result<()> {
    let market = feels_indexer::database::Market {
        id: uuid::Uuid::new_v4(),
        address: address.to_string(),
        token_0: "So11111111111111111111111111111111111111112".to_string(),
        token_1: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        sqrt_price: rust_decimal::Decimal::from(1000000000000000000u64),
        liquidity: rust_decimal::Decimal::from(1000000u64),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -100800,
        global_upper_tick: 100800,
        fee_growth_global_0: rust_decimal::Decimal::from(0),
        fee_growth_global_1: rust_decimal::Decimal::from(0),
        total_volume_0: rust_decimal::Decimal::from(5000000000u64),
        total_volume_1: rust_decimal::Decimal::from(1000000000u64),
        total_fees_0: rust_decimal::Decimal::from(15000000u64),
        total_fees_1: rust_decimal::Decimal::from(3000000u64),
        swap_count: 1000,
        unique_traders: 50,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_updated_slot: 12345678,
    };
    
    // Store in PostgreSQL (would normally do this)
    // db_manager.postgres.upsert_market(&market).await?;
    
    // For now, just store in RocksDB
    db_manager.rocksdb.put_market(address, &feels_indexer::models::Market {
        address: Pubkey::from_str(address)?,
        token_0: Pubkey::from_str("So11111111111111111111111111111111111111112")?,
        token_1: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?,
        sqrt_price: 1000000000000000000u128,
        liquidity: 1000000u128,
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: feels_indexer::models::PoolPhase::PriceDiscovery,
        global_lower_tick: -100800,
        global_upper_tick: 100800,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
        total_volume_0: 5000000000,
        total_volume_1: 1000000000,
        total_fees_0: 15000000,
        total_fees_1: 3000000,
        swap_count: 1000,
        unique_traders: 50,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_updated_slot: 12345678,
    })?;
    
    Ok(())
}

#[tokio::test]
async fn test_health_endpoint() -> Result<()> {
    let (app, _db_manager, _temp_dir) = create_test_api().await?;
    
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Should contain database health information
    assert!(body_str.contains("rocksdb"));
    
    Ok(())
}

#[tokio::test]
async fn test_markets_endpoint_empty() -> Result<()> {
    let (app, _db_manager, _temp_dir) = create_test_api().await?;
    
    let request = Request::builder()
        .uri("/api/v1/markets")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Should return empty data array
    assert!(body_str.contains("\"data\":[]"));
    assert!(body_str.contains("\"total\":0"));
    
    Ok(())
}

#[tokio::test]
async fn test_markets_endpoint_with_data() -> Result<()> {
    let (app, db_manager, _temp_dir) = create_test_api().await?;
    
    // Add a test market
    let market_address = "11111111111111111111111111111112";
    create_and_store_test_market(&db_manager, market_address).await?;
    
    // Create new app instance to pick up the data
    let state = ApiState {
        db_manager: db_manager.clone(),
    };
    let app = create_router(state);
    
    let request = Request::builder()
        .uri("/api/v1/markets")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Should contain market data
    assert!(body_str.contains("\"data\":["));
    assert!(body_str.contains(market_address));
    
    Ok(())
}

#[tokio::test]
async fn test_market_detail_endpoint() -> Result<()> {
    let (app, db_manager, _temp_dir) = create_test_api().await?;
    
    // Add a test market
    let market_address = "11111111111111111111111111111112";
    create_and_store_test_market(&db_manager, market_address).await?;
    
    // Create new app instance to pick up the data
    let state = ApiState {
        db_manager: db_manager.clone(),
    };
    let app = create_router(state);
    
    let request = Request::builder()
        .uri(&format!("/api/v1/markets/{}", market_address))
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Should contain market details
    assert!(body_str.contains(market_address));
    assert!(body_str.contains("\"token_0\""));
    assert!(body_str.contains("\"token_1\""));
    
    Ok(())
}

#[tokio::test]
async fn test_market_not_found() -> Result<()> {
    let (app, _db_manager, _temp_dir) = create_test_api().await?;
    
    let request = Request::builder()
        .uri("/api/v1/markets/nonexistent_market_address")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_endpoint() -> Result<()> {
    let (app, _db_manager, _temp_dir) = create_test_api().await?;
    
    let request = Request::builder()
        .uri("/api/v1/stats")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Should contain stats
    assert!(body_str.contains("\"total_markets\""));
    assert!(body_str.contains("\"total_volume_24h\""));
    
    Ok(())
}

#[tokio::test]
async fn test_pagination() -> Result<()> {
    let (app, db_manager, _temp_dir) = create_test_api().await?;
    
    // Add multiple test markets
    for i in 0..15 {
        let address = format!("1111111111111111111111111111111{}", i);
        create_and_store_test_market(&db_manager, &address).await?;
    }
    
    // Create new app instance to pick up the data
    let state = ApiState {
        db_manager: db_manager.clone(),
    };
    let app = create_router(state);
    
    // Test first page
    let request = Request::builder()
        .uri("/api/v1/markets?page=1&limit=10")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    // Parse JSON to check pagination
    let json: serde_json::Value = serde_json::from_str(&body_str)?;
    
    assert_eq!(json["page"], 1);
    assert_eq!(json["limit"], 10);
    assert!(json["data"].as_array().unwrap().len() <= 10);
    
    Ok(())
}