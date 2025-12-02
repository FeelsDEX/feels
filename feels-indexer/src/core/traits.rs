//! Core trait abstractions (Ports in Hexagonal Architecture)

use async_trait::async_trait;
use futures::Stream;
use serde::{de::DeserializeOwned, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::pin::Pin;
use std::time::Duration;

use super::error::IndexerResult;
use super::types::*;

/// Account stream type
pub type AccountStream = Pin<Box<dyn Stream<Item = IndexerResult<AccountUpdate>> + Send>>;

/// Transaction stream type
pub type TransactionStream = Pin<Box<dyn Stream<Item = IndexerResult<TransactionUpdate>> + Send>>;

/// Storage port - abstraction for all storage operations
#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Store a market
    async fn store_market(&self, market: &crate::domain::models::IndexedMarket) -> IndexerResult<()>;
    
    /// Get a market by address
    async fn get_market(&self, address: &Pubkey) -> IndexerResult<Option<crate::domain::models::IndexedMarket>>;
    
    /// Query markets with filters
    async fn query_markets(&self, query: MarketQuery) -> IndexerResult<Vec<crate::domain::models::IndexedMarket>>;
    
    /// Store a position
    async fn store_position(&self, position: &crate::domain::models::IndexedPosition) -> IndexerResult<()>;
    
    /// Get a position by address
    async fn get_position(&self, address: &Pubkey) -> IndexerResult<Option<crate::domain::models::IndexedPosition>>;
    
    /// Store a swap
    async fn store_swap(&self, swap: &crate::domain::models::IndexedSwap) -> IndexerResult<()>;
    
    /// Health check
    async fn health_check(&self) -> IndexerResult<StorageHealth>;
}

/// Cache port - abstraction for caching operations
#[async_trait]
pub trait CachePort: Send + Sync {
    /// Get a value from cache
    async fn get<T>(&self, key: &str) -> IndexerResult<Option<T>>
    where
        T: DeserializeOwned + Send;
    
    /// Set a value in cache with TTL
    async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> IndexerResult<()>
    where
        T: Serialize + Send + Sync;
    
    /// Delete a key from cache
    async fn delete(&self, key: &str) -> IndexerResult<()>;
    
    /// Check if cache is healthy
    async fn health_check(&self) -> IndexerResult<()>;
}

/// Event stream port - abstraction for blockchain event streaming
#[async_trait]
pub trait EventStreamPort: Send + Sync {
    /// Subscribe to account updates for a specific program
    async fn subscribe_accounts(&self, program_id: Pubkey) -> IndexerResult<AccountStream>;
    
    /// Subscribe to transaction updates
    async fn subscribe_transactions(&self) -> IndexerResult<TransactionStream>;
    
    /// Health check
    async fn health_check(&self) -> IndexerResult<()>;
}

/// Account processor trait - transforms raw account data to domain models
#[async_trait]
pub trait AccountProcessor: Send + Sync {
    /// The raw account data type
    type Account;
    
    /// The output domain model type
    type Output;
    
    /// Deserialize raw account data
    fn deserialize(&self, data: &[u8]) -> IndexerResult<Self::Account>;
    
    /// Transform account to domain model
    fn transform(&self, account: Self::Account, context: ProcessContext) -> IndexerResult<Self::Output>;
    
    /// Process account update (template method)
    async fn process(&self, _pubkey: Pubkey, data: &[u8], context: ProcessContext) -> IndexerResult<Self::Output> {
        let account = self.deserialize(data)?;
        self.transform(account, context)
    }
}

/// Storage health information
#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub postgres: bool,
    pub rocksdb: bool,
    pub redis: bool,
    pub tantivy: bool,
    pub overall: bool,
}

impl StorageHealth {
    pub fn all_healthy() -> Self {
        Self {
            postgres: true,
            rocksdb: true,
            redis: true,
            tantivy: true,
            overall: true,
        }
    }
    
    pub fn is_healthy(&self) -> bool {
        self.overall
    }
}

