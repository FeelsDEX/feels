//! RocksDB client implementation

use crate::config::RocksDBConfig;
use crate::core::{IndexerResult, IndexerError, StorageError};
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Column family names
pub struct ColumnFamilies;

impl ColumnFamilies {
    pub const MARKETS: &'static str = "markets";
    pub const POSITIONS: &'static str = "positions";
    pub const SWAPS: &'static str = "swaps";
    pub const BUFFERS: &'static str = "buffers";
    pub const FLOORS: &'static str = "floors";
    pub const METADATA: &'static str = "metadata";
}

/// RocksDB client for raw blockchain data storage
pub struct RocksDBClient {
    db: DB,
}

impl RocksDBClient {
    /// Open RocksDB with the specified configuration
    pub async fn open(config: &RocksDBConfig) -> IndexerResult<Self> {
        let path = &config.path;
        
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        }
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_open_files(config.max_open_files);
        opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
        opts.set_max_write_buffer_number(config.max_write_buffer_number);
        
        if config.enable_compression {
            opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        }
        
        // Define column families
        let cf_names = [ColumnFamilies::MARKETS,
            ColumnFamilies::POSITIONS,
            ColumnFamilies::SWAPS,
            ColumnFamilies::BUFFERS,
            ColumnFamilies::FLOORS,
            ColumnFamilies::METADATA];
        
        let cfs: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                cf_opts.set_max_write_buffer_number(config.max_write_buffer_number);
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect();
        
        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        
        info!("RocksDB opened successfully at: {:?}", path);
        
        Ok(Self { db })
    }
    
    /// Put a value into a column family
    pub fn put<V>(&self, cf_name: &str, key: &[u8], value: &V) -> IndexerResult<()>
    where
        V: Serialize,
    {
        let cf = self.db
            .cf_handle(cf_name)
            .ok_or_else(|| IndexerError::Storage(StorageError::RocksDB(
                format!("Column family not found: {}", cf_name)
            )))?;
        
        let serialized = bincode::serialize(value)?;
        
        self.db
            .put_cf(&cf, key, serialized)
            .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        
        debug!("Put key in CF {}: {} bytes", cf_name, key.len());
        
        Ok(())
    }
    
    /// Get a value from a column family
    pub fn get<V>(&self, cf_name: &str, key: &[u8]) -> IndexerResult<Option<V>>
    where
        V: for<'de> Deserialize<'de>,
    {
        let cf = self.db
            .cf_handle(cf_name)
            .ok_or_else(|| IndexerError::Storage(StorageError::RocksDB(
                format!("Column family not found: {}", cf_name)
            )))?;
        
        let data = self.db
            .get_cf(&cf, key)
            .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        
        match data {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    
    /// Delete a key from a column family
    pub fn delete(&self, cf_name: &str, key: &[u8]) -> IndexerResult<()> {
        let cf = self.db
            .cf_handle(cf_name)
            .ok_or_else(|| IndexerError::Storage(StorageError::RocksDB(
                format!("Column family not found: {}", cf_name)
            )))?;
        
        self.db
            .delete_cf(&cf, key)
            .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        
        debug!("Deleted key from CF {}", cf_name);
        
        Ok(())
    }
    
    /// Check if database is healthy (can perform basic operations)
    pub fn health_check(&self) -> IndexerResult<()> {
        // Try to get a property to ensure DB is operational
        self.db
            .property_value("rocksdb.stats")
            .map_err(|e| IndexerError::Storage(StorageError::RocksDB(e.to_string())))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_rocksdb_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: false,
            max_open_files: 100,
            write_buffer_size_mb: 16,
            max_write_buffer_number: 2,
            block_cache_size_mb: 32,
        };
        
        let db = RocksDBClient::open(&config).await.unwrap();
        
        // Test put/get
        let test_data = vec![1u8, 2, 3, 4, 5];
        db.put(ColumnFamilies::MARKETS, b"test_key", &test_data).unwrap();
        
        let retrieved: Option<Vec<u8>> = db.get(ColumnFamilies::MARKETS, b"test_key").unwrap();
        assert_eq!(retrieved, Some(test_data));
        
        // Test delete
        db.delete(ColumnFamilies::MARKETS, b"test_key").unwrap();
        let after_delete: Option<Vec<u8>> = db.get(ColumnFamilies::MARKETS, b"test_key").unwrap();
        assert_eq!(after_delete, None);
        
        // Test health check
        assert!(db.health_check().is_ok());
    }
}

