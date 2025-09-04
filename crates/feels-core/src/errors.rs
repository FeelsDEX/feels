//! # Core Error Types
//! 
//! Common error types shared between on-chain and off-chain code.
//! Uses conditional compilation to provide appropriate error implementations.

use thiserror::Error;

/// Core protocol errors that can occur in both environments
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::error::AnchorError))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
pub enum FeelsCoreError {
    // ========================================================================
    // Math Errors
    // ========================================================================
    
    #[error("Math overflow")]
    MathOverflow,
    
    #[error("Math underflow")]
    MathUnderflow,
    
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Mul div overflow")]
    MulDivOverflow,
    
    #[error("Precision loss")]
    PrecisionLoss,
    
    #[error("Invalid logarithm input")]
    InvalidLogarithmInput,
    
    #[error("Exponential overflow")]
    ExponentialOverflow,
    
    // ========================================================================
    // Validation Errors
    // ========================================================================
    
    #[error("Invalid amount")]
    InvalidAmount,
    
    #[error("Invalid price")]
    InvalidPrice,
    
    #[error("Invalid tick")]
    InvalidTick,
    
    #[error("Invalid parameter")]
    InvalidParameter,
    
    #[error("Value out of bounds")]
    OutOfBounds,
    
    #[error("Invalid weight configuration")]
    InvalidWeights,
    
    #[error("Weights do not sum to 100%")]
    InvalidWeightSum,
    
    #[error("Invalid price range")]
    InvalidPriceRange,
    
    #[error("Tick out of range")]
    TickOutOfRange,
    
    // ========================================================================
    // Route and Trading Errors
    // ========================================================================
    
    #[error("Invalid route: {0}")]
    InvalidRoute(&'static str),
    
    #[error("Route too long: {0} hops (max {1})")]
    RouteTooLong(usize, usize),
    
    #[error("Too many segments: {0} (max {1})")]
    TooManySegments(usize, usize),
    
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,
    
    #[error("Excessive price impact")]
    ExcessivePriceImpact,
    
    #[error("Slippage exceeded")]
    SlippageExceeded,
    
    // ========================================================================
    // Field and Market State Errors
    // ========================================================================
    
    #[error("Invalid field commitment")]
    InvalidFieldCommitment,
    
    #[error("Stale field data")]
    StaleFieldData,
    
    #[error("Invalid sequence number")]
    InvalidSequence,
    
    #[error("Field hash mismatch")]
    FieldHashMismatch,
    
    #[error("Local coefficients expired")]
    LocalCoefficientsExpired,
    
    #[error("Inconsistent market state")]
    InconsistentMarketState,
    
    #[error("Market data stale")]
    MarketDataStale,
    
    #[error("Position out of bounds")]
    PositionOutOfBounds,
    
    // ========================================================================
    // Account and Authorization Errors
    // ========================================================================
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Invalid account")]
    InvalidAccount,
    
    #[error("Account not initialized")]
    NotInitialized,
    
    #[error("Account already initialized")]
    AlreadyInitialized,
    
    #[error("Invalid account owner")]
    InvalidAccountOwner,
    
    // ========================================================================
    // Computational Errors
    // ========================================================================
    
    #[error("Computation failed")]
    ComputationFailed,
    
    #[error("Eigenvalue computation failed")]
    EigenvalueFailed,
    
    #[error("Optimization failed")]
    OptimizationFailed,
    
    #[error("Numerical instability")]
    NumericalInstability,
    
    // ========================================================================
    // General Errors
    // ========================================================================
    
    #[error("Stale data")]
    StaleData,
    
    #[error("Conversion error")]
    ConversionError,
    
    #[error("Not implemented")]
    NotImplemented,
    
    #[error("Internal error")]
    InternalError,
}

/// Result type using core errors
pub type CoreResult<T> = Result<T, FeelsCoreError>;

// Conversion implementations for on-chain use
#[cfg(feature = "anchor")]
impl From<FeelsCoreError> for anchor_lang::error::Error {
    fn from(err: FeelsCoreError) -> Self {
        anchor_lang::error::Error::from(err)
    }
}

// Helper functions for creating specific errors
impl FeelsCoreError {
    /// Create a route too long error
    pub fn route_too_long(actual: usize, max: usize) -> Self {
        Self::RouteTooLong(actual, max)
    }
    
    /// Create a too many segments error
    pub fn too_many_segments(actual: usize, max: usize) -> Self {
        Self::TooManySegments(actual, max)
    }
    
    /// Create an invalid route error with reason
    pub fn invalid_route(reason: &'static str) -> Self {
        Self::InvalidRoute(reason)
    }
}

// Extended error information for off-chain use
#[cfg(feature = "client")]
pub mod extended {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    /// Extended error with additional context for off-chain diagnostics
    #[derive(Error, Debug, Clone, Serialize, Deserialize)]
    pub enum ExtendedError {
        
        /// Math error with operation context
        #[error("Math error in '{operation}': {details}")]
        MathError { operation: String, details: String },
        
        /// Field error with commitment data
        #[error("Field error (seq {sequence:?}): {reason}")]
        FieldError { reason: String, sequence: Option<u64> },
        
        /// Market error with state details
        #[error("Market error: {reason}")]
        MarketError { reason: String },
        
        /// Network error
        #[error("Network error: {message}")]
        NetworkError { message: String },
        
        /// RPC error
        #[error("RPC error (code {code:?}): {message}")]
        RpcError { message: String, code: Option<i32> },
        
        /// Transaction failed
        #[error("Transaction failed: {error}")]
        TransactionFailed { error: String },
        
        /// Configuration error
        #[error("Configuration error for '{component}': {reason}")]
        ConfigurationError { component: String, reason: String },
    }
    
    impl ExtendedError {
        /// Create a math error with context
        pub fn math_error(operation: &str, details: &str) -> Self {
            Self::MathError {
                operation: operation.to_string(),
                details: details.to_string(),
            }
        }
        
        /// Create a field error
        pub fn field_error(reason: &str, sequence: Option<u64>) -> Self {
            Self::FieldError {
                reason: reason.to_string(),
                sequence,
            }
        }
        
        /// Create a market error
        pub fn market_error(reason: &str) -> Self {
            Self::MarketError {
                reason: reason.to_string(),
            }
        }
        
        /// Create an RPC error
        pub fn rpc_error(message: &str, code: Option<i32>) -> Self {
            Self::RpcError {
                message: message.to_string(),
                code,
            }
        }
    }
    
    /// Extended result type for off-chain use
    pub type ExtendedResult<T> = Result<T, ExtendedError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = FeelsCoreError::route_too_long(3, 2);
        assert_eq!(format!("{}", err), "Route too long: 3 hops (max 2)");
        
        let err = FeelsCoreError::invalid_route("must start from hub");
        assert_eq!(format!("{}", err), "Invalid route: must start from hub");
    }
    
    #[cfg(feature = "client")]
    #[test]
    fn test_extended_errors() {
        use extended::*;
        
        let err = ExtendedError::math_error("sqrt", "negative input");
        assert!(format!("{}", err).contains("sqrt"));
        
        let err = ExtendedError::field_error("invalid hash", Some(42));
        assert!(format!("{}", err).contains("seq 42"));
    }
}