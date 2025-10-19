//! Processor integration tests

use anyhow::Result;
use feels_indexer::config::RocksDBConfig;
use feels_indexer::database::{DatabaseManager, rocksdb::ColumnFamilies};
use feels_indexer::processors::{ProcessorRegistry, MarketProcessor, AccountProcessor};
use feels_indexer::models::BlockInfo;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::TempDir;
use std::path::PathBuf;

/// Create a test database manager and processor setup
async fn create_test_setup() -> Result<(Arc<DatabaseManager>, ProcessorRegistry, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 100,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    let db_manager = Arc::new(DatabaseManager::new_rocksdb_only(config).await?);
    let registry = ProcessorRegistry::new(db_manager.clone());
    
    Ok((db_manager, registry, temp_dir))
}

/// Create test market account data (simplified)
fn create_test_market_data() -> Vec<u8> {
    // This would normally be the actual serialized account data from Solana
    // For testing, we'll create a simple mock structure
    let mut data = vec![0u8; 1000]; // 1KB of test data
    
    // Add discriminator (8 bytes) - this would be the actual market discriminator
    data[0..8].copy_from_slice(&[219, 190, 213, 55, 0, 227, 198, 154]);
    
    // Add some mock data
    data[8..16].copy_from_slice(&1000000000000000000u64.to_le_bytes()); // Mock sqrt_price
    data[16..24].copy_from_slice(&500000000000000000u64.to_le_bytes()); // Mock liquidity
    data[24..28].copy_from_slice(&3000u32.to_le_bytes()); // Mock fee_bps
    
    data
}

/// Create test block info
fn create_test_block_info(slot: u64) -> BlockInfo {
    BlockInfo {
        slot,
        timestamp: chrono::Utc::now().timestamp(),
        block_height: Some(slot / 2), // Mock block height
    }
}

#[tokio::test]
async fn test_processor_registry_creation() -> Result<()> {
    let (db_manager, _registry, _temp_dir) = create_test_setup().await?;
    
    // Test that we can create a processor registry
    let _new_registry = ProcessorRegistry::new(db_manager);
    
    Ok(())
}

#[tokio::test]
async fn test_market_processor_account_update() -> Result<()> {
    let (db_manager, _registry, _temp_dir) = create_test_setup().await?;
    
    let processor = MarketProcessor::new(db_manager.clone());
    let pubkey = Pubkey::from_str("11111111111111111111111111111112")?;
    let data = create_test_market_data();
    let block_info = create_test_block_info(12345678);
    
    // Process the account update
    processor.process_account_update(pubkey, &data, block_info).await?;
    
    // Verify that something was stored
    let stored_market = db_manager.rocksdb.get_market(&pubkey.to_string())?;
    
    // We expect some data to be stored
    assert!(stored_market.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_processor_registry_routing() -> Result<()> {
    let (_db_manager, registry, _temp_dir) = create_test_setup().await?;
    
    let pubkey = Pubkey::from_str("11111111111111111111111111111112")?;
    let data = create_test_market_data();
    let slot = 12345678;
    
    // Test that the registry can route account updates
    registry.process_account_update(&pubkey, &data, slot).await?;
    
    // If we get here without errors, routing worked
    Ok(())
}

#[tokio::test]
async fn test_multiple_processor_types() -> Result<()> {
    let (_db_manager, registry, _temp_dir) = create_test_setup().await?;
    
    // Test processing different types of accounts
    let market_pubkey = Pubkey::from_str("MarketPubkey11111111111111111111")?;
    let position_pubkey = Pubkey::from_str("PositionPubkey1111111111111111111")?;
    
    let market_data = create_test_market_data();
    let position_data = vec![0u8; 500]; // Mock position data
    
    let slot = 12345678;
    
    // Process both account types
    registry.process_account_update(&market_pubkey, &market_data, slot).await?;
    registry.process_account_update(&position_pubkey, &position_data, slot).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_processor_error_handling() -> Result<()> {
    let (_db_manager, registry, _temp_dir) = create_test_setup().await?;
    
    let pubkey = Pubkey::from_str("11111111111111111111111111111112")?;
    let invalid_data = vec![0u8; 5]; // Too short to be valid
    let slot = 12345678;
    
    // Process invalid data - should not panic
    let result = registry.process_account_update(&pubkey, &invalid_data, slot).await;
    
    // We expect it to handle the error gracefully
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_processing() -> Result<()> {
    let (_db_manager, registry, _temp_dir) = create_test_setup().await?;
    let registry = Arc::new(registry);
    
    // Create multiple tasks to process accounts concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let pubkey = Pubkey::new_unique();
            let data = create_test_market_data();
            let slot = 12345678 + i;
            
            registry_clone.process_account_update(&pubkey, &data, slot).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}