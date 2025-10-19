//! Database integration tests for PostgreSQL and Redis
//! 
//! These tests require running PostgreSQL and Redis instances.
//! They will be skipped if the services are not available.

use anyhow::Result;
use feels_indexer::config::{PostgresConfig, RedisConfig, RocksDBConfig};
use feels_indexer::database::{DatabaseManager, Market, Swap, Position, MarketSnapshot};
use tempfile::TempDir;
use std::env;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::Utc;

/// Check if PostgreSQL is available by trying to connect
async fn postgres_available(config: &PostgresConfig) -> bool {
    let url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        config.username, config.password, config.host, config.port, config.database
    );
    
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect(&url)
        .await
        .is_ok()
}

/// Check if Redis is available by trying to connect
async fn redis_available(url: &str) -> bool {
    redis::Client::open(url)
        .and_then(|client| client.get_connection())
        .is_ok()
}

/// Get test database configuration from environment or use defaults
fn get_test_postgres_config() -> PostgresConfig {
    PostgresConfig {
        host: env::var("TEST_POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("TEST_POSTGRES_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5432),
        database: env::var("TEST_POSTGRES_DB").unwrap_or_else(|_| "feels_indexer_test".to_string()),
        username: env::var("TEST_POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()),
        password: env::var("TEST_POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string()),
        max_connections: 5,
    }
}

/// Get test Redis configuration from environment or use defaults
fn get_test_redis_config() -> RedisConfig {
    RedisConfig {
        url: env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        pool_size: 5,
        ttl_seconds: 3600,
    }
}

/// Create a test database manager with all services
async fn create_full_db_manager() -> Result<Option<(DatabaseManager, TempDir)>> {
    let postgres_config = get_test_postgres_config();
    let redis_config = get_test_redis_config();
    
    // Check if services are available
    if !postgres_available(&postgres_config).await {
        eprintln!("PostgreSQL not available, skipping test");
        return Ok(None);
    }
    
    if !redis_available(&redis_config.url).await {
        eprintln!("Redis not available, skipping test");
        return Ok(None);
    }
    
    let temp_dir = TempDir::new()?;
    let rocksdb_config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    // Build PostgreSQL URL
    let postgres_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        postgres_config.username,
        postgres_config.password,
        postgres_config.host,
        postgres_config.port,
        postgres_config.database
    );
    
    let db_manager = DatabaseManager::new(
        &postgres_url,
        &redis_config.url,
        rocksdb_config,
        temp_dir.path(),
    ).await?;
    
    Ok(Some((db_manager, temp_dir)))
}

#[tokio::test]
async fn test_postgres_market_operations() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // Create a test market
    let market = Market {
        id: Uuid::new_v4(),
        address: "TestMarket11111111111111111111111".to_string(),
        token_0: "So11111111111111111111111111111111111111112".to_string(),
        token_1: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(500000),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(1000000),
        total_volume_1: Decimal::from(2000000),
        total_fees_0: Decimal::from(3000),
        total_fees_1: Decimal::from(6000),
        swap_count: 100,
        unique_traders: 25,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    
    // Test upsert
    db_manager.postgres.upsert_market(&market).await?;
    
    // Test get
    let retrieved = db_manager.postgres.get_market(&market.address).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.address, market.address);
    assert_eq!(retrieved.token_0, market.token_0);
    assert_eq!(retrieved.fee_bps, market.fee_bps);
    
    // Test get_top_markets
    let top_markets = db_manager.postgres.get_top_markets(10, 0).await?;
    assert!(!top_markets.is_empty());
    assert!(top_markets.iter().any(|m| m.address == market.address));
    
    // Test get_markets_by_liquidity
    let liquid_markets = db_manager.postgres
        .get_markets_by_liquidity(Decimal::from(100000), 10)
        .await?;
    assert!(!liquid_markets.is_empty());
    
    // Test get_markets_count
    let count = db_manager.postgres.get_markets_count().await?;
    assert!(count > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_postgres_swap_operations() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // First create a market
    let market = Market {
        id: Uuid::new_v4(),
        address: "SwapTestMarket1111111111111111111".to_string(),
        token_0: "TokenA111111111111111111111111111111111111".to_string(),
        token_1: "TokenB111111111111111111111111111111111111".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(500000),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(0),
        total_volume_1: Decimal::from(0),
        total_fees_0: Decimal::from(0),
        total_fees_1: Decimal::from(0),
        swap_count: 0,
        unique_traders: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    db_manager.postgres.upsert_market(&market).await?;
    
    // Create test swaps
    let swap1 = Swap {
        id: Uuid::new_v4(),
        signature: "swap1_signature_1111111111111111111111111111".to_string(),
        market_id: market.id,
        trader: "Trader11111111111111111111111111111111111111".to_string(),
        amount_in: 1000000,
        amount_out: 990000,
        token_in: market.token_0.clone(),
        token_out: market.token_1.clone(),
        sqrt_price_before: Decimal::from(1000000),
        sqrt_price_after: Decimal::from(1001000),
        tick_before: 0,
        tick_after: 100,
        liquidity: Decimal::from(500000),
        fee_amount: 3000,
        timestamp: Utc::now(),
        slot: 12345679,
        block_height: Some(6172839),
        price_impact_bps: Some(10),
        effective_price: Some(Decimal::from_f64(0.99).unwrap()),
    };
    
    let swap2 = Swap {
        id: Uuid::new_v4(),
        signature: "swap2_signature_2222222222222222222222222222".to_string(),
        market_id: market.id,
        trader: "Trader22222222222222222222222222222222222222".to_string(),
        amount_in: 2000000,
        amount_out: 1980000,
        token_in: market.token_1.clone(),
        token_out: market.token_0.clone(),
        sqrt_price_before: Decimal::from(1001000),
        sqrt_price_after: Decimal::from(999000),
        tick_before: 100,
        tick_after: -50,
        liquidity: Decimal::from(500000),
        fee_amount: 6000,
        timestamp: Utc::now(),
        slot: 12345680,
        block_height: Some(6172840),
        price_impact_bps: Some(20),
        effective_price: Some(Decimal::from_f64(0.99).unwrap()),
    };
    
    // Test insert
    db_manager.postgres.insert_swap(&swap1).await?;
    db_manager.postgres.insert_swap(&swap2).await?;
    
    // Test get_swaps_for_market
    let market_swaps = db_manager.postgres
        .get_swaps_for_market(market.id, 10, 0)
        .await?;
    assert_eq!(market_swaps.len(), 2);
    
    // Test get_swaps_by_trader
    let trader_swaps = db_manager.postgres
        .get_swaps_by_trader(&swap1.trader, 10, 0)
        .await?;
    assert_eq!(trader_swaps.len(), 1);
    assert_eq!(trader_swaps[0].signature, swap1.signature);
    
    // Test get_swaps_count_by_market_id
    let swap_count = db_manager.postgres
        .get_swaps_count_by_market_id(market.id)
        .await?;
    assert_eq!(swap_count, 2);
    
    // Test get_swaps_count
    let total_swaps = db_manager.postgres.get_swaps_count().await?;
    assert!(total_swaps >= 2);
    
    Ok(())
}

#[tokio::test]
async fn test_postgres_position_operations() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // First create a market
    let market = Market {
        id: Uuid::new_v4(),
        address: "PositionTestMarket111111111111111".to_string(),
        token_0: "TokenX111111111111111111111111111111111111".to_string(),
        token_1: "TokenY111111111111111111111111111111111111".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(0), // Will be updated by positions
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(0),
        total_volume_1: Decimal::from(0),
        total_fees_0: Decimal::from(0),
        total_fees_1: Decimal::from(0),
        swap_count: 0,
        unique_traders: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    db_manager.postgres.upsert_market(&market).await?;
    
    // Create test positions
    let position1 = Position {
        id: Uuid::new_v4(),
        address: "Position1111111111111111111111111".to_string(),
        market_id: market.id,
        owner: "Owner11111111111111111111111111111111111111".to_string(),
        liquidity: Decimal::from(100000),
        tick_lower: -1000,
        tick_upper: 1000,
        fee_growth_inside_0_last: Decimal::from(0),
        fee_growth_inside_1_last: Decimal::from(0),
        tokens_owed_0: 0,
        tokens_owed_1: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    
    let position2 = Position {
        id: Uuid::new_v4(),
        address: "Position2222222222222222222222222".to_string(),
        market_id: market.id,
        owner: "Owner11111111111111111111111111111111111111".to_string(), // Same owner
        liquidity: Decimal::from(200000),
        tick_lower: -2000,
        tick_upper: 2000,
        fee_growth_inside_0_last: Decimal::from(0),
        fee_growth_inside_1_last: Decimal::from(0),
        tokens_owed_0: 0,
        tokens_owed_1: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345679,
    };
    
    // Test upsert
    db_manager.postgres.upsert_position(&position1).await?;
    db_manager.postgres.upsert_position(&position2).await?;
    
    // Test get_position_by_address
    let retrieved = db_manager.postgres
        .get_position_by_address(&position1.address)
        .await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.address, position1.address);
    assert_eq!(retrieved.liquidity, position1.liquidity);
    
    // Test get_positions_by_owner
    let owner_positions = db_manager.postgres
        .get_positions_by_owner(&position1.owner)
        .await?;
    assert_eq!(owner_positions.len(), 2);
    
    // Test get_positions_by_market_id
    let market_positions = db_manager.postgres
        .get_positions_by_market_id(market.id, 10, 0)
        .await?;
    assert_eq!(market_positions.len(), 2);
    
    // Test get_positions_count_by_market_id
    let position_count = db_manager.postgres
        .get_positions_count_by_market_id(market.id)
        .await?;
    assert_eq!(position_count, 2);
    
    // Test get_positions_paginated
    let all_positions = db_manager.postgres
        .get_positions_paginated(10, 0)
        .await?;
    assert!(all_positions.len() >= 2);
    
    // Test get_positions_count
    let total_positions = db_manager.postgres.get_positions_count().await?;
    assert!(total_positions >= 2);
    
    Ok(())
}

#[tokio::test]
async fn test_postgres_market_snapshots() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // Create a market first
    let market = Market {
        id: Uuid::new_v4(),
        address: "SnapshotTestMarket111111111111111".to_string(),
        token_0: "TokenA111111111111111111111111111111111111".to_string(),
        token_1: "TokenB111111111111111111111111111111111111".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(500000),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(1000000),
        total_volume_1: Decimal::from(2000000),
        total_fees_0: Decimal::from(3000),
        total_fees_1: Decimal::from(6000),
        swap_count: 100,
        unique_traders: 25,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    db_manager.postgres.upsert_market(&market).await?;
    
    // Create snapshots at different times
    let now = Utc::now();
    for i in 0..5 {
        let snapshot = MarketSnapshot {
            id: Uuid::new_v4(),
            market_id: market.id,
            timestamp: now - chrono::Duration::hours(i),
            slot: 12345678 + (i as i64 * 100),
            sqrt_price: Decimal::from(1000000 + i * 1000),
            tick: (i * 10) as i32,
            liquidity: Decimal::from(500000 + i * 10000),
            volume_0: Decimal::from(100000 * i),
            volume_1: Decimal::from(200000 * i),
            fees_0: Decimal::from(300 * i),
            fees_1: Decimal::from(600 * i),
            swap_count: (10 * i) as i32,
            tvl_token_0: Decimal::from(1000000),
            tvl_token_1: Decimal::from(2000000),
            tvl_usd: Some(Decimal::from(3000000)),
        };
        
        db_manager.postgres.insert_market_snapshot(&snapshot).await?;
    }
    
    // Test get_market_snapshots
    let snapshots = db_manager.postgres
        .get_market_snapshots(market.id, 24)
        .await?;
    assert_eq!(snapshots.len(), 5);
    
    // Verify snapshots are ordered by timestamp (most recent first)
    for i in 1..snapshots.len() {
        assert!(snapshots[i-1].timestamp >= snapshots[i].timestamp);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_redis_caching() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // Test setting and getting a cached value
    let key = "test:market:stats";
    let value = serde_json::json!({
        "total_volume": "1000000",
        "total_fees": "3000",
        "active_markets": 10
    });
    
    db_manager.redis.set_json(key, &value, 60).await?;
    
    let retrieved: Option<serde_json::Value> = db_manager.redis.get_json(key).await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), value);
    
    // Test cache expiration
    let short_ttl_key = "test:short:ttl";
    db_manager.redis.set_json(short_ttl_key, &value, 1).await?;
    
    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let expired: Option<serde_json::Value> = db_manager.redis.get_json(short_ttl_key).await?;
    assert!(expired.is_none());
    
    // Test deletion
    db_manager.redis.delete(key).await?;
    let deleted: Option<serde_json::Value> = db_manager.redis.get_json(key).await?;
    assert!(deleted.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_redis_market_caching() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // Create and store a market in PostgreSQL
    let market = Market {
        id: Uuid::new_v4(),
        address: "CachedMarket11111111111111111111".to_string(),
        token_0: "TokenA111111111111111111111111111111111111".to_string(),
        token_1: "TokenB111111111111111111111111111111111111".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(500000),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(1000000),
        total_volume_1: Decimal::from(2000000),
        total_fees_0: Decimal::from(3000),
        total_fees_1: Decimal::from(6000),
        swap_count: 100,
        unique_traders: 25,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    
    db_manager.postgres.upsert_market(&market).await?;
    
    // Cache the market data
    let cache_key = format!("market:{}", market.address);
    db_manager.redis.set_json(&cache_key, &market, 300).await?;
    
    // Retrieve from cache
    let cached: Option<Market> = db_manager.redis.get_json(&cache_key).await?;
    assert!(cached.is_some());
    let cached = cached.unwrap();
    assert_eq!(cached.address, market.address);
    assert_eq!(cached.total_volume_0, market.total_volume_0);
    
    Ok(())
}

#[tokio::test]
async fn test_cross_database_consistency() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    let market_address = "ConsistencyTestMarket11111111111";
    
    // Create market in PostgreSQL
    let market = Market {
        id: Uuid::new_v4(),
        address: market_address.to_string(),
        token_0: "TokenA111111111111111111111111111111111111".to_string(),
        token_1: "TokenB111111111111111111111111111111111111".to_string(),
        sqrt_price: Decimal::from(1000000),
        liquidity: Decimal::from(500000),
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: "PriceDiscovery".to_string(),
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: Decimal::from(0),
        fee_growth_global_1: Decimal::from(0),
        total_volume_0: Decimal::from(1000000),
        total_volume_1: Decimal::from(2000000),
        total_fees_0: Decimal::from(3000),
        total_fees_1: Decimal::from(6000),
        swap_count: 100,
        unique_traders: 25,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    
    db_manager.postgres.upsert_market(&market).await?;
    
    // Store in RocksDB
    let rocksdb_market = feels_indexer::models::Market {
        address: solana_sdk::pubkey::Pubkey::new_unique(),
        token_0: solana_sdk::pubkey::Pubkey::new_unique(),
        token_1: solana_sdk::pubkey::Pubkey::new_unique(),
        sqrt_price: 1000000,
        liquidity: 500000,
        current_tick: 0,
        tick_spacing: 10,
        fee_bps: 30,
        is_paused: false,
        phase: feels_indexer::models::PoolPhase::PriceDiscovery,
        global_lower_tick: -887200,
        global_upper_tick: 887200,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
        total_volume_0: 1000000,
        total_volume_1: 2000000,
        total_fees_0: 3000,
        total_fees_1: 6000,
        swap_count: 100,
        unique_traders: 25,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_updated_slot: 12345678,
    };
    
    db_manager.rocksdb.put_market(market_address, &rocksdb_market)?;
    
    // Cache in Redis
    let cache_key = format!("market:{}", market_address);
    db_manager.redis.set_json(&cache_key, &market, 300).await?;
    
    // Verify all three databases have the data
    let pg_market = db_manager.postgres.get_market(market_address).await?;
    assert!(pg_market.is_some());
    
    let rocksdb_market = db_manager.rocksdb.get_market(market_address)?;
    assert!(rocksdb_market.is_some());
    
    let redis_market: Option<Market> = db_manager.redis.get_json(&cache_key).await?;
    assert!(redis_market.is_some());
    
    // Verify consistency across databases
    let pg_market = pg_market.unwrap();
    let redis_market = redis_market.unwrap();
    
    assert_eq!(pg_market.address, redis_market.address);
    assert_eq!(pg_market.total_volume_0, redis_market.total_volume_0);
    assert_eq!(pg_market.swap_count, redis_market.swap_count);
    
    Ok(())
}

#[tokio::test]
async fn test_database_health_check() -> Result<()> {
    let Some((db_manager, _temp_dir)) = create_full_db_manager().await? else {
        return Ok(()); // Skip test if services not available
    };
    
    // Test individual health checks
    let pg_health = db_manager.postgres.health_check().await;
    assert!(pg_health.is_ok());
    
    let redis_health = db_manager.redis.health_check().await;
    assert!(redis_health.is_ok());
    
    // Test combined health check
    let overall_health = db_manager.health_check().await?;
    assert!(overall_health.postgres);
    assert!(overall_health.redis);
    assert!(overall_health.rocksdb);
    assert!(overall_health.overall);
    
    Ok(())
}