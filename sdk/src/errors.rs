use thiserror::Error;

#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Client error: {0}")]
    ClientError(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Program error: {0}")]
    ProgramError(#[from] anchor_lang::error::Error),
    
    #[error("Solana client error: {0}")]
    SolanaClientError(#[from] solana_client::client_error::ClientError),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Simulation error: {0}")]
    SimulationError(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

pub type SdkResult<T> = Result<T, SdkError>;