//! Error types for the keeper service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeeperError {
    #[error("Invalid weights: {0}")]
    InvalidWeights(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Computation error: {0}")]
    ComputationError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<std::io::Error> for KeeperError {
    fn from(err: std::io::Error) -> Self {
        KeeperError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for KeeperError {
    fn from(err: serde_json::Error) -> Self {
        KeeperError::SerializationError(err.to_string())
    }
}