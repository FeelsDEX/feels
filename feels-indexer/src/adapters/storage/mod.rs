//! Storage adapters
//!
//! This module contains all storage backend implementations that together
//! implement the StoragePort trait, providing a unified interface for
//! data persistence.

pub mod postgres;
pub mod redis;
pub mod rocksdb;
pub mod tantivy;

use crate::core::{
    CachePort, IndexerResult, MarketQuery, StorageHealth, StoragePort,
};
use crate::domain::models::{IndexedMarket, IndexedPosition, IndexedSwap};
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

pub use postgres::PostgresClient;
pub use redis::RedisClient;
pub use rocksdb::{RocksDBClient, ColumnFamilies};
pub use tantivy::TantivyClient;

/// Unified storage adapter that coordinates all storage backends
pub struct StorageAdapter {
    postgres: Arc<PostgresClient>,
    rocksdb: Arc<RocksDBClient>,
    redis: Arc<RedisClient>,
    tantivy: Arc<TantivyClient>,
}

impl StorageAdapter {
    /// Create a new storage adapter with all backends
    pub async fn new(
        postgres_url: &str,
        redis_url: &str,
        rocksdb_config: crate::config::RocksDBConfig,
        tantivy_path: &std::path::Path,
    ) -> IndexerResult<Self> {
        let postgres = Arc::new(PostgresClient::connect(postgres_url).await?);
        let redis = Arc::new(RedisClient::connect(redis_url).await?);
        let rocksdb = Arc::new(RocksDBClient::open(&rocksdb_config).await?);
        let tantivy = Arc::new(TantivyClient::open(tantivy_path).await?);
        
        debug!("All storage backends initialized successfully");
        
        Ok(Self {
            postgres,
            rocksdb,
            redis,
            tantivy,
        })
    }
    
    /// Get reference to cache port
    pub fn cache(&self) -> &Arc<RedisClient> {
        &self.redis
    }
    
    /// Get reference to RocksDB client
    pub fn rocksdb(&self) -> &Arc<RocksDBClient> {
        &self.rocksdb
    }
}

#[async_trait]
impl StoragePort for StorageAdapter {
    async fn store_market(&self, market: &IndexedMarket) -> IndexerResult<()> {
        // Store in PostgreSQL (analytical queries)
        self.postgres.store_market(market).await?;
        
        // Store in RocksDB (raw state)
        let key = market.address.to_bytes();
        self.rocksdb.put(ColumnFamilies::MARKETS, &key, market)?;
        
        // Invalidate cache
        let cache_key = format!("market:{}", market.address);
        if let Err(e) = self.redis.delete(&cache_key).await {
            warn!("Failed to invalidate market cache: {}", e);
        }
        
        // Index in search
        if let Err(e) = self.tantivy.index_market(market).await {
            warn!("Failed to index market in search: {}", e);
        }
        
        debug!("Stored market: {}", market.address);
        
        Ok(())
    }
    
    async fn get_market(&self, address: &Pubkey) -> IndexerResult<Option<IndexedMarket>> {
        let cache_key = format!("market:{}", address);
        
        // Try cache first
        if let Some(market) = self.redis.get(&cache_key).await? {
            debug!("Market cache hit: {}", address);
            return Ok(Some(market));
        }
        
        // Try PostgreSQL
        if let Some(market) = self.postgres.get_market(address).await? {
            // Populate cache
            if let Err(e) = self.redis.set(&cache_key, &market, Duration::from_secs(300)).await {
                warn!("Failed to cache market: {}", e);
            }
            return Ok(Some(market));
        }
        
        debug!("Market not found: {}", address);
        Ok(None)
    }
    
    async fn query_markets(&self, query: MarketQuery) -> IndexerResult<Vec<IndexedMarket>> {
        // Use PostgreSQL for complex queries
        self.postgres.query_markets(query).await
    }
    
    async fn store_position(&self, position: &IndexedPosition) -> IndexerResult<()> {
        // Store in PostgreSQL
        self.postgres.store_position(position).await?;
        
        // Store in RocksDB
        let key = position.address.to_bytes();
        self.rocksdb.put(ColumnFamilies::POSITIONS, &key, position)?;
        
        debug!("Stored position: {}", position.address);
        
        Ok(())
    }
    
    async fn get_position(&self, address: &Pubkey) -> IndexerResult<Option<IndexedPosition>> {
        // Try cache
        let cache_key = format!("position:{}", address);
        if let Some(position) = self.redis.get(&cache_key).await? {
            return Ok(Some(position));
        }
        
        // Try PostgreSQL
        if let Some(position) = self.postgres.get_position(address).await? {
            if let Err(e) = self.redis.set(&cache_key, &position, Duration::from_secs(180)).await {
                warn!("Failed to cache position: {}", e);
            }
            return Ok(Some(position));
        }
        
        Ok(None)
    }
    
    async fn store_swap(&self, swap: &IndexedSwap) -> IndexerResult<()> {
        // Store in PostgreSQL (for analytics)
        self.postgres.store_swap(swap).await?;
        
        // Store in RocksDB (for raw data)
        let key = swap.signature.as_bytes();
        self.rocksdb.put(ColumnFamilies::SWAPS, key, swap)?;
        
        debug!("Stored swap: {}", swap.signature);
        
        Ok(())
    }
    
    async fn health_check(&self) -> IndexerResult<StorageHealth> {
        let postgres = self.postgres.health_check().await.is_ok();
        let rocksdb = self.rocksdb.health_check().is_ok();
        let redis = self.redis.health_check().await.is_ok();
        let tantivy = self.tantivy.health_check().await.is_ok();
        
        let overall = postgres && rocksdb && redis && tantivy;
        
        Ok(StorageHealth {
            postgres,
            rocksdb,
            redis,
            tantivy,
            overall,
        })
    }
}

