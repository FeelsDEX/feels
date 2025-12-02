//! Feels Protocol Indexer Library
//! 
//! Modern multi-tier indexer for Feels Protocol state with PostgreSQL,
//! Redis caching, Tantivy search, and RocksDB for raw blockchain data.
//!
//! ## Architecture
//!
//! This codebase follows hexagonal architecture (ports & adapters):
//!
//! - **core**: Domain abstractions, error types, and port traits
//! - **domain**: Business logic and domain models
//! - **adapters**: Infrastructure implementations (storage, networking)
//! - **infrastructure**: Cross-cutting concerns (config, telemetry)
//! - **api**: REST API layer
//!
//! ## New Architecture (Active Development)
pub mod adapters;
pub mod core;
pub mod domain;
pub mod infrastructure;

// ## Active Modules
pub mod config;
pub mod database;
pub mod geyser;

// Legacy models module - now re-exports from domain
pub mod models {
    //! Legacy models module (DEPRECATED - use crate::domain::models instead)
    pub use crate::domain::models::*;
}

// API module for REST server
pub mod api;

// Re-export from new architecture
pub use core::{IndexerError, IndexerResult};
pub use adapters::storage::StorageAdapter;

// Legacy re-exports (for backwards compatibility)
pub use config::IndexerConfig;
pub use database::{DatabaseManager, DatabaseHealth};

#[cfg(test)]
mod tests {
    
    use crate::database::rocksdb::{RocksDBManager, ColumnFamilies};
    use crate::config::RocksDBConfig;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_rocksdb_basic() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config = RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: false,
            max_open_files: 100,
            write_buffer_size_mb: 16,
            max_write_buffer_number: 2,
            block_cache_size_mb: 32,
        };
        
        let db = RocksDBManager::new(config).await?;
        
        // Test basic put/get with typed data
        let test_value = vec![1u8, 2, 3, 4, 5];
        db.put(ColumnFamilies::MARKETS, b"test", &test_value)?;
        let result: Option<Vec<u8>> = db.get(ColumnFamilies::MARKETS, b"test")?;
        
        assert_eq!(result, Some(test_value));
        
        Ok(())
    }
}