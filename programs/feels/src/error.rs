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
    
    #[msg("Validation error occurred")]
    ValidationError,
    
    #[msg("Invalid amount provided")]
    InvalidAmount,
    
    #[msg("Invalid range specified")]
    InvalidRangeSpecified,
    
    #[msg("Constraint violation detected")]
    ConstraintViolation,
    
    // ========================================================================
    // Math Errors: Arithmetic and mathematical operation failures
    // ========================================================================
    
    #[msg("Mathematical operation failed")]
    MathError,
    
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    
    #[msg("Arithmetic underflow occurred")]  
    ArithmeticUnderflow,
    
    #[msg("Division by zero")]
    DivisionByZero,
    
    #[msg("Math overflow")]
    MathOverflow,
    
    // ========================================================================
    // State Errors: State management and consistency violations
    // ========================================================================
    
    #[msg("State error occurred")]
    StateError,
    
    #[msg("Insufficient resources")]
    InsufficientResource,
    
    #[msg("Entity not found")]
    NotFound,
    
    #[msg("Entity not initialized")]
    NotInitialized,
    
    // ========================================================================
    // Security Errors: Access control and security violations
    // ========================================================================
    
    #[msg("Unauthorized action")]
    Unauthorized,
    
    #[msg("Security violation detected")]
    SecurityViolation,
    
    #[msg("Reentrancy detected")]
    ReentrancyDetected,
    
    #[msg("Invalid owner")]
    InvalidOwner,
    
    #[msg("Invalid account owner")]
    InvalidAccountOwner,
    
    #[msg("Invalid market")]
    InvalidMarket,
    
    // ========================================================================
    // Common Errors
    // ========================================================================
    
    #[msg("Invalid input")]
    InvalidInput,
    
    #[msg("Invalid tick")]
    InvalidTick,
    
    #[msg("Invalid percentage")]
    InvalidPercentage,
    
    #[msg("Invalid liquidity")]
    InvalidLiquidity,
    
    #[msg("Tick out of bounds")]
    TickOutOfBounds,
    
    #[msg("Square root price out of bounds")]
    SqrtPriceOutOfBounds,
    
    // Missing variants that are referenced in code
    #[msg("Invalid parameter")]
    InvalidParameter,
    
    #[msg("Invalid tick range")]
    InvalidTickRange,
    
    #[msg("Invalid pool")]
    InvalidPool,
    
    #[msg("Invalid weights")]
    InvalidWeights,
    
    #[msg("Invalid duration")]
    InvalidDuration,
    
    #[msg("Invalid rate parameters")]
    InvalidRateParams,
    
    #[msg("Invalid range")]
    InvalidRange,
    
    #[msg("Invalid sequence number")]
    InvalidSequence,
    
    #[msg("Data staleness violation")]
    StaleData,
    
    #[msg("Update frequency violation")]
    UpdateTooFrequent,
    
    #[msg("Commitment expired")]
    CommitmentExpired,
    
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    
    #[msg("Decimals too large")]
    DecimalsTooLarge,
    
    #[msg("Invalid token name")]
    InvalidTokenName,
    
    #[msg("Oracle price deviation")]
    OraclePriceDeviation,
    
    #[msg("Market conditions prevent leverage")]
    MarketConditionsPreventLeverage,
    
    #[msg("Invalid tick array")]
    InvalidTickArray,
    
    #[msg("Insufficient data")]
    InsufficientData,
    
    #[msg("Tick not aligned")]
    TickNotAligned,
    
    #[msg("Tick array not empty")]
    TickArrayNotEmpty,
    
    #[msg("Tick array not found")]
    TickArrayNotFound,
    
    #[msg("Invalid operation")]
    InvalidOperation,
    
    #[msg("Invalid price range")]
    InvalidPriceRange,
    
    #[msg("Token not found")]
    TokenNotFound,
    
    #[msg("Stale price")]
    StalePrice,
    
    #[msg("Rate out of bounds")]
    RateOutOfBounds,
    
    #[msg("Insufficient observations")]
    InsufficientObservations,
    
    #[msg("Insufficient buffer")]
    InsufficientBuffer,
    
    #[msg("Excessive change")]
    ExcessiveChange,
    
    // ========================================================================
    // Additional Missing Variants
    // ========================================================================
    
    #[msg("Invalid authority")]
    InvalidAuthority,
    
    #[msg("Invalid tick array account")]
    InvalidTickArrayAccount,
    
    #[msg("Hook validation failed")]
    HookValidationFailed,
    
    #[msg("Hook registry full")]
    HookRegistryFull,
    
    #[msg("Message queue full")]
    MessageQueueFull,
    
    #[msg("Invalid permission")]
    InvalidPermission,
    
    #[msg("Non-canonical token order")]
    NonCanonicalTokenOrder,
    
    #[msg("Swap amount too small")]
    SwapAmountTooSmall,
    
    #[msg("Price manipulation detected")]
    PriceManipulationDetected,
    
    #[msg("Extreme price")]
    ExtremePrice,
    
    #[msg("Tick array index out of bounds")]
    TickArrayIndexOutOfBounds,
    
    // Additional error variants for compatibility
    #[msg("Math underflow")]
    MathUnderflow,
    
    #[msg("Invalid fee setting")]
    InvalidFeeSetting,
    
    #[msg("Arithmetic error")]
    ArithmeticError,
    
    #[msg("Fee increase too large")]
    FeeIncreaseTooLarge,
    
    #[msg("Invalid pool status")]
    InvalidPoolStatus,
    
    #[msg("Fee decrease too large")]
    FeeDecreaseTooLarge,
    
    #[msg("Fee above maximum")]
    FeeAboveMaximum,
    
    #[msg("Invalid token order")]
    InvalidTokenOrder,
    
    #[msg("Exceeded max orders")]
    ExceededMaxOrders,
    
    #[msg("Invalid position token amount")]
    InvalidPositionTokenAmount,
    
    #[msg("Invalid slippage limit")]
    InvalidSlippageLimit,
    
    #[msg("Fee below minimum")]
    FeeBelowMinimum,
    
    #[msg("Invalid signature")]
    InvalidSignature,
    
    #[msg("Keeper not authorized")]
    UnauthorizedKeeper,
    
    #[msg("Keeper registry full")]
    KeeperRegistryFull,
    
    #[msg("Keeper already exists")]
    KeeperAlreadyExists,
    
    #[msg("Keeper not found")]
    KeeperNotFound,
    
    #[msg("Invalid field commitment")]
    InvalidFieldCommitment,
    
    #[msg("Oracle data is stale")]
    StaleOracle,
}

impl FeelsError {
    // ========================================================================
    // Helper Constructors with Context
    // ========================================================================
    
    /// Helper method for zero amount validation
    pub fn zero_amount(_context: &str) -> Self {
        FeelsError::InvalidAmount
    }
    
    /// Helper for range validation with context
    pub fn invalid_range(lower: i32, upper: i32) -> Self {
        msg!("Invalid range: {} to {}", lower, upper);
        FeelsError::InvalidRange
    }
    
    /// Helper for tick validation with context  
    pub fn invalid_tick(tick: i32, min: i32, max: i32) -> Self {
        msg!("Invalid tick {} (bounds: {} to {})", tick, min, max);
        FeelsError::InvalidTick
    }
    
    /// Helper for arithmetic overflow with operation context
    pub fn math_overflow(operation: &str) -> Self {
        msg!("Math overflow in operation: {}", operation);
        FeelsError::ArithmeticOverflow
    }
    
    /// Helper for underflow with operation context
    pub fn math_underflow(operation: &str) -> Self {
        msg!("Math underflow in operation: {}", operation);
        FeelsError::ArithmeticUnderflow
    }
    
    /// Helper for insufficient liquidity with amount context
    pub fn insufficient_liquidity(required: u128, available: u128) -> Self {
        msg!("Insufficient liquidity: need {}, have {}", required, available);
        FeelsError::InsufficientLiquidity
    }
    
    /// Helper for stale data with age context
    pub fn stale_data(age_seconds: i64, max_age: i64) -> Self {
        msg!("Stale data: age {}s exceeds max {}s", age_seconds, max_age);
        FeelsError::StaleData
    }
    
    /// Helper for invalid authority with context
    pub fn invalid_authority(expected: &str, actual: &str) -> Self {
        msg!("Invalid authority: expected {}, got {}", expected, actual);
        FeelsError::InvalidAuthority
    }
    
    /// Helper for excessive change with percentage context
    pub fn excessive_change(change_bps: u32, max_bps: u32) -> Self {
        msg!("Excessive change: {}bps exceeds max {}bps", change_bps, max_bps);
        FeelsError::ExcessiveChange
    }
    
    /// Helper for commitment expiry with time context
    pub fn commitment_expired(expires_at: i64, current_time: i64) -> Self {
        msg!("Commitment expired at {}, current time {}", expires_at, current_time);
        FeelsError::CommitmentExpired
    }
    
    /// Helper for price deviation with percentage context
    pub fn price_deviation(deviation_bps: u32, max_bps: u32) -> Self {
        msg!("Price deviation {}bps exceeds max {}bps", deviation_bps, max_bps);
        FeelsError::OraclePriceDeviation
    }
}

/// Alias for backward compatibility
pub use FeelsError as FeelsProtocolError;