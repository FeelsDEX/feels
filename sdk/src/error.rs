//! SDK error types

use thiserror::Error;

/// SDK error type
#[derive(Error, Debug)]
pub enum SdkError {
    /// Anchor client error
    #[error("Anchor client error: {0}")]
    AnchorClient(String),

    /// RPC error
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Invalid route
    #[error("Invalid route: {0}")]
    InvalidRoute(String),

    /// Market not found
    #[error("Market not found: {0}")]
    MarketNotFound(String),

    /// Insufficient balance
    #[error("Insufficient balance: expected {expected}, available {available}")]
    InsufficientBalance { expected: u64, available: u64 },

    /// Slippage exceeded
    #[error("Slippage exceeded: expected {expected}, actual {actual}")]
    SlippageExceeded { expected: u64, actual: u64 },

    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Deserialization error
    #[error("Failed to deserialize account: {0}")]
    DeserializationError(String),

    /// Serialization error
    #[error("Failed to serialize data: {0}")]
    SerializationError(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Other error
    #[error("SDK error: {0}")]
    Other(String),
}

impl From<anchor_client::ClientError> for SdkError {
    fn from(err: anchor_client::ClientError) -> Self {
        SdkError::AnchorClient(err.to_string())
    }
}

impl From<solana_client::client_error::ClientError> for SdkError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        SdkError::Rpc(err.to_string())
    }
}

impl From<std::io::Error> for SdkError {
    fn from(err: std::io::Error) -> Self {
        SdkError::Other(err.to_string())
    }
}

impl From<anchor_lang::error::Error> for SdkError {
    fn from(err: anchor_lang::error::Error) -> Self {
        SdkError::AnchorClient(err.to_string())
    }
}

pub type SdkResult<T> = Result<T, SdkError>;
