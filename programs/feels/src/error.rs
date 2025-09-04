//! # Thermodynamic AMM Error System
//! 
//! Comprehensive error handling for the Feels Protocol's 3D market physics implementation.
//! Errors are organized around the core thermodynamic concepts and market operations:
//! 
//! ## Error Categories - Physics-Focused Organization
//! 
//! ### **1. Thermodynamic Violations**
//! - **Conservation Law Errors**: When Σ wᵢ ln(gᵢ) ≠ 0 
//! - **Work Calculation Errors**: Issues in W = V(P₂) - V(P₁) computation
//! - **Field Evolution Errors**: Problems in (S,T,L) scalar updates
//! - **Physics Math Errors**: Overflow/underflow in thermodynamic calculations
//! 
//! ### **2. Hub-and-Spoke Routing Violations**
//! - **Route Constraint Errors**: Violations of max-2-hop routing rules
//! - **Hub Token Errors**: Non-FeelsSOL routes where FeelsSOL is required
//! - **Entry/Exit Violations**: JitoSOL ↔ FeelsSOL pairing requirements
//! 
//! ### **3. Market State & Safety**
//! - **Leverage Safety Errors**: L-dimension risk management violations
//! - **Oracle & Staleness**: Time-based data validity issues  
//! - **Rate-of-Change Limits**: Excessive market state transitions
//! - **Buffer & Rebate Errors**: Fee collection and rebate capacity issues
//! 
//! ### **4. Standard AMM Operations**
//! - **Liquidity Math Errors**: Concentrated liquidity calculation issues
//! - **Tick Management Errors**: Discrete price level violations
//! - **Access Control Errors**: Unauthorized operations and ownership
//! 
//! The error system provides contextual helpers to make debugging physics-related
//! issues easier, with detailed logging for complex thermodynamic calculations.

use anchor_lang::prelude::*;

/// Main protocol error enum organized around thermodynamic concepts
#[error_code]
pub enum FeelsError {
    // ========================================================================
    // 1. THERMODYNAMIC VIOLATIONS - Physics Model Constraint Errors
    // ========================================================================
    // Errors related to conservation laws, work calculations, and field evolution
    
    #[msg("Conservation law violation: Σ wᵢ ln(gᵢ) ≠ 0")]
    ConservationViolation,
    
    #[msg("Invalid work calculation in thermodynamic transition")]
    InvalidWorkCalculation,
    
    #[msg("Field evolution rate exceeds safety bounds")]
    ExcessiveFieldChange,
    
    #[msg("Invalid update mode for field commitment")]
    InvalidUpdateMode,
    
    // ========================================================================
    // 2. HUB-AND-SPOKE ROUTING VIOLATIONS
    // ========================================================================
    // Errors enforcing the hub-and-spoke architecture constraints
    
    #[msg("Route exceeds maximum allowed hops")]
    RouteTooLong,
    
    #[msg("Invalid pool in route - must include FeelsSOL")]
    InvalidRoutePool,
    
    #[msg("Route segments exceed maximum allowed")]
    TooManySegments,
    
    #[msg("Invalid entry/exit pairing - must use JitoSOL <-> FeelsSOL")]
    InvalidEntryExitPairing,
    
    // ========================================================================
    // 3. MARKET STATE & SAFETY VIOLATIONS
    // ========================================================================
    // Errors related to leverage safety, oracle staleness, and rate limits
    
    #[msg("Data staleness violation")]
    StaleData,
    
    #[msg("Update frequency violation")]
    UpdateTooFrequent,
    
    #[msg("Commitment expired")]
    CommitmentExpired,
    
    #[msg("Oracle data is stale")]
    StaleOracle,
    
    #[msg("Oracle price deviation")]
    OraclePriceDeviation,
    
    #[msg("Market conditions prevent leverage")]
    MarketConditionsPreventLeverage,
    
    #[msg("Insufficient buffer")]
    InsufficientBuffer,
    
    #[msg("Excessive change")]
    ExcessiveChange,
    
    #[msg("Price manipulation detected")]
    PriceManipulationDetected,
    
    // ========================================================================
    // 4. GENERAL VALIDATION ERRORS
    // ========================================================================
    // Basic input validation and constraint violations
    
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
    
    #[msg("Price out of bounds")]
    PriceOutOfBounds,
    
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
    
    #[msg("Invalid tick index")]
    InvalidTickIndex,
    
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
    
    // ========================================================================
    // Routing Errors
    // ========================================================================
    
    #[msg("Route exceeds maximum allowed hops")]
    RouteTooLong,
    
    #[msg("Invalid pool in route - must include FeelsSOL")]
    InvalidRoutePool,
    
    #[msg("Route segments exceed maximum allowed")]
    TooManySegments,
    
    #[msg("Invalid entry/exit pairing - must use JitoSOL <-> FeelsSOL")]
    InvalidEntryExitPairing,
    
    #[msg("Invalid expiration time")]
    InvalidExpiration,
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
    
    /// Helper for route too long error with hop count
    pub fn route_too_long(hops: usize, max_hops: usize) -> Self {
        msg!("Route has {} hops, maximum allowed is {}", hops, max_hops);
        FeelsError::RouteTooLong
    }
    
    /// Helper for invalid route pool error
    pub fn invalid_route_pool(pool: &str) -> Self {
        msg!("Invalid pool in route: {} - must include FeelsSOL", pool);
        FeelsError::InvalidRoutePool
    }
    
    /// Helper for too many segments error
    pub fn too_many_segments(segments: usize, max_segments: usize) -> Self {
        msg!("Route has {} segments, maximum allowed is {}", segments, max_segments);
        FeelsError::TooManySegments
    }
}

/// Alias for backward compatibility
pub use FeelsError as FeelsProtocolError;