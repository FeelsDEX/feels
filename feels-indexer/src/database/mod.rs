//! Modern multi-tier database layer

#[cfg(feature = "runtime-sqlx")]
pub mod postgres_runtime;

// Use runtime-sqlx by default
pub use postgres_runtime as postgres_impl;
pub mod redis;
pub mod redis_operations;
pub mod rocksdb;
pub mod rocksdb_operations;
pub mod tantivy;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;

// Re-export models for use in this module
use crate::models::IndexedMarket;

/// Database connection manager
pub struct DatabaseManager {
    pub postgres: Arc<postgres_impl::PostgresManager>,
    pub redis: Arc<redis::RedisManager>,
    pub rocksdb: Arc<rocksdb::RocksDBManager>,
    pub tantivy: Arc<tantivy::SearchManager>,
}

impl DatabaseManager {
    pub async fn new(
        postgres_url: &str,
        redis_url: &str,
        rocksdb_config: crate::config::RocksDBConfig,
        tantivy_path: &std::path::Path,
    ) -> Result<Self> {
        let postgres = postgres_impl::PostgresManager::new(postgres_url).await?;
        let redis = redis::RedisManager::new(redis_url).await?;
        let rocksdb = rocksdb::RocksDBManager::new(rocksdb_config).await?;
        let tantivy = tantivy::SearchManager::new(tantivy_path).await?;

        Ok(Self {
            postgres: Arc::new(postgres),
            redis: Arc::new(redis),
            rocksdb: Arc::new(rocksdb),
            tantivy: Arc::new(tantivy),
        })
    }
    
    // Forward market methods to PostgreSQL
    pub async fn get_market(&self, address: &solana_sdk::pubkey::Pubkey) -> Result<Option<IndexedMarket>> {
        // Convert Market to IndexedMarket if found
        if let Some(_market) = self.postgres.get_market_by_address(&address.to_string()).await? {
            // TODO: Convert Market to IndexedMarket properly
            // For now, return None to avoid compilation errors
            Ok(None)
        } else {
            Ok(None)
        }
    }
    
    pub async fn find_market_by_tokens(&self, _token_0: &solana_sdk::pubkey::Pubkey, _token_1: &solana_sdk::pubkey::Pubkey) -> Result<Option<IndexedMarket>> {
        // TODO: Implement find_market_by_tokens in PostgresManager
        // For now, return None to avoid compilation errors
        Ok(None)
    }

    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let postgres_healthy = self.postgres.health_check().await.is_ok();
        let redis_healthy = self.redis.health_check().await.is_ok();
        let rocksdb_healthy = true; // RocksDB is embedded, always healthy if initialized
        let tantivy_healthy = self.tantivy.health_check().await.is_ok();

        Ok(DatabaseHealth {
            postgres: postgres_healthy,
            redis: redis_healthy,
            rocksdb: rocksdb_healthy,
            tantivy: tantivy_healthy,
            overall: postgres_healthy && redis_healthy && rocksdb_healthy && tantivy_healthy,
        })
    }
    
    // Helper for tests - also available to integration tests
    // Creates a dummy DatabaseManager with only RocksDB initialized
    pub async fn new_dummy() -> Result<Self> {
        use std::sync::Arc;
        
        // Create minimal configs for dummy managers
        let postgres_url = "postgresql://test:test@localhost/test";
        let redis_url = "redis://localhost:6379";
        
        // Create a temporary directory for tantivy in the system temp
        let temp_path = std::env::temp_dir().join(format!("tantivy_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_path)?;
        
        Ok(Self {
            postgres: Arc::new(postgres_impl::PostgresManager::new(postgres_url).await?),
            redis: Arc::new(redis::RedisManager::new(redis_url).await?),
            rocksdb: Arc::new(rocksdb::RocksDBManager::new(crate::config::RocksDBConfig {
                path: std::env::temp_dir().join(format!("rocksdb_test_{}", uuid::Uuid::new_v4())),
                enable_compression: false,
                max_open_files: 100,
                write_buffer_size_mb: 16,
                max_write_buffer_number: 2,
                block_cache_size_mb: 32,
            }).await?),
            tantivy: Arc::new(tantivy::SearchManager::new(&temp_path).await?),
        })
    }
    
    // Helper for tests that only need RocksDB
    pub async fn new_rocksdb_only(rocksdb_config: crate::config::RocksDBConfig) -> Result<Self> {
        use std::sync::Arc;
        
        let rocksdb = rocksdb::RocksDBManager::new(rocksdb_config).await?;
        
        // Create temp path for tantivy
        let temp_path = std::env::temp_dir().join(format!("tantivy_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_path)?;
        
        let postgres_url = "postgresql://test:test@localhost/test";
        let redis_url = "redis://localhost:6379";
        
        Ok(Self {
            postgres: Arc::new(postgres_impl::PostgresManager::new(postgres_url).await?),
            redis: Arc::new(redis::RedisManager::new(redis_url).await?),
            rocksdb: Arc::new(rocksdb),
            tantivy: Arc::new(tantivy::SearchManager::new(&temp_path).await?),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub postgres: bool,
    pub redis: bool,
    pub rocksdb: bool,
    pub tantivy: bool,
    pub overall: bool,
}

/// Common database operations trait
#[async_trait]
pub trait DatabaseOperations {
    async fn health_check(&self) -> Result<()>;
}

/// Market data for database operations
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Market {
    pub id: Uuid,
    pub address: String,
    pub token_0: String,
    pub token_1: String,
    pub sqrt_price: rust_decimal::Decimal,
    pub liquidity: rust_decimal::Decimal,
    pub current_tick: i32,
    pub tick_spacing: i16,
    pub fee_bps: i16,
    pub is_paused: bool,
    pub phase: String,
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    pub fee_growth_global_0: rust_decimal::Decimal,
    pub fee_growth_global_1: rust_decimal::Decimal,
    pub total_volume_0: rust_decimal::Decimal,
    pub total_volume_1: rust_decimal::Decimal,
    pub total_fees_0: rust_decimal::Decimal,
    pub total_fees_1: rust_decimal::Decimal,
    pub swap_count: i64,
    pub unique_traders: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_updated_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Position {
    pub id: Uuid,
    pub address: String,
    pub market_id: Uuid,
    pub owner: String,
    pub liquidity: rust_decimal::Decimal,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_growth_inside_0_last: rust_decimal::Decimal,
    pub fee_growth_inside_1_last: rust_decimal::Decimal,
    pub tokens_owed_0: i64,
    pub tokens_owed_1: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_updated_slot: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Swap {
    pub id: Uuid,
    pub signature: String,
    pub market_id: Uuid,
    pub trader: String,
    pub amount_in: i64,
    pub amount_out: i64,
    pub token_in: String,
    pub token_out: String,
    pub sqrt_price_before: rust_decimal::Decimal,
    pub sqrt_price_after: rust_decimal::Decimal,
    pub tick_before: i32,
    pub tick_after: i32,
    pub liquidity: rust_decimal::Decimal,
    pub fee_amount: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub slot: i64,
    pub block_height: Option<i64>,
    pub price_impact_bps: Option<i16>,
    pub effective_price: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketSnapshot {
    pub id: Uuid,
    pub market_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub slot: i64,
    pub sqrt_price: rust_decimal::Decimal,
    pub tick: i32,
    pub liquidity: rust_decimal::Decimal,
    pub volume_0: rust_decimal::Decimal,
    pub volume_1: rust_decimal::Decimal,
    pub fees_0: rust_decimal::Decimal,
    pub fees_1: rust_decimal::Decimal,
    pub swap_count: i32,
    pub tvl_token_0: rust_decimal::Decimal,
    pub tvl_token_1: rust_decimal::Decimal,
    pub tvl_usd: Option<rust_decimal::Decimal>,
}

