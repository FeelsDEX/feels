//! Centralized error types for the Feels indexer

use thiserror::Error;

/// Main indexer error type
#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Account not found: {address}")]
    AccountNotFound { address: String },
    
    #[error("Invalid account data for {account_type}: {reason}")]
    InvalidAccountData {
        account_type: String,
        reason: String,
    },
    
    #[error("Processing error: {0}")]
    Processing(String),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Connection pool exhausted")]
    PoolExhausted,
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Search index error: {0}")]
    SearchIndex(String),
    
    #[error("RocksDB error: {0}")]
    RocksDB(String),
}

/// Network-specific errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("Geyser stream error: {0}")]
    GeyserStream(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
    
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
}

/// Result type alias for indexer operations
pub type IndexerResult<T> = Result<T, IndexerError>;

/// Helper to convert sqlx errors
impl From<sqlx::Error> for IndexerError {
    fn from(err: sqlx::Error) -> Self {
        IndexerError::Storage(StorageError::Database(err.to_string()))
    }
}

/// Helper to convert redis errors
impl From<redis::RedisError> for IndexerError {
    fn from(err: redis::RedisError) -> Self {
        IndexerError::Storage(StorageError::Cache(err.to_string()))
    }
}

/// Helper to convert rocksdb errors
impl From<rocksdb::Error> for IndexerError {
    fn from(err: rocksdb::Error) -> Self {
        IndexerError::Storage(StorageError::RocksDB(err.to_string()))
    }
}

/// Helper to convert serialization errors
impl From<bincode::Error> for IndexerError {
    fn from(err: bincode::Error) -> Self {
        IndexerError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for IndexerError {
    fn from(err: serde_json::Error) -> Self {
        IndexerError::Serialization(err.to_string())
    }
}

