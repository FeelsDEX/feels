//! Simple standalone test to verify basic functionality

use std::path::PathBuf;

#[test]
fn test_basic_math() {
    assert_eq!(2 + 2, 4);
}

#[test] 
fn test_sdk_types() {
    use feels_indexer::sdk_types::AccountType;
    
    // Test discriminator parsing
    let market_disc = [219, 190, 213, 55, 0, 227, 198, 154];
    let account_type = AccountType::from_discriminator(&market_disc);
    assert_eq!(account_type, Some(AccountType::Market));
    
    // Test unknown discriminator
    let unknown_disc = [0, 0, 0, 0, 0, 0, 0, 0];
    let account_type = AccountType::from_discriminator(&unknown_disc);
    assert_eq!(account_type, None);
}

#[test]
fn test_config_creation() {
    use feels_indexer::config::{IndexerConfig, PostgresConfig, RedisConfig, RocksDBConfig, ApiConfig};
    
    let config = IndexerConfig {
        geyser_endpoint: "http://localhost:10000".to_string(),
        program_id: "11111111111111111111111111111111".to_string(),
        postgres: PostgresConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            max_connections: 10,
        },
        redis: RedisConfig {
            url: "redis://localhost".to_string(),
            pool_size: 10,
            ttl_seconds: 3600,
        },
        rocksdb: RocksDBConfig {
            path: PathBuf::from("/tmp/test"),
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
        geyser: None,
    };
    
    assert_eq!(config.geyser_endpoint, "http://localhost:10000");
    assert_eq!(config.postgres.port, 5432);
}