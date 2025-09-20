//! Unit tests for RocksDB functionality

use anyhow::Result;
use feels_indexer::database::rocksdb::{RocksDBManager, ColumnFamilies};
use feels_indexer::config::RocksDBConfig;
use tempfile::TempDir;

/// Create a test RocksDB instance with temporary directory
async fn create_test_rocksdb() -> Result<(RocksDBManager, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    let rocksdb = RocksDBManager::new(config).await?;
    Ok((rocksdb, temp_dir))
}

#[tokio::test]
async fn test_rocksdb_initialization() -> Result<()> {
    let (_rocksdb, _temp_dir) = create_test_rocksdb().await?;
    // If we get here without panicking, initialization worked
    Ok(())
}

#[tokio::test]
async fn test_basic_put_and_get() -> Result<()> {
    let (rocksdb, _temp_dir) = create_test_rocksdb().await?;
    
    let key = b"test_key";
    let value = b"test_value";
    
    // Test put
    rocksdb.put_raw(ColumnFamilies::MARKETS, key, value)?;
    
    // Test get
    let retrieved: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::MARKETS, key)?;
    assert_eq!(retrieved, Some(value.to_vec()));
    
    // Test get non-existent key
    let missing: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::MARKETS, b"missing_key")?;
    assert_eq!(missing, None);
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_column_families() -> Result<()> {
    let (rocksdb, _temp_dir) = create_test_rocksdb().await?;
    
    let key = b"test_key";
    let market_value = b"market_data";
    let swap_value = b"swap_data";
    
    // Store data in different column families
    rocksdb.put_raw(ColumnFamilies::MARKETS, key, market_value)?;
    rocksdb.put_raw(ColumnFamilies::SWAPS, key, swap_value)?;
    
    // Retrieve from different column families
    let retrieved_market: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::MARKETS, key)?;
    let retrieved_swap: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::SWAPS, key)?;
    
    assert_eq!(retrieved_market, Some(market_value.to_vec()));
    assert_eq!(retrieved_swap, Some(swap_value.to_vec()));
    
    // Verify isolation - data in one CF doesn't affect another
    assert_ne!(retrieved_market, retrieved_swap);
    
    Ok(())
}

#[tokio::test]
async fn test_serialization_functionality() -> Result<()> {
    let (rocksdb, _temp_dir) = create_test_rocksdb().await?;
    
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestData {
        id: u64,
        name: String,
    }
    
    let test_data = TestData {
        id: 42,
        name: "test".to_string(),
    };
    
    // Test put with serialization
    rocksdb.put(ColumnFamilies::METADATA, b"test_key", &test_data)?;
    
    // Test get with deserialization
    let retrieved: Option<TestData> = rocksdb.get(ColumnFamilies::METADATA, b"test_key")?;
    assert_eq!(retrieved, Some(test_data));
    
    Ok(())
}

#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    let (rocksdb, _temp_dir) = create_test_rocksdb().await?;
    
    let mut batch = rocksdb.create_batch();
    
    // Add multiple operations to batch
    batch.put_raw(ColumnFamilies::METADATA, b"key1", b"value1")?;
    batch.put_raw(ColumnFamilies::METADATA, b"key2", b"value2")?;
    batch.put_raw(ColumnFamilies::METADATA, b"key3", b"value3")?;
    
    // Write batch
    batch.write()?;
    
    // Verify all values were written
    let val1: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::METADATA, b"key1")?;
    let val2: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::METADATA, b"key2")?;
    let val3: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::METADATA, b"key3")?;
    
    assert_eq!(val1, Some(b"value1".to_vec()));
    assert_eq!(val2, Some(b"value2".to_vec()));
    assert_eq!(val3, Some(b"value3".to_vec()));
    
    Ok(())
}

#[tokio::test]
async fn test_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().to_path_buf();
    
    // First instance - write data
    {
        let config = RocksDBConfig {
            path: path.clone(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let rocksdb = RocksDBManager::new(config).await?;
        rocksdb.put_raw(ColumnFamilies::MARKETS, b"persist_key", b"persist_value")?;
    }
    
    // Second instance - read data
    {
        let config = RocksDBConfig {
            path,
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let rocksdb = RocksDBManager::new(config).await?;
        let value: Option<Vec<u8>> = rocksdb.get(ColumnFamilies::MARKETS, b"persist_key")?;
        assert_eq!(value, Some(b"persist_value".to_vec()));
    }
    
    Ok(())
}