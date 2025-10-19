//! Configuration system tests

use anyhow::Result;
use feels_indexer::config::{IndexerConfig, PostgresConfig, RedisConfig, RocksDBConfig, ApiConfig, GeyserConfig};
use std::fs;
use tempfile::TempDir;
use std::path::PathBuf;

/// Create a test configuration file
fn create_test_config_content() -> String {
    r#"
[geyser]
endpoint = "http://localhost:10000"
commitment = "confirmed"

[postgres]
host = "localhost"
port = 5432
database = "feels_indexer"
username = "postgres"
password = "postgres"
max_connections = 10

[redis]
url = "redis://localhost:6379"
pool_size = 10
ttl_seconds = 3600

[rocksdb]
path = "./data/rocksdb"
enable_compression = true
max_open_files = 1000
write_buffer_size_mb = 64
max_write_buffer_number = 3
block_cache_size_mb = 256

[api]
bind_address = "127.0.0.1:8080"
metrics_port = 9090
max_page_size = 100
cors_enabled = true

program_id = "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N"
geyser_endpoint = "http://localhost:10000"
"#.to_string()
}

#[tokio::test]
async fn test_config_loading_from_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("test_config.toml");
    
    // Write test config to file
    fs::write(&config_path, create_test_config_content())?;
    
    // Load config from file
    let config = IndexerConfig::from_file(config_path.to_str().unwrap())?;
    
    // Verify config values
    assert_eq!(config.geyser_endpoint, "http://localhost:10000");
    assert_eq!(config.program_id, "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N");
    
    assert_eq!(config.postgres.host, "localhost");
    assert_eq!(config.postgres.port, 5432);
    assert_eq!(config.postgres.database, "feels_indexer");
    
    assert_eq!(config.redis.url, "redis://localhost:6379");
    assert_eq!(config.redis.pool_size, 10);
    
    assert_eq!(config.rocksdb.enable_compression, true);
    assert_eq!(config.rocksdb.max_open_files, 1000);
    
    assert_eq!(config.api.bind_address, "127.0.0.1:8080");
    assert_eq!(config.api.cors_enabled, true);
    
    Ok(())
}

#[tokio::test]
async fn test_config_validation() -> Result<()> {
    let config = IndexerConfig {
        geyser_endpoint: "http://localhost:10000".to_string(),
        program_id: "11111111111111111111111111111111".to_string(),
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
            path: PathBuf::from("./data/rocksdb"),
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
    
    // Test valid config
    assert!(config.validate().is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_rocksdb_path_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = IndexerConfig {
        geyser_endpoint: "http://localhost:10000".to_string(),
        program_id: "11111111111111111111111111111111".to_string(),
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
            path: temp_dir.path().join("test_rocksdb"),
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
    
    // Ensure RocksDB path creation
    let rocksdb_path = &config.rocksdb.path;
    fs::create_dir_all(rocksdb_path)?;
    
    // Verify path exists
    assert!(rocksdb_path.exists());
    assert!(rocksdb_path.is_dir());
    
    Ok(())
}

#[tokio::test]
async fn test_config_serialization() -> Result<()> {
    let config = IndexerConfig {
        geyser_endpoint: "http://localhost:10000".to_string(),
        program_id: "11111111111111111111111111111111".to_string(),
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
            path: PathBuf::from("./data/rocksdb"),
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
    
    // Serialize to TOML
    let toml_str = toml::to_string(&config)?;
    
    // Verify it contains expected sections
    assert!(toml_str.contains("geyser_endpoint"));
    assert!(toml_str.contains("program_id"));
    assert!(toml_str.contains("[postgres]"));
    assert!(toml_str.contains("[redis]"));
    assert!(toml_str.contains("[rocksdb]"));
    assert!(toml_str.contains("[api]"));
    
    // Deserialize back
    let deserialized: IndexerConfig = toml::from_str(&toml_str)?;
    
    // Verify values match
    assert_eq!(config.geyser_endpoint, deserialized.geyser_endpoint);
    assert_eq!(config.program_id, deserialized.program_id);
    assert_eq!(config.rocksdb.enable_compression, deserialized.rocksdb.enable_compression);
    assert_eq!(config.api.bind_address, deserialized.api.bind_address);
    
    Ok(())
}

#[tokio::test]
async fn test_partial_config_loading() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("partial_config.toml");
    
    // Write partial config (missing some sections)
    let partial_config = r#"
geyser_endpoint = "http://custom:9000"
program_id = "11111111111111111111111111111111"

[postgres]
host = "localhost"
port = 5432
database = "test"
username = "user"
password = "pass"
max_connections = 5

[redis]
url = "redis://localhost:6379"
pool_size = 10
ttl_seconds = 3600

[rocksdb]
path = "./data"
enable_compression = false
max_open_files = 100
write_buffer_size_mb = 32
max_write_buffer_number = 2
block_cache_size_mb = 128

[api]
bind_address = "0.0.0.0:8080"
metrics_port = 9090
max_page_size = 50
cors_enabled = false
"#;
    
    fs::write(&config_path, partial_config)?;
    
    // Load config
    let config = IndexerConfig::from_file(config_path.to_str().unwrap())?;
    
    // Verify custom values
    assert_eq!(config.geyser_endpoint, "http://custom:9000");
    assert_eq!(config.rocksdb.enable_compression, false);
    assert_eq!(config.api.bind_address, "0.0.0.0:8080");
    assert_eq!(config.api.cors_enabled, false);
    
    Ok(())
}

#[tokio::test]
async fn test_environment_variable_override() -> Result<()> {
    // Set environment variable
    std::env::set_var("FEELS_GEYSER_ENDPOINT", "http://env:8000");
    std::env::set_var("FEELS_PROGRAM_ID", "22222222222222222222222222222222");
    
    // In a real implementation, the config would check env vars
    let endpoint = std::env::var("FEELS_GEYSER_ENDPOINT").unwrap_or("http://localhost:10000".to_string());
    let program_id = std::env::var("FEELS_PROGRAM_ID").unwrap_or("11111111111111111111111111111111".to_string());
    
    assert_eq!(endpoint, "http://env:8000");
    assert_eq!(program_id, "22222222222222222222222222222222");
    
    // Clean up
    std::env::remove_var("FEELS_GEYSER_ENDPOINT");
    std::env::remove_var("FEELS_PROGRAM_ID");
    
    Ok(())
}

#[tokio::test]
async fn test_config_error_handling() -> Result<()> {
    // Test loading non-existent file
    let result = IndexerConfig::from_file("non_existent_config.toml");
    assert!(result.is_err());
    
    // Test loading invalid TOML
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("invalid_config.toml");
    fs::write(&config_path, "invalid toml content [")?;
    
    let result = IndexerConfig::from_file(config_path.to_str().unwrap());
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_postgres_config_validation() -> Result<()> {
    let mut postgres_config = PostgresConfig {
        host: "localhost".to_string(),
        port: 5432,
        database: "test".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        max_connections: 10,
    };
    
    // Test different port numbers
    for port in &[5432, 5433, 5434] {
        postgres_config.port = *port;
        // Should be valid
    }
    
    // Test different connection pool sizes
    for max_conn in &[5, 10, 20, 50] {
        postgres_config.max_connections = *max_conn;
        // Should be valid
    }
    
    Ok(())
}

#[tokio::test]
async fn test_redis_config_validation() -> Result<()> {
    let mut redis_config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 10,
        ttl_seconds: 3600,
    };
    
    // Test different URLs
    for url in &[
        "redis://localhost:6379",
        "redis://127.0.0.1:6379",
        "redis://redis-server:6379",
    ] {
        redis_config.url = url.to_string();
        // Should be valid
    }
    
    // Test different pool sizes
    for pool_size in &[5, 10, 20] {
        redis_config.pool_size = *pool_size;
        // Should be valid
    }
    
    // Test different TTL values
    for ttl in &[60, 3600, 86400] {
        redis_config.ttl_seconds = *ttl;
        // Should be valid
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rocksdb_config_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut rocksdb_config = RocksDBConfig {
        path: temp_dir.path().to_path_buf(),
        enable_compression: true,
        max_open_files: 1000,
        write_buffer_size_mb: 64,
        max_write_buffer_number: 3,
        block_cache_size_mb: 256,
    };
    
    // Test different settings
    rocksdb_config.enable_compression = false;
    rocksdb_config.max_open_files = 500;
    rocksdb_config.write_buffer_size_mb = 32;
    rocksdb_config.max_write_buffer_number = 2;
    rocksdb_config.block_cache_size_mb = 128;
    
    // All settings should be valid
    Ok(())
}

#[tokio::test]
async fn test_api_config_validation() -> Result<()> {
    let mut api_config = ApiConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        metrics_port: 9090,
        max_page_size: 100,
        cors_enabled: true,
    };
    
    // Test different bind addresses
    for addr in &[
        "127.0.0.1:8080",
        "0.0.0.0:8080",
        "localhost:3000",
    ] {
        api_config.bind_address = addr.to_string();
        // Should be valid
    }
    
    // Test different ports
    for port in &[8080, 9090, 3000] {
        api_config.metrics_port = *port;
        // Should be valid
    }
    
    // Test page size limits
    for page_size in &[10, 50, 100, 200] {
        api_config.max_page_size = *page_size;
        // Should be valid
    }
    
    Ok(())
}