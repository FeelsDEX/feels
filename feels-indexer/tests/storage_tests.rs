//! Storage layer integration tests

use anyhow::Result;
use feels_indexer::config::StorageConfig;
use feels_indexer::storage::{FeelsStorage, Storage, ColumnFamilies};
use feels_indexer::models::{IndexedMarket, PoolPhase};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tempfile::TempDir;

/// Create a test storage instance with temporary directory
async fn create_test_storage() -> Result<(FeelsStorage, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = StorageConfig {
        rocksdb_path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    let storage = FeelsStorage::new(&config).await?;
    Ok((storage, temp_dir))
}

/// Create a test market for testing
fn create_test_market() -> IndexedMarket {
    use feels_indexer::models::BlockInfo;
    
    IndexedMarket {
        address: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        token_0: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
        token_1: Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
        sqrt_price: 1000000000000000000u128,
        liquidity: 1000000u128,
        current_tick: 0,
        tick_spacing: 64,
        fee_bps: 300, // 0.3%
        is_paused: false,
        phase: PoolPhase::PriceDiscovery,
        global_lower_tick: -887272,
        global_upper_tick: 887272,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
        last_updated: BlockInfo {
            slot: 12345678,
            timestamp: 1640995200,
            block_height: Some(12345000),
        },
        total_volume_0: 5000000000u128,
        total_volume_1: 1000000000u128,
        total_fees_0: 15000000u128,
        total_fees_1: 3000000u128,
        swap_count: 1000,
        unique_traders: 250,
    }
}

#[tokio::test]
async fn test_storage_initialization() -> Result<()> {
    let (_storage, _temp_dir) = create_test_storage().await?;
    // If we get here without panicking, initialization worked
    Ok(())
}

#[tokio::test]
async fn test_put_and_get() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    let key = b"test_key";
    let value = b"test_value";
    
    // Test put
    storage.put(ColumnFamilies::MARKETS, key, value).await?;
    
    // Test get
    let retrieved = storage.get(ColumnFamilies::MARKETS, key).await?;
    assert_eq!(retrieved, Some(value.to_vec()));
    
    // Test get non-existent key
    let missing = storage.get(ColumnFamilies::MARKETS, b"missing_key").await?;
    assert_eq!(missing, None);
    
    Ok(())
}

#[tokio::test]
async fn test_market_serialization() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    let market = create_test_market();
    
    // Serialize market
    let serialized = FeelsStorage::serialize(&market)?;
    let key = market.address.to_bytes();
    
    // Store market
    storage.put(ColumnFamilies::MARKETS, &key, &serialized).await?;
    
    // Retrieve and deserialize
    let retrieved_data = storage.get(ColumnFamilies::MARKETS, &key).await?;
    assert!(retrieved_data.is_some());
    
    let deserialized: IndexedMarket = FeelsStorage::deserialize(&retrieved_data.unwrap())?;
    
    // Verify all fields match
    assert_eq!(deserialized.address, market.address);
    assert_eq!(deserialized.token_0, market.token_0);
    assert_eq!(deserialized.token_1, market.token_1);
    assert_eq!(deserialized.phase, market.phase);
    assert_eq!(deserialized.sqrt_price, market.sqrt_price);
    assert_eq!(deserialized.current_tick, market.current_tick);
    assert_eq!(deserialized.liquidity, market.liquidity);
    assert_eq!(deserialized.fee_bps, market.fee_bps);
    assert_eq!(deserialized.total_volume_0, market.total_volume_0);
    
    Ok(())
}

#[tokio::test]
async fn test_iterator() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    // Insert multiple key-value pairs
    let test_data = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"prefix_key1".to_vec(), b"prefix_value1".to_vec()),
        (b"prefix_key2".to_vec(), b"prefix_value2".to_vec()),
    ];
    
    for (key, value) in &test_data {
        storage.put(ColumnFamilies::MARKETS, key, value).await?;
    }
    
    // Test iteration without prefix
    let all_items: Vec<_> = storage.iter(ColumnFamilies::MARKETS, None).await?.collect();
    assert_eq!(all_items.len(), 5);
    
    // Test iteration with prefix
    let prefix_items: Vec<_> = storage.iter(ColumnFamilies::MARKETS, Some(b"prefix_")).await?.collect();
    assert_eq!(prefix_items.len(), 2);
    
    // Verify prefix items contain correct keys
    for (key, _value) in prefix_items {
        assert!(key.starts_with(b"prefix_"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_column_families() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    let key = b"test_key";
    let market_value = b"market_data";
    let swap_value = b"swap_data";
    
    // Store data in different column families
    storage.put(ColumnFamilies::MARKETS, key, market_value).await?;
    storage.put(ColumnFamilies::SWAPS, key, swap_value).await?;
    
    // Retrieve from different column families
    let retrieved_market = storage.get(ColumnFamilies::MARKETS, key).await?;
    let retrieved_swap = storage.get(ColumnFamilies::SWAPS, key).await?;
    
    assert_eq!(retrieved_market, Some(market_value.to_vec()));
    assert_eq!(retrieved_swap, Some(swap_value.to_vec()));
    
    // Verify isolation - data in one CF doesn't affect another
    assert_ne!(retrieved_market, retrieved_swap);
    
    Ok(())
}

#[tokio::test]
async fn test_large_data_handling() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    // Create a large value (1MB)
    let large_value = vec![0u8; 1024 * 1024];
    let key = b"large_key";
    
    // Store large value
    storage.put(ColumnFamilies::MARKETS, key, &large_value).await?;
    
    // Retrieve and verify
    let retrieved = storage.get(ColumnFamilies::MARKETS, key).await?;
    assert_eq!(retrieved, Some(large_value));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    // Create multiple concurrent operations
    let mut handles = vec![];
    
    for i in 0..10 {
        let storage_clone = storage.clone();
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i);
            let value = format!("concurrent_value_{}", i);
            
            storage_clone.put(ColumnFamilies::MARKETS, key.as_bytes(), value.as_bytes()).await?;
            
            let retrieved = storage_clone.get(ColumnFamilies::MARKETS, key.as_bytes()).await?;
            assert_eq!(retrieved, Some(value.as_bytes().to_vec()));
            
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let (storage, _temp_dir) = create_test_storage().await?;
    
    // Test with invalid column family (this should be handled gracefully)
    let result = storage.get("invalid_cf", b"test_key").await;
    assert!(result.is_err());
    
    Ok(())
}
