/// Hierarchical error system for the Feels Protocol
/// 
/// Uses parameterized errors with context to reduce the number of variants
/// while providing detailed error information.
use anchor_lang::prelude::*;

/// Main protocol error enum with hierarchical categories
#[error_code]
pub enum FeelsError {
    // ========================================================================
    // Validation Errors: Input validation and constraint violations
    // ========================================================================
    
    #[msg("Validation error: {field} - {reason}")]
    ValidationError { field: String, reason: String },
    
    #[msg("Invalid amount: {amount_type} - {reason}")]
    InvalidAmount { amount_type: String, reason: String },
    
    #[msg("Invalid range: {range_type} from {min} to {max}")]
    InvalidRange { range_type: String, min: String, max: String },
    
    #[msg("Constraint violation: {constraint} - {value}")]
    ConstraintViolation { constraint: String, value: String },
    
    // ========================================================================
    // Math Errors: Arithmetic and mathematical operation failures
    // ========================================================================
    
    #[msg("Math error: {operation} - {details}")]
    MathError { operation: String, details: String },
    
    #[msg("Overflow in {operation}")]
    Overflow { operation: String },
    
    #[msg("Underflow in {operation}")]
    Underflow { operation: String },
    
    #[msg("Division by zero in {context}")]
    DivisionByZero { context: String },
    
    // ========================================================================
    // State Errors: State management and consistency violations
    // ========================================================================
    
    #[msg("State error: {state_type} - {reason}")]
    StateError { state_type: String, reason: String },
    
    #[msg("Insufficient {resource}: have {available}, need {required}")]
    InsufficientResource { 
        resource: String, 
        available: String, 
        required: String 
    },
    
    #[msg("{entity} not found: {identifier}")]
    NotFound { entity: String, identifier: String },
    
    #[msg("{entity} not initialized: {identifier}")]
    NotInitialized { entity: String, identifier: String },
    
    // ========================================================================
    // Security Errors: Access control and security violations
    // ========================================================================
    
    #[msg("Unauthorized: {action} requires {required_role}")]
    Unauthorized { action: String, required_role: String },
    
    #[msg("Security violation: {violation_type} - {details}")]
    SecurityViolation { violation_type: String, details: String },
    
    #[msg("Reentrancy detected in {operation}")]
    ReentrancyDetected { operation: String },
    
    // ========================================================================
    // Protocol Errors: Protocol-specific business logic errors
    // ========================================================================
    
    #[msg("Protocol error: {error_type} - {details}")]
    ProtocolError { error_type: String, details: String },
    
    #[msg("Feature not available: {feature} - {reason}")]
    FeatureUnavailable { feature: String, reason: String },
    
    #[msg("Operation paused: {operation} - expires at {expiry}")]
    OperationPaused { operation: String, expiry: String },
    
    // ========================================================================
    // Market Errors: Market-related errors
    // ========================================================================
    
    #[msg("Market condition: {condition} - {details}")]
    MarketCondition { condition: String, details: String },
    
    #[msg("Slippage exceeded: expected {expected}, got {actual}")]
    SlippageExceeded { expected: String, actual: String },
    
    #[msg("Price impact too high: {impact}% exceeds {max_allowed}%")]
    ExcessivePriceImpact { impact: String, max_allowed: String },
    
    // ========================================================================
    // Conservation Errors: Physics model conservation violations
    // ========================================================================
    
    #[msg("Conservation violation: {invariant} - expected {expected}, got {actual}")]
    ConservationViolation { 
        invariant: String, 
        expected: String, 
        actual: String 
    },
    
    #[msg("Rebase error: {rebase_type} - {reason}")]
    RebaseError { rebase_type: String, reason: String },
    
    #[msg("Weight change too large: {dimension} changed by {change_bps} bps")]
    ExcessiveWeightChange { dimension: String, change_bps: u32 },
}

// ============================================================================
// Error Context Helpers
// ============================================================================

/// Helper functions to create specific errors with context
impl FeelsError {
    /// Create a validation error for amounts
    pub fn invalid_amount(amount_type: &str, amount: u64) -> Self {
        Self::InvalidAmount {
            amount_type: amount_type.to_string(),
            reason: format!("Amount {} is invalid", amount),
        }
    }
    
    /// Create a validation error for zero amounts
    pub fn zero_amount(amount_type: &str) -> Self {
        Self::InvalidAmount {
            amount_type: amount_type.to_string(),
            reason: "Amount must be greater than zero".to_string(),
        }
    }
    
    /// Create an overflow error
    pub fn overflow(operation: &str) -> Self {
        Self::Overflow {
            operation: operation.to_string(),
        }
    }
    
    /// Create an underflow error
    pub fn underflow(operation: &str) -> Self {
        Self::Underflow {
            operation: operation.to_string(),
        }
    }
    
    /// Create an insufficient liquidity error
    pub fn insufficient_liquidity(available: u128, required: u128) -> Self {
        Self::InsufficientResource {
            resource: "liquidity".to_string(),
            available: available.to_string(),
            required: required.to_string(),
        }
    }
    
    /// Create a tick not found error
    pub fn tick_not_found(tick: i32) -> Self {
        Self::NotFound {
            entity: "Tick".to_string(),
            identifier: tick.to_string(),
        }
    }
    
    /// Create a slippage error
    pub fn slippage(expected: u64, actual: u64) -> Self {
        Self::SlippageExceeded {
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }
    
    /// Create a conservation violation error
    pub fn conservation(invariant: &str, expected: i128, actual: i128) -> Self {
        Self::ConservationViolation {
            invariant: invariant.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }
}

// ============================================================================
// Error Categories for Logging
// ============================================================================

/// Error categories for structured logging and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Validation,
    Math,
    State,
    Security,
    Protocol,
    Market,
    Conservation,
}

impl FeelsError {
    /// Get the category of this error for logging/monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::ValidationError { .. } |
            Self::InvalidAmount { .. } |
            Self::InvalidRange { .. } |
            Self::ConstraintViolation { .. } => ErrorCategory::Validation,
            
            Self::MathError { .. } |
            Self::Overflow { .. } |
            Self::Underflow { .. } |
            Self::DivisionByZero { .. } => ErrorCategory::Math,
            
            Self::StateError { .. } |
            Self::InsufficientResource { .. } |
            Self::NotFound { .. } |
            Self::NotInitialized { .. } => ErrorCategory::State,
            
            Self::Unauthorized { .. } |
            Self::SecurityViolation { .. } |
            Self::ReentrancyDetected { .. } => ErrorCategory::Security,
            
            Self::ProtocolError { .. } |
            Self::FeatureUnavailable { .. } |
            Self::OperationPaused { .. } => ErrorCategory::Protocol,
            
            Self::MarketCondition { .. } |
            Self::SlippageExceeded { .. } |
            Self::ExcessivePriceImpact { .. } => ErrorCategory::Market,
            
            Self::ConservationViolation { .. } |
            Self::RebaseError { .. } |
            Self::ExcessiveWeightChange { .. } => ErrorCategory::Conservation,
        }
    }
    
    /// Get severity level for monitoring
    pub fn severity(&self) -> ErrorSeverity {
        match self.category() {
            ErrorCategory::Security => ErrorSeverity::Critical,
            ErrorCategory::Conservation => ErrorSeverity::Critical,
            ErrorCategory::Math => ErrorSeverity::High,
            ErrorCategory::State => ErrorSeverity::High,
            ErrorCategory::Market => ErrorSeverity::Medium,
            ErrorCategory::Protocol => ErrorSeverity::Medium,
            ErrorCategory::Validation => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// Backwards Compatibility
// ============================================================================

/// Alias for backwards compatibility
pub use FeelsError as FeelsProtocolError;

// Common error mappings for migration
impl FeelsError {
    pub fn invalid_authority() -> Self {
        Self::Unauthorized {
            action: "Admin operation".to_string(),
            required_role: "Authority".to_string(),
        }
    }
    
    pub fn math_overflow() -> Self {
        Self::Overflow {
            operation: "Arithmetic operation".to_string(),
        }
    }
    
    pub fn invalid_input() -> Self {
        Self::ValidationError {
            field: "Input".to_string(),
            reason: "Invalid input provided".to_string(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_categories() {
        let validation_err = FeelsError::zero_amount("swap");
        assert_eq!(validation_err.category(), ErrorCategory::Validation);
        assert_eq!(validation_err.severity(), ErrorSeverity::Low);
        
        let security_err = FeelsError::Unauthorized {
            action: "pause".to_string(),
            required_role: "admin".to_string(),
        };
        assert_eq!(security_err.category(), ErrorCategory::Security);
        assert_eq!(security_err.severity(), ErrorSeverity::Critical);
    }
    
    #[test]
    fn test_error_helpers() {
        let err = FeelsError::insufficient_liquidity(1000, 2000);
        match err {
            FeelsError::InsufficientResource { resource, available, required } => {
                assert_eq!(resource, "liquidity");
                assert_eq!(available, "1000");
                assert_eq!(required, "2000");
            }
            _ => panic!("Wrong error type"),
        }
    }
}