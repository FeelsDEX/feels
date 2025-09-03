use anchor_lang::prelude::*;
use std::fmt;
use thiserror::Error;

// ============================================================================
// Main Error Enum
// ============================================================================

/// Comprehensive error enum for the Feels Protocol
#[derive(Error, Debug, Clone, PartialEq)]
pub enum FeelsProtocolError {
    // ========================================================================
    // Math Errors
    // ========================================================================
    
    /// Arithmetic overflow occurred
    #[error("Math overflow in '{operation}' with values: {values:?}")]
    MathOverflow { operation: String, values: Vec<String> },
    
    /// Arithmetic underflow occurred
    #[error("Math underflow in '{operation}' with values: {values:?}")]
    MathUnderflow { operation: String, values: Vec<String> },
    
    /// Division by zero
    #[error("Division by zero in context: {context}")]
    DivisionByZero { context: String },
    
    /// Invalid mathematical operation
    #[error("Invalid math operation '{operation}': {reason}")]
    InvalidMathOperation { operation: String, reason: String },
    
    /// Precision loss in calculation
    #[error("Precision loss in '{operation}': {precision_lost} bits lost")]
    PrecisionLoss { operation: String, precision_lost: u32 },
    
    // ========================================================================
    // Field Commitment Errors
    // ========================================================================
    
    /// Field commitment validation failed
    #[error("Invalid field commitment (seq {sequence:?}): {reason}")]
    InvalidFieldCommitment { reason: String, sequence: Option<u64> },
    
    /// Field commitment is stale
    #[error("Stale field commitment: {age_seconds}s old (max {max_age}s)")]
    StaleFieldCommitment { age_seconds: i64, max_age: i64 },
    
    /// Field commitment sequence number invalid
    #[error("Invalid sequence: expected {expected}, got {received}")]
    InvalidSequence { expected: u64, received: u64 },
    
    /// Field commitment hash mismatch
    #[error("Hash mismatch: expected {expected:?}, computed {computed:?}")]
    HashMismatch { expected: [u8; 32], computed: [u8; 32] },
    
    /// Local coefficients expired
    #[error("Local coefficients expired at {expired_at}")]
    LocalCoefficientsExpired { expired_at: i64 },
    
    /// Position out of bounds
    #[error("Position out of bounds: {position:?} not in valid range")]
    PositionOutOfBounds { position: [f64; 3] },
    
    // ========================================================================
    // Market State Errors
    // ========================================================================
    
    /// Inconsistent market state
    #[error("Inconsistent market state: {reason}")]
    InconsistentMarketState { reason: String },
    
    /// Market data is stale
    #[error("Market data stale: {age_seconds}s old (max {max_age}s)")]
    MarketDataStale { age_seconds: i64, max_age: i64 },
    
    /// Insufficient market data
    #[error("Insufficient market data: {reason}")]
    InsufficientMarketData { reason: String },
    
    /// Insufficient liquidity
    #[error("Insufficient liquidity: need {required}, have {available}")]
    InsufficientLiquidity { required: u128, available: u128 },
    
    /// Excessive price impact
    #[error("Excessive price impact: {impact_bps} bps (max {max_impact_bps} bps)")]
    ExcessivePriceImpact { impact_bps: u64, max_impact_bps: u64 },
    
    // ========================================================================
    // Validation Errors
    // ========================================================================
    
    /// Invalid parameter
    #[error("Invalid parameter '{parameter}': got '{value}', expected '{expected}'")]
    InvalidParameter { parameter: String, value: String, expected: String },
    
    /// Parameter out of range
    #[error("Parameter '{parameter}' out of range: {value} not in [{min}, {max}]")]
    ParameterOutOfRange { parameter: String, value: f64, min: f64, max: f64 },
    
    /// Invalid weights
    #[error("Invalid weights: {reason}")]
    InvalidWeights { reason: String },
    
    /// Invalid tick
    #[error("Invalid tick {tick}: not in valid range [{min_tick}, {max_tick}]")]
    InvalidTick { tick: i32, min_tick: i32, max_tick: i32 },
    
    /// Invalid price range
    #[error("Invalid price range: {min_price} to {max_price} ({reason})")]
    InvalidPriceRange { min_price: u128, max_price: u128, reason: String },
    
    // ========================================================================
    // Account and Authorization Errors
    // ========================================================================
    
    /// Unauthorized access attempt
    #[error("Unauthorized: authority {authority} != required {required}")]
    Unauthorized { authority: Pubkey, required: Pubkey },
    
    /// Invalid account provided
    #[error("Invalid account {account}: {reason}")]
    InvalidAccount { account: Pubkey, reason: String },
    
    /// Account not initialized
    #[error("Account not initialized: {account}")]
    NotInitialized { account: Pubkey },
    
    /// Account already initialized
    #[error("Account already initialized: {account}")]
    AlreadyInitialized { account: Pubkey },
    
    /// Invalid account owner
    #[error("Invalid account owner: expected {expected}, got {actual}")]
    InvalidAccountOwner { expected: Pubkey, actual: Pubkey },
    
    // ========================================================================
    // Network and RPC Errors
    // ========================================================================
    
    /// RPC communication error
    #[error("RPC error (code {code:?}): {message}")]
    RpcError { message: String, code: Option<i32> },
    
    /// Transaction failed
    #[error("Transaction failed ({tx_hash:?}): {error}")]
    TransactionFailed { error: String, tx_hash: Option<String> },
    
    /// Network timeout
    #[error("Network timeout after {timeout_ms}ms")]
    NetworkTimeout { timeout_ms: u64 },
    
    /// Account fetch failed
    #[error("Failed to fetch account {account}: {reason}")]
    AccountFetchFailed { account: Pubkey, reason: String },
    
    // ========================================================================
    // Computation Errors
    // ========================================================================
    
    /// Computation failed
    #[error("Computation failed in '{operation}': {reason}")]
    ComputationFailed { operation: String, reason: String },
    
    /// Eigenvalue computation failed
    #[error("Eigenvalue computation failed: {reason}")]
    EigenvalueFailed { reason: String },
    
    /// Optimization failed
    #[error("Optimization failed after {iterations} iterations: {reason}")]
    OptimizationFailed { iterations: u32, reason: String },
    
    /// Numerical instability
    #[error("Numerical instability detected: {details}")]
    NumericalInstability { details: String },
    
    // ========================================================================
    // Configuration Errors
    // ========================================================================
    
    /// Invalid configuration
    #[error("Invalid configuration for '{component}': {reason}")]
    InvalidConfiguration { component: String, reason: String },
    
    /// Missing configuration
    #[error("Missing configuration for '{component}': {reason}")]
    MissingConfiguration { component: String, reason: String },
    
    /// Configuration conflict
    #[error("Configuration conflict: {reason}")]
    ConfigurationConflict { reason: String },
    
    // ========================================================================
    // General Errors
    // ========================================================================
    
    /// Generic error with optional context
    #[error("Error: {message}")]
    Generic { message: String, context: Option<String> },
    
    /// Feature not implemented
    #[error("Not implemented: {feature}")]
    NotImplemented { feature: String },
    
    /// Internal error
    #[error("Internal error in '{component}': {details}")]
    Internal { component: String, details: String },
}

impl FeelsProtocolError {
    /// Create a math overflow error with context
    pub fn math_overflow(operation: &str, values: &[&str]) -> Self {
        Self::MathOverflow {
            operation: operation.to_string(),
            values: values.iter().map(|s| s.to_string()).collect(),
        }
    }
    
    /// Create a math underflow error with context
    pub fn math_underflow(operation: &str, values: &[&str]) -> Self {
        Self::MathUnderflow {
            operation: operation.to_string(),
            values: values.iter().map(|s| s.to_string()).collect(),
        }
    }
    
    /// Create an invalid parameter error
    pub fn invalid_parameter(parameter: &str, value: &str, expected: &str) -> Self {
        Self::InvalidParameter {
            parameter: parameter.to_string(),
            value: value.to_string(),
            expected: expected.to_string(),
        }
    }
    
    /// Create an unauthorized error
    pub fn unauthorized(authority: Pubkey, required: Pubkey) -> Self {
        Self::Unauthorized { authority, required }
    }
    
    /// Create a stale field commitment error
    pub fn stale_field_commitment(age_seconds: i64, max_age: i64) -> Self {
        Self::StaleFieldCommitment { age_seconds, max_age }
    }
    
    /// Create an insufficient liquidity error
    pub fn insufficient_liquidity(required: u128, available: u128) -> Self {
        Self::InsufficientLiquidity { required, available }
    }
    
    /// Create an RPC error
    pub fn rpc_error(message: &str, code: Option<i32>) -> Self {
        Self::RpcError {
            message: message.to_string(),
            code,
        }
    }
    
    /// Create a generic error
    pub fn generic(message: &str) -> Self {
        Self::Generic {
            message: message.to_string(),
            context: None,
        }
    }
    
    /// Create a generic error with context
    pub fn generic_with_context(message: &str, context: &str) -> Self {
        Self::Generic {
            message: message.to_string(),
            context: Some(context.to_string()),
        }
    }
    
    /// Create a field commitment error
    pub fn field_commitment_error(reason: &str, sequence: Option<u64>) -> Self {
        Self::InvalidFieldCommitment {
            reason: reason.to_string(),
            sequence,
        }
    }
    
    /// Create a parse error
    pub fn parse_error(message: &str, context: Option<&str>) -> Self {
        Self::Generic {
            message: format!("Parse error: {}", message),
            context: context.map(|s| s.to_string()),
        }
    }
    
    /// Create an insufficient balance error
    pub fn insufficient_balance(current: u64, required: u64) -> Self {
        Self::Generic {
            message: format!("Insufficient balance: {} lamports, required {} lamports", current, required),
            context: None,
        }
    }
}

/// Result type alias using the shared error type
pub type FeelsResult<T> = std::result::Result<T, FeelsProtocolError>;