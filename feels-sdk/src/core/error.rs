use thiserror::Error;

#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("RPC error: {0}")]
    // RpcError(#[from] solana_client::client_error::ClientError),  // Disabled - no solana-client
    RpcError(String),  // Simple string error for RPC issues

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Math overflow")]
    MathOverflow,

    #[error("Invalid route: {0}")]
    InvalidRoute(String),

    #[error("No route found from {0} to {1}")]
    NoRouteFound(String, String),

    #[error("Market not found")]
    MarketNotFound,

    #[error("Invalid tick array")]
    InvalidTickArray,

    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
}

pub type SdkResult<T> = Result<T, SdkError>;
