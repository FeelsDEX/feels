//! RocksDB storage layer with serde support

use anyhow::{anyhow, Result};
use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, MultiThreaded, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::config::RocksDBConfig;
use crate::models::{*, Transaction};
use crate::models::buffer::IndexedBuffer as Buffer;
use crate::models::floor::IndexedFloor as FloorLiquidity;
use crate::models::market::IndexedMarket as Market;
use crate::models::position::IndexedPosition as Position;
use super::Swap;

/// Type alias for the RocksDB instance
pub type RocksDB = DBWithThreadMode<MultiThreaded>;

/// Column family names for different data types
pub struct ColumnFamilies;

impl ColumnFamilies {
    pub const MARKETS: &'static str = "markets";
    pub const SWAPS: &'static str = "swaps";
    pub const POSITIONS: &'static str = "positions";
    pub const FLOOR_LIQUIDITY: &'static str = "floor_liquidity";
    pub const BUFFERS: &'static str = "buffers";
    pub const TRANSACTIONS: &'static str = "transactions";
    pub const BLOCKS: &'static str = "blocks";
    pub const SLOTS: &'static str = "slots";
    pub const METADATA: &'static str = "metadata";
    pub const ACCOUNTS: &'static str = "accounts";
    pub const SNAPSHOTS: &'static str = "snapshots";

    /// Get all column family names
    pub fn all() -> Vec<&'static str> {
        vec![
            Self::MARKETS,
            Self::SWAPS,
            Self::POSITIONS,
            Self::FLOOR_LIQUIDITY,
            Self::BUFFERS,
            Self::TRANSACTIONS,
            Self::BLOCKS,
            Self::SLOTS,
            Self::METADATA,
            Self::ACCOUNTS,
            Self::SNAPSHOTS,
        ]
    }
}

/// RocksDB storage manager with serde support
#[derive(Clone)]
pub struct RocksDBManager {
    db: Arc<RocksDB>,
    config: RocksDBConfig,
}

impl RocksDBManager {
    /// Create a new RocksDB manager
    pub async fn new(config: RocksDBConfig) -> Result<Self> {
        info!("Initializing RocksDB at path: {}", config.path.display());

        // Create database options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        
        // Performance tuning
        db_opts.set_max_open_files(config.max_open_files);
        db_opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
        db_opts.set_max_write_buffer_number(config.max_write_buffer_number);
        
        if config.enable_compression {
            db_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        }

        // Set up block cache
        let cache = rocksdb::Cache::new_lru_cache(config.block_cache_size_mb * 1024 * 1024);
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_block_cache(&cache);
        db_opts.set_block_based_table_factory(&block_opts);

        // Create column family descriptors
        let cf_descriptors: Vec<ColumnFamilyDescriptor> = ColumnFamilies::all()
            .into_iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                ColumnFamilyDescriptor::new(name, cf_opts)
            })
            .collect();

        // Open database with column families
        let db = RocksDB::open_cf_descriptors(&db_opts, &config.path, cf_descriptors)
            .map_err(|e| anyhow!("Failed to open RocksDB: {}", e))?;

        info!("RocksDB initialized successfully");

        Ok(Self {
            db: Arc::new(db),
            config,
        })
    }

    /// Get a column family handle
    fn get_cf(&self, name: &str) -> Result<Arc<rocksdb::BoundColumnFamily<'_>>> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| anyhow!("Column family '{}' not found", name))
    }

    /// Serialize a value using bincode
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        bincode::serialize(value).map_err(|e| anyhow!("Serialization failed: {}", e))
    }

    /// Deserialize a value using bincode
    fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        bincode::deserialize(data).map_err(|e| anyhow!("Deserialization failed: {}", e))
    }

    /// Put a serializable value into a column family
    pub fn put<T: Serialize>(&self, cf_name: &str, key: &[u8], value: &T) -> Result<()> {
        let cf = self.get_cf(cf_name)?;
        let serialized = self.serialize(value)?;
        self.db
            .put_cf(&cf, key, serialized)
            .map_err(|e| anyhow!("Failed to put value: {}", e))
    }
    
    /// Put raw bytes into a column family
    pub fn put_raw(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.get_cf(cf_name)?;
        self.db
            .put_cf(&cf, key, value)
            .map_err(|e| anyhow!("Failed to put value: {}", e))
    }

    /// Get and deserialize a value from a column family
    pub fn get<T: for<'de> Deserialize<'de>>(&self, cf_name: &str, key: &[u8]) -> Result<Option<T>> {
        let cf = self.get_cf(cf_name)?;
        match self.db.get_cf(&cf, key) {
            Ok(Some(data)) => Ok(Some(self.deserialize(&data)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow!("Failed to get value: {}", e)),
        }
    }

    /// Delete a key from a column family
    pub fn delete(&self, cf_name: &str, key: &[u8]) -> Result<()> {
        let cf = self.get_cf(cf_name)?;
        self.db
            .delete_cf(&cf, key)
            .map_err(|e| anyhow!("Failed to delete key: {}", e))
    }

    /// Check if a key exists in a column family
    pub fn exists(&self, cf_name: &str, key: &[u8]) -> Result<bool> {
        let cf = self.get_cf(cf_name)?;
        match self.db.get_cf(&cf, key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(anyhow!("Failed to check key existence: {}", e)),
        }
    }

    /// Create a write batch for atomic operations
    pub fn create_batch(&self) -> RocksDBBatch {
        RocksDBBatch {
            batch: WriteBatch::default(),
            manager: self.clone(),
        }
    }
    
    /// Batch write multiple key-value pairs to a column family
    pub fn batch_write(&self, cf_name: &str, items: HashMap<Vec<u8>, Vec<u8>>) -> Result<()> {
        let mut batch = self.create_batch();
        for (key, value) in items {
            batch.put_raw(cf_name, &key, &value)?;
        }
        batch.write()
    }
    
    /// Get an iterator with range bounds
    pub fn iter_range<'a>(
        &'a self, 
        cf_name: &str, 
        start: &[u8], 
        end: Vec<u8>
    ) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>> {
        let cf = self.get_cf(cf_name)?;
        let iter = self.db.iterator_cf(
            &cf,
            rocksdb::IteratorMode::From(start, rocksdb::Direction::Forward)
        );
        
        Ok(Box::new(
            iter.filter_map(move |result| {
                match result {
                    Ok((k, v)) => {
                        if k.as_ref() <= end.as_slice() {
                            Some((k.into_vec(), v.into_vec()))
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            })
        ))
    }
    
    /// Compact a range in a column family
    pub fn compact_range(
        &self, 
        cf_name: &str, 
        start: Option<&[u8]>, 
        end: Option<&[u8]>
    ) -> Result<()> {
        let cf = self.get_cf(cf_name)?;
        self.db.compact_range_cf(&cf, start, end);
        Ok(())
    }

    /// Get an iterator over a column family
    pub fn iter_cf(&self, cf_name: &str) -> Result<impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + '_> {
        let cf = self.get_cf(cf_name)?;
        Ok(self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start)
            .map(|result| result.expect("Failed to read from iterator")))
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        for cf_name in ColumnFamilies::all() {
            let cf = self.get_cf(cf_name)?;
            
            // Get approximate number of keys
            if let Ok(Some(count)) = self.db.property_value_cf(&cf, "rocksdb.estimate-num-keys") {
                stats.insert(format!("{}_keys", cf_name), count);
            }
            
            // Get approximate size
            if let Ok(Some(size)) = self.db.property_value_cf(&cf, "rocksdb.total-sst-files-size") {
                stats.insert(format!("{}_size_bytes", cf_name), size);
            }
        }
        
        Ok(stats)
    }

    /// Compact a column family
    pub fn compact_cf(&self, cf_name: &str) -> Result<()> {
        let cf = self.get_cf(cf_name)?;
        self.db.compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }

    /// Flush all column families
    pub fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .map_err(|e| anyhow!("Failed to flush database: {}", e))
    }
}

/// Write batch for atomic operations
pub struct RocksDBBatch {
    batch: WriteBatch,
    manager: RocksDBManager,
}

impl RocksDBBatch {
    /// Put a serializable value into the batch
    pub fn put<T: Serialize>(&mut self, cf_name: &str, key: &[u8], value: &T) -> Result<()> {
        let cf = self.manager.get_cf(cf_name)?;
        let serialized = self.manager.serialize(value)?;
        self.batch.put_cf(&cf, key, serialized);
        Ok(())
    }
    
    /// Put raw bytes into the batch
    pub fn put_raw(&mut self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.manager.get_cf(cf_name)?;
        self.batch.put_cf(&cf, key, value);
        Ok(())
    }

    /// Delete a key in the batch
    pub fn delete(&mut self, cf_name: &str, key: &[u8]) -> Result<()> {
        let cf = self.manager.get_cf(cf_name)?;
        self.batch.delete_cf(&cf, key);
        Ok(())
    }

    /// Write the batch atomically
    pub fn write(self) -> Result<()> {
        self.manager
            .db
            .write(self.batch)
            .map_err(|e| anyhow!("Failed to write batch: {}", e))
    }

    /// Clear the batch
    pub fn clear(&mut self) {
        self.batch.clear();
    }
}

// Convenience methods for specific data types
impl RocksDBManager {
    /// Store a market
    pub fn put_market(&self, market_id: &str, market: &Market) -> Result<()> {
        self.put(ColumnFamilies::MARKETS, market_id.as_bytes(), market)
    }

    /// Get a market
    pub fn get_market(&self, market_id: &str) -> Result<Option<Market>> {
        self.get(ColumnFamilies::MARKETS, market_id.as_bytes())
    }

    /// Store a swap
    pub fn put_swap(&self, swap_id: &str, swap: &Swap) -> Result<()> {
        self.put(ColumnFamilies::SWAPS, swap_id.as_bytes(), swap)
    }

    /// Get a swap
    pub fn get_swap(&self, swap_id: &str) -> Result<Option<Swap>> {
        self.get(ColumnFamilies::SWAPS, swap_id.as_bytes())
    }

    /// Store a position
    pub fn put_position(&self, position_id: &str, position: &Position) -> Result<()> {
        self.put(ColumnFamilies::POSITIONS, position_id.as_bytes(), position)
    }

    /// Get a position
    pub fn get_position(&self, position_id: &str) -> Result<Option<Position>> {
        self.get(ColumnFamilies::POSITIONS, position_id.as_bytes())
    }

    /// Store floor liquidity
    pub fn put_floor_liquidity(&self, floor_id: &str, floor: &FloorLiquidity) -> Result<()> {
        self.put(ColumnFamilies::FLOOR_LIQUIDITY, floor_id.as_bytes(), floor)
    }

    /// Get floor liquidity
    pub fn get_floor_liquidity(&self, floor_id: &str) -> Result<Option<FloorLiquidity>> {
        self.get(ColumnFamilies::FLOOR_LIQUIDITY, floor_id.as_bytes())
    }

    /// Store a buffer
    pub fn put_buffer(&self, buffer_id: &str, buffer: &Buffer) -> Result<()> {
        self.put(ColumnFamilies::BUFFERS, buffer_id.as_bytes(), buffer)
    }

    /// Get a buffer
    pub fn get_buffer(&self, buffer_id: &str) -> Result<Option<Buffer>> {
        self.get(ColumnFamilies::BUFFERS, buffer_id.as_bytes())
    }

    /// Store a transaction
    pub fn put_transaction(&self, tx_id: &str, transaction: &Transaction) -> Result<()> {
        self.put(ColumnFamilies::TRANSACTIONS, tx_id.as_bytes(), transaction)
    }

    /// Get a transaction by ID
    pub fn get_transaction_by_id(&self, tx_id: &str) -> Result<Option<Transaction>> {
        self.get(ColumnFamilies::TRANSACTIONS, tx_id.as_bytes())
    }

    /// Store block info
    pub fn put_block(&self, block_hash: &str, block: &BlockInfo) -> Result<()> {
        self.put(ColumnFamilies::BLOCKS, block_hash.as_bytes(), block)
    }

    /// Get block info
    pub fn get_block(&self, block_hash: &str) -> Result<Option<BlockInfo>> {
        self.get(ColumnFamilies::BLOCKS, block_hash.as_bytes())
    }

    /// Store slot info
    pub fn put_slot(&self, slot: u64, slot_info: &SlotInfo) -> Result<()> {
        self.put(ColumnFamilies::SLOTS, &slot.to_be_bytes(), slot_info)
    }

    /// Get slot info
    pub fn get_slot(&self, slot: u64) -> Result<Option<SlotInfo>> {
        self.get(ColumnFamilies::SLOTS, &slot.to_be_bytes())
    }

    /// Store metadata
    pub fn put_metadata(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        self.put(ColumnFamilies::METADATA, key.as_bytes(), value)
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Result<Option<serde_json::Value>> {
        self.get(ColumnFamilies::METADATA, key.as_bytes())
    }
    
    /// Store raw transaction data
    pub async fn store_transaction_raw(&self, signature: &str, data: &[u8], slot: u64) -> Result<()> {
        // Create a composite key: signature:slot
        let key = format!("{}:{}", signature, slot);
        self.put_raw(ColumnFamilies::TRANSACTIONS, key.as_bytes(), data)?;
        Ok(())
    }
    
    /// Get raw transaction data
    pub async fn get_transaction_raw(&self, signature: &str, slot: u64) -> Result<Option<Vec<u8>>> {
        let key = format!("{}:{}", signature, slot);
        let cf = self.get_cf(ColumnFamilies::TRANSACTIONS)?;
        match self.db.get_cf(&cf, key.as_bytes()) {
            Ok(Some(data)) => Ok(Some(data)),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow!("Failed to get transaction: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    

    fn create_test_config() -> (RocksDBConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 16,
            max_write_buffer_number: 2,
            block_cache_size_mb: 32,
        };
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_rocksdb_manager_creation() {
        let (config, _temp_dir) = create_test_config();
        let manager = RocksDBManager::new(config).await.unwrap();
        
        // Test that all column families exist
        for cf_name in ColumnFamilies::all() {
            assert!(manager.get_cf(cf_name).is_ok());
        }
    }

    #[tokio::test]
    async fn test_put_get_operations() {
        let (config, _temp_dir) = create_test_config();
        let manager = RocksDBManager::new(config).await.unwrap();

        // Test with a simple struct
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestData {
            id: u64,
            name: String,
        }

        let test_data = TestData {
            id: 123,
            name: "test".to_string(),
        };

        // Put and get
        manager.put("metadata", b"test_key", &test_data).unwrap();
        let retrieved: Option<TestData> = manager.get("metadata", b"test_key").unwrap();
        
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let (config, _temp_dir) = create_test_config();
        let manager = RocksDBManager::new(config).await.unwrap();

        let mut batch = manager.create_batch();
        
        // Add multiple operations to batch
        batch.put("metadata", b"key1", &"value1".to_string()).unwrap();
        batch.put("metadata", b"key2", &"value2".to_string()).unwrap();
        batch.put("metadata", b"key3", &"value3".to_string()).unwrap();
        
        // Write batch
        batch.write().unwrap();
        
        // Verify all values were written
        let val1: Option<String> = manager.get("metadata", b"key1").unwrap();
        let val2: Option<String> = manager.get("metadata", b"key2").unwrap();
        let val3: Option<String> = manager.get("metadata", b"key3").unwrap();
        
        assert_eq!(val1, Some("value1".to_string()));
        assert_eq!(val2, Some("value2".to_string()));
        assert_eq!(val3, Some("value3".to_string()));
    }
}
