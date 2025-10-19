//! Basic functionality tests for the feels-indexer

use anyhow::Result;
use feels_indexer::config::{IndexerConfig, PostgresConfig, RedisConfig, RocksDBConfig, ApiConfig, GeyserConfig};
use feels_indexer::database::{DatabaseManager, rocksdb::ColumnFamilies};
use tempfile::TempDir;
use std::path::PathBuf;

/// Create a test database manager with temporary directory
async fn create_test_db_manager() -> Result<(DatabaseManager, TempDir)> {
    let temp_dir = TempDir::new()?;
    let rocksdb_config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    // For tests, we'll only initialize RocksDB
    let db_manager = DatabaseManager::new_rocksdb_only(rocksdb_config).await?;
    Ok((db_manager, temp_dir))
}

#[tokio::test]
async fn test_storage_initialization() -> Result<()> {
    let (_db_manager, _temp_dir) = create_test_db_manager().await?;
    // If we get here without panicking, initialization worked
    Ok(())
}

#[tokio::test]
async fn test_basic_put_and_get() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    let key = b"test_key";
    let value = b"test_value";
    
    // Test put
    db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, value)?;
    
    // Test get
    let retrieved: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key)?;
    assert_eq!(retrieved, Some(value.to_vec()));
    
    // Test get non-existent key
    let missing: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, b"missing_key")?;
    assert_eq!(missing, None);
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_column_families() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    let key = b"test_key";
    let market_value = b"market_data";
    let swap_value = b"swap_data";
    
    // Store data in different column families
    db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, market_value)?;
    db_manager.rocksdb.put_raw(ColumnFamilies::SWAPS, key, swap_value)?;
    
    // Retrieve from different column families
    let retrieved_market: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key)?;
    let retrieved_swap: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::SWAPS, key)?;
    
    assert_eq!(retrieved_market, Some(market_value.to_vec()));
    assert_eq!(retrieved_swap, Some(swap_value.to_vec()));
    
    // Verify isolation - data in one CF doesn't affect another
    assert_ne!(retrieved_market, retrieved_swap);
    
    Ok(())
}

#[tokio::test]
async fn test_iterator_functionality() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    // Insert multiple key-value pairs
    let test_data = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"prefix_key1".to_vec(), b"prefix_value1".to_vec()),
        (b"prefix_key2".to_vec(), b"prefix_value2".to_vec()),
    ];
    
    for (key, value) in &test_data {
        db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, value)?;
    }
    
    // Test iteration
    let all_items: Vec<_> = db_manager.rocksdb.iter_cf(ColumnFamilies::MARKETS)?
        .map(|(k, v)| (k.to_vec(), v.to_vec()))
        .collect();
    assert_eq!(all_items.len(), 5);
    
    // Test iteration with range
    let prefix_items: Vec<_> = db_manager.rocksdb
        .iter_range(ColumnFamilies::MARKETS, b"prefix_", b"prefix_~".to_vec())?
        .collect();
    assert_eq!(prefix_items.len(), 2);
    
    // Verify prefix items contain correct keys
    for (key, _value) in prefix_items {
        assert!(key.starts_with(b"prefix_"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_serialization_functionality() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    // Test with a simple struct
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestData {
        id: u64,
        name: String,
    }
    
    let test_data = TestData {
        id: 123,
        name: "Hello, World!".to_string(),
    };
    
    // Store serialized data
    db_manager.rocksdb.put(ColumnFamilies::MARKETS, b"test_struct", &test_data)?;
    
    // Retrieve and deserialize
    let retrieved: Option<TestData> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, b"test_struct")?;
    assert_eq!(retrieved, Some(test_data));
    
    Ok(())
}

#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    // Create a batch
    let mut batch = db_manager.rocksdb.create_batch();
    
    // Add multiple operations to the batch
    for i in 0..10 {
        let key = format!("batch_key_{}", i);
        let value = format!("batch_value_{}", i);
        batch.put_raw(ColumnFamilies::MARKETS, key.as_bytes(), value.as_bytes())?;
    }
    
    // Write the batch
    batch.write()?;
    
    // Verify all values were written
    for i in 0..10 {
        let key = format!("batch_key_{}", i);
        let value = format!("batch_value_{}", i);
        let retrieved: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key.as_bytes())?;
        assert_eq!(retrieved, Some(value.as_bytes().to_vec()));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_delete_operation() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    let key = b"delete_test_key";
    let value = b"delete_test_value";
    
    // Put a value
    db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, value)?;
    
    // Verify it exists
    let exists_before: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key)?;
    assert_eq!(exists_before, Some(value.to_vec()));
    
    // Delete the key
    db_manager.rocksdb.delete(ColumnFamilies::MARKETS, key)?;
    
    // Verify it's gone
    let exists_after: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key)?;
    assert_eq!(exists_after, None);
    
    Ok(())
}

#[tokio::test]
async fn test_exists_operation() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    let key = b"exists_test_key";
    let value = b"exists_test_value";
    
    // Check non-existent key
    assert!(!db_manager.rocksdb.exists(ColumnFamilies::MARKETS, key)?);
    
    // Put a value
    db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, value)?;
    
    // Check it exists
    assert!(db_manager.rocksdb.exists(ColumnFamilies::MARKETS, key)?);
    
    Ok(())
}

#[tokio::test]
async fn test_large_data_handling() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    // Create a large value (1MB)
    let large_value = vec![0u8; 1024 * 1024];
    let key = b"large_key";
    
    // Store large value
    db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key, &large_value)?;
    
    // Retrieve and verify
    let retrieved: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, key)?;
    assert_eq!(retrieved, Some(large_value));
    
    Ok(())
}

#[tokio::test]
async fn test_storage_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();
    
    // Create first storage instance and store data
    {
        let rocksdb_config = RocksDBConfig {
            path: storage_path.clone(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let db_manager = DatabaseManager::new_rocksdb_only(rocksdb_config).await?;
        let test_key = b"persistence_test";
        let test_value = b"test_data_12345";
        
        db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, test_key, test_value)?;
    } // Storage instance dropped here
    
    // Create second storage instance and verify data persists
    {
        let rocksdb_config = RocksDBConfig {
            path: storage_path,
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let db_manager = DatabaseManager::new_rocksdb_only(rocksdb_config).await?;
        let test_key = b"persistence_test";
        
        let retrieved: Option<Vec<u8>> = db_manager.rocksdb.get(ColumnFamilies::MARKETS, test_key)?;
        assert_eq!(retrieved, Some(b"test_data_12345".to_vec()));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_database_stats() -> Result<()> {
    let (db_manager, _temp_dir) = create_test_db_manager().await?;
    
    // Add some data
    for i in 0..100 {
        let key = format!("stats_key_{}", i);
        let value = format!("stats_value_{}", i);
        db_manager.rocksdb.put_raw(ColumnFamilies::MARKETS, key.as_bytes(), value.as_bytes())?;
    }
    
    // Get stats
    let stats = db_manager.rocksdb.get_stats()?;
    
    // Verify we have some stats
    assert!(!stats.is_empty());
    
    Ok(())
}