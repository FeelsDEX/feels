//! Feels Protocol Indexer Library
//! 
//! Modern multi-tier indexer for Feels Protocol state with PostgreSQL,
//! Redis caching, Tantivy search, and RocksDB for raw blockchain data.

#![allow(dead_code)]

pub mod api;
pub mod config;
pub mod database;
pub mod geyser;
pub mod models;
pub mod processors;
pub mod repositories;
pub mod rpc_client;
pub mod services;
pub mod sdk_types;
pub mod streaming_client;

mod minimal_test;

// Re-export commonly used types
pub use config::IndexerConfig;
pub use database::{DatabaseManager, DatabaseHealth};
pub use models::*;
pub use sdk_types::feels_sdk;

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
        
        // Test basic put/get
        db.put_raw(ColumnFamilies::MARKETS, b"test", b"value")?;
        let result = db.get::<Vec<u8>>(ColumnFamilies::MARKETS, b"test")?;
        
        assert_eq!(result, Some(b"value".to_vec()));
        
        Ok(())
    }
}