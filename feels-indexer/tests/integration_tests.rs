//! End-to-end integration tests

use anyhow::Result;
use feels_indexer::config::{IndexerConfig, PostgresConfig, RedisConfig, RocksDBConfig, ApiConfig, GeyserConfig};
use feels_indexer::database::DatabaseManager;
use feels_indexer::processors::ProcessorRegistry;
use feels_indexer::api::{ApiState, create_router};
use axum::{body::Body, http::{Request, StatusCode}};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;
use std::path::PathBuf;

/// Create a complete test environment
async fn create_test_environment() -> Result<(IndexerConfig, Arc<DatabaseManager>, ProcessorRegistry, TempDir)> {
    let temp_dir = TempDir::new()?;
    
    let config = IndexerConfig {
        geyser_endpoint: "http://localhost:10000".to_string(),
        program_id: "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N".to_string(),
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            max_connections: 10,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            ttl_seconds: 3600,
        },
        rocksdb: RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        },
        api: ApiConfig {
            bind_address: "127.0.0.1:8080".to_string(),
            metrics_port: 9090,
            max_page_size: 100,
            cors_enabled: true,
        },
        geyser: Some(GeyserConfig {
            endpoint: "http://localhost:10000".to_string(),
            commitment: "confirmed".to_string(),
        }),
    };
    
    let db_manager = Arc::new(DatabaseManager::new_rocksdb_only(config.rocksdb.clone()).await?);
    let registry = ProcessorRegistry::new(db_manager.clone());
    
    Ok((config, db_manager, registry, temp_dir))
}

/// Create mock market data
fn create_mock_market_data() -> Vec<u8> {
    let mut data = vec![0u8; 1000];
    // Add market discriminator
    data[0..8].copy_from_slice(&[219, 190, 213, 55, 0, 227, 198, 154]);
    // Add some mock data
    data[8..16].copy_from_slice(&1000000000000000000u64.to_le_bytes());
    data[16..24].copy_from_slice(&500000000000000000u64.to_le_bytes());
    data[24..28].copy_from_slice(&3000u32.to_le_bytes());
    data
}

#[tokio::test]
async fn test_full_system_initialization() -> Result<()> {
    let (_config, _db_manager, _registry, _temp_dir) = create_test_environment().await?;
    
    // If we get here without panicking, the full system initialized successfully
    Ok(())
}

#[tokio::test]
async fn test_data_flow_account_to_api() -> Result<()> {
    let (_config, db_manager, registry, _temp_dir) = create_test_environment().await?;
    
    // Simulate account update processing
    let market_pubkey = Pubkey::new_unique();
    let mock_account_data = create_mock_market_data();
    let slot = 12345678;
    
    // Process account update through the registry
    registry.process_account_update(&market_pubkey, &mock_account_data, slot).await?;
    
    // Verify data was stored
    let stored = db_manager.rocksdb.get_market(&market_pubkey.to_string())?;
    assert!(stored.is_some());
    
    // Create API and test data retrieval
    let api_state = ApiState { 
        db_manager: db_manager.clone()
    };
    let app = create_router(api_state);
    
    let request = Request::builder()
        .uri("/api/v1/markets")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_processing_and_api_access() -> Result<()> {
    let (_config, db_manager, registry, _temp_dir) = create_test_environment().await?;
    let registry = Arc::new(registry);
    
    // Create API
    let api_state = ApiState { 
        db_manager: db_manager.clone() 
    };
    let app = create_router(api_state);
    
    // Start concurrent processing
    let mut processing_handles = vec![];
    for i in 0..10 {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let pubkey = Pubkey::new_unique();
            let data = create_mock_market_data();
            let slot = 12345678 + i;
            
            registry_clone.process_account_update(&pubkey, &data, slot).await
        });
        processing_handles.push(handle);
    }
    
    // While processing is happening, make concurrent API requests
    let mut api_handles = vec![];
    for _ in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/v1/markets")
                .body(Body::empty())?;
            
            let response = app_clone.oneshot(request).await?;
            assert_eq!(response.status(), StatusCode::OK);
            Ok::<(), anyhow::Error>(())
        });
        api_handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in processing_handles {
        handle.await??;
    }
    for handle in api_handles {
        handle.await??;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_health_check_system() -> Result<()> {
    let (_config, db_manager, _registry, _temp_dir) = create_test_environment().await?;
    
    // Check database health
    let health = db_manager.health_check().await?;
    assert!(health.rocksdb); // RocksDB should always be healthy if initialized
    
    // Create API and check health endpoint
    let api_state = ApiState { 
        db_manager: db_manager.clone() 
    };
    let app = create_router(api_state);
    
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    assert!(body_str.contains("rocksdb"));
    
    Ok(())
}

#[tokio::test]
async fn test_data_persistence() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let market_address = "TestMarket111111111111111111111111";
    
    // First instance - store data
    {
        let config = RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let db_manager = DatabaseManager::new_rocksdb_only(config).await?;
        let registry = ProcessorRegistry::new(Arc::new(db_manager.clone()));
        
        let pubkey = Pubkey::from_str(market_address)?;
        let data = create_mock_market_data();
        
        registry.process_account_update(&pubkey, &data, 12345678).await?;
    }
    
    // Second instance - verify data persists
    {
        let config = RocksDBConfig {
            path: temp_dir.path().to_path_buf(),
            enable_compression: true,
            max_open_files: 100,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 3,
            block_cache_size_mb: 256,
        };
        
        let db_manager = DatabaseManager::new_rocksdb_only(config).await?;
        let stored = db_manager.rocksdb.get_market(market_address)?;
        
        assert!(stored.is_some());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_recovery() -> Result<()> {
    let (_config, db_manager, registry, _temp_dir) = create_test_environment().await?;
    
    // Process invalid data - should not crash
    let pubkey = Pubkey::new_unique();
    let invalid_data = vec![0u8; 5]; // Too short to be valid
    
    let result = registry.process_account_update(&pubkey, &invalid_data, 12345678).await;
    assert!(result.is_ok()); // Should handle error gracefully
    
    // API should still work
    let api_state = ApiState { 
        db_manager: db_manager.clone() 
    };
    let app = create_router(api_state);
    
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    Ok(())
}