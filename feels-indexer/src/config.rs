//! Modern configuration management for the Feels indexer

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[derive(Default)]
pub struct IndexerConfig {
    pub geyser: GeyserConfig,
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub search: SearchConfig,
    pub api: ApiConfig,
    pub monitoring: MonitoringConfig,
    pub indexer: IndexerSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GeyserConfig {
    #[validate(url)]
    pub endpoint: String,
    pub program_id: String,
    pub commitment: String,
    #[validate(range(min = 1, max = 100))]
    pub max_reconnect_attempts: u32,
    #[validate(range(min = 1, max = 300))]
    pub reconnect_delay_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StorageConfig {
    pub rocksdb: RocksDBConfig,
    pub tantivy_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RocksDBConfig {
    pub path: PathBuf,
    pub enable_compression: bool,
    #[validate(range(min = 100, max = 10000))]
    pub max_open_files: i32,
    #[validate(range(min = 64, max = 2048))]
    pub write_buffer_size_mb: usize,
    #[validate(range(min = 2, max = 16))]
    pub max_write_buffer_number: i32,
    #[validate(range(min = 128, max = 4096))]
    pub block_cache_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DatabaseConfig {
    #[validate(url)]
    pub postgres_url: String,
    #[validate(range(min = 1, max = 100))]
    pub max_connections: u32,
    #[validate(range(min = 1, max = 50))]
    pub min_connections: u32,
    #[validate(range(min = 5, max = 300))]
    pub acquire_timeout_secs: u64,
    #[validate(range(min = 60, max = 3600))]
    pub idle_timeout_secs: u64,
    #[validate(range(min = 300, max = 7200))]
    pub max_lifetime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RedisConfig {
    #[validate(url)]
    pub url: String,
    #[validate(range(min = 1, max = 50))]
    pub max_connections: u32,
    #[validate(range(min = 1, max = 30))]
    pub connection_timeout_secs: u64,
    #[validate(range(min = 1, max = 30))]
    pub response_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SearchConfig {
    pub index_path: PathBuf,
    #[validate(range(min = 64, max = 1024))]
    pub writer_memory_mb: usize,
    #[validate(range(min = 10, max = 300))]
    pub commit_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ApiConfig {
    pub bind_address: String,
    pub enable_cors: bool,
    #[validate(range(min = 5, max = 300))]
    pub request_timeout_secs: u64,
    #[validate(range(min = 1, max = 100))]
    pub max_request_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MonitoringConfig {
    #[validate(range(min = 1024, max = 65535))]
    pub metrics_port: u16,
    pub log_level: String,
    pub structured_logging: bool,
    #[validate(url)]
    pub jaeger_endpoint: String,
    pub enable_tracing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct IndexerSettings {
    #[validate(range(min = 100, max = 10000))]
    pub batch_size: usize,
    #[validate(range(min = 1, max = 60))]
    pub flush_interval_secs: u64,
    #[validate(range(min = 10, max = 1000))]
    pub max_lag_slots: u64,
    pub enable_backfill: bool,
    #[validate(range(min = 1000, max = 50000))]
    pub backfill_batch_size: usize,
}


impl Default for GeyserConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:10000".to_string(),
            program_id: "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N".to_string(),
            commitment: "confirmed".to_string(),
            max_reconnect_attempts: 10,
            reconnect_delay_secs: 5,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            rocksdb: RocksDBConfig::default(),
            tantivy_path: "./data/tantivy".into(),
        }
    }
}

impl Default for RocksDBConfig {
    fn default() -> Self {
        Self {
            path: "./data/rocksdb".into(),
            enable_compression: true,
            max_open_files: 1000,
            write_buffer_size_mb: 256,
            max_write_buffer_number: 4,
            block_cache_size_mb: 512,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgresql://feels:feels@localhost:5432/feels_indexer".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            max_connections: 10,
            connection_timeout_secs: 5,
            response_timeout_secs: 2,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_path: "./data/tantivy".into(),
            writer_memory_mb: 128,
            commit_interval_secs: 30,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            enable_cors: true,
            request_timeout_secs: 30,
            max_request_size_mb: 10,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_port: 9090,
            log_level: "info".to_string(),
            structured_logging: true,
            jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            enable_tracing: true,
        }
    }
}

impl Default for IndexerSettings {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            flush_interval_secs: 10,
            max_lag_slots: 100,
            enable_backfill: true,
            backfill_batch_size: 5000,
        }
    }
}

impl IndexerConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }

    /// Ensure required directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.storage.rocksdb.path)?;
        std::fs::create_dir_all(&self.storage.tantivy_path)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Basic validation - check required fields
        if self.geyser.endpoint.is_empty() {
            return Err(anyhow::anyhow!("Geyser endpoint cannot be empty"));
        }
        if self.geyser.program_id.is_empty() {
            return Err(anyhow::anyhow!("Program ID cannot be empty"));
        }
        Ok(())
    }
}