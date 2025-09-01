/// Unified error system for the Feels Protocol
/// 
/// Consolidates all protocol errors into a single, well-organized enum with
/// clear categories for better error handling and debugging. All errors provide
/// clear, actionable messages for developers and users.
use anchor_lang::prelude::*;

/// Main protocol error enum with categorized error variants
#[error_code]
pub enum FeelsProtocolError {
    // ========================================================================
    // Validation Errors (0-99): Input validation and constraint violations
    // ========================================================================
    
    #[msg("Feature not implemented")]
    NotImplemented,
    #[msg("Invalid metadata format")]
    InvalidMetadata,
    #[msg("Invalid amount - must be greater than zero")]
    InvalidAmount,
    #[msg("Invalid token amount")]
    InvalidTokenAmount,
    #[msg("Input amount is zero")]
    InputAmountZero,
    #[msg("Input amount exceeds available liquidity")]
    InputAmountExceedsLiquidity,
    #[msg("Output amount is zero")]
    OutputAmountZero,
    #[msg("Invalid percentage - must be between 0 and 100")]
    InvalidPercentage,
    #[msg("Token ticker length must be between 1 and 12 characters")]
    InvalidTickerLength,
    #[msg("Token ticker contains invalid characters - only alphanumeric allowed")]
    InvalidTickerFormat,
    #[msg("Token ticker is restricted and cannot be used")]
    RestrictedTicker,
    #[msg("Token decimals must match for proper price calculations")]
    IncompatibleDecimals,
    #[msg("Token decimals too large - must be <= 18")]
    DecimalsTooLarge,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid mint - mint cannot be the same as underlying asset")]
    SameMintAsUnderlying,
    #[msg("Invalid token pair for swap")]
    InvalidTokenPair,
    #[msg("Not FeelsSOL pair")]
    NotFeelsSOLPair,
    #[msg("Invalid FeelsSOL mint")]
    InvalidFeelsSOL,
    #[msg("Invalid token name")]
    InvalidTokenName,
    #[msg("Invalid token symbol")]
    InvalidTokenSymbol,
    #[msg("Ticker and symbol must match")]
    TickerSymbolMismatch,
    #[msg("Invalid initial supply")]
    InvalidInitialSupply,

    // Rate and Tick Validation
    #[msg("Tick out of bounds")]
    TickOutOfBounds,
    #[msg("Tick not aligned to spacing")]
    TickNotAligned,
    #[msg("Invalid tick spacing")]
    InvalidTickSpacing,
    #[msg("Invalid tick range")]
    InvalidTickRange,
    #[msg("Invalid tick index")]
    InvalidTickIndex,
    #[msg("Rate out of bounds")]
    RateOutOfBounds,
    #[msg("Rate limit too aggressive - would result in no liquidity")]
    RateLimitTooAggressive,
    #[msg("Rate limit outside valid protocol range")]
    RateLimitOutsideValidRange,
    #[msg("Invalid rate range")]
    InvalidRateRange,
    #[msg("Invalid sqrt rate")]
    InvalidSqrtRate,
    #[msg("Invalid rate limit")]
    InvalidRateLimit,
    
    // Liquidity Validation
    #[msg("Invalid liquidity")]
    InvalidLiquidity,
    #[msg("Invalid liquidity amount")]
    InvalidLiquidityAmount,
    
    // Fee Validation
    #[msg("Invalid fee rate")]
    InvalidFeeRate,
    
    // Swap Validation
    #[msg("Invalid swap direction")]
    InvalidSwapDirection,
    #[msg("Swap amount too small - below minimum threshold")]
    SwapAmountTooSmall,
    #[msg("Token mints must be in canonical order (sorted by bytes)")]
    NonCanonicalTokenOrder,

    // Phase 2 Validation
    #[msg("Invalid leverage value")]
    InvalidLeverage,
    #[msg("Leverage exceeds current ceiling")]
    LeverageExceedsCeiling,
    #[msg("Leverage ceiling exceeds maximum allowed")]
    LeverageCeilingExceedsMax,
    #[msg("Leverage too high for current volatility")]
    LeverageTooHighForVolatility,
    #[msg("Leverage too high for liquidity depth")]
    LeverageTooHighForLiquidity,
    #[msg("Leverage too high for price impact")]
    LeverageTooHighForImpact,
    #[msg("Leverage value is too low")]
    LeverageTooLow,
    #[msg("Leverage exceeds maximum allowed")]
    LeverageExceedsMaximum,
    #[msg("Market conditions prevent higher leverage")]
    MarketConditionsPreventLeverage,
    #[msg("Invalid risk profile hash")]
    InvalidRiskProfile,
    #[msg("Invalid protection curve configuration")]
    InvalidProtectionCurve,
    #[msg("Invalid observation timestamp")]
    InvalidObservationTimestamp,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Price manipulation detected")]
    PriceManipulationDetected,
    #[msg("Extreme price movement")]
    ExtremePrice,
    #[msg("Logarithm undefined")]
    LogarithmUndefined,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Invalid duration")]
    InvalidDuration,
    #[msg("Minimum duration requirement not met")]
    MinimumDurationNotMet,
    
    // Flash loan errors
    #[msg("Flash loan not repaid in same slot")]
    FlashLoanNotRepaidInSameSlot,
    #[msg("Flash loan repayment before borrow")]
    FlashLoanRepaymentBeforeBorrow,
    #[msg("Flash loan already repaid")]
    FlashLoanAlreadyRepaid,
    #[msg("Insufficient flash loan repayment")]
    InsufficientFlashLoanRepayment,

    // ========================================================================
    // Math Errors (100-199): Arithmetic and mathematical operation failures
    // ========================================================================

    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Liquidity overflow")]
    LiquidityOverflow,
    #[msg("Liquidity underflow")]
    LiquidityUnderflow,
    
    // ========================================================================
    // State Errors (200-299): State management and consistency violations
    // ========================================================================

    #[msg("Invalid pool version")]
    InvalidVersion,
    #[msg("Insufficient token balance")]
    InsufficientBalance,
    #[msg("Insufficient liquidity in pool for swap")]
    InsufficientLiquidity,
    #[msg("Position has not matured yet - cannot remove liquidity before maturity")]
    PositionNotMatured,
    #[msg("Tick not found")]
    TickNotFound,
    #[msg("Tick not initialized")]
    TickNotInitialized,
    #[msg("Tick array not initialized")]
    TickArrayNotInitialized,
    #[msg("Tick array is not empty")]
    TickArrayNotEmpty,
    #[msg("Pool operations paused")]
    PoolOperationsPaused,
    #[msg("Pause not expired")]
    PauseNotExpired,
    #[msg("Emergency mode active")]
    EmergencyModeActive,
    #[msg("Slippage exceeded - output amount below minimum")]
    SlippageExceeded,
    #[msg("Slippage protection triggered - price moved beyond limit")]
    SlippageProtectionTriggered,
    #[msg("Swap would result in zero output")]
    SwapResultsInZeroOutput,
    
    // Reentrancy Protection
    #[msg("Reentrancy detected - operation already in progress")]
    ReentrancyDetected,
    #[msg("Invalid reentrancy state transition")]
    InvalidReentrancyState,
    
    // Token Operations
    #[msg("Token mint operation failed")]
    MintFailed,
    #[msg("Token burn operation failed")]
    BurnFailed,
    #[msg("Tokens are locked")]
    TokensLocked,

    // Tick Array State
    #[msg("Invalid tick array start")]
    InvalidTickArrayStart,
    #[msg("Invalid tick array count")]
    InvalidTickArrayCount,
    #[msg("Invalid tick array index")]
    InvalidTickArrayIndex,
    #[msg("Invalid tick array account")]
    InvalidTickArray,
    #[msg("Invalid tick array boundary")]
    InvalidTickArrayBoundary,

    // Phase 2 State Errors
    #[msg("Leverage not enabled for this pool")]
    LeverageNotEnabled,
    #[msg("Update too frequent - cooldown not met")]
    UpdateTooFrequent,
    #[msg("No leveraged position to redenominate")]
    NoLeverageToRedenominate,
    #[msg("Position vault not initialized")]
    PositionVaultNotInitialized,
    #[msg("Redenomination threshold exceeded")]
    RedenominationRequired,
    #[msg("Unauthorized redenomination")]
    UnauthorizedRedenomination,
    #[msg("Invalid redenomination amount")]
    InvalidRedenominationAmount,
    #[msg("Redenomination cooldown still active")]
    RedenominationCooldownActive,
    #[msg("Excessive redenomination loss")]
    ExcessiveRedenominationLoss,
    #[msg("Redenomination oracle price deviation")]
    RedenominationOracleDeviation,
    #[msg("Valence session not initialized")]
    ValenceSessionNotInitialized,
    #[msg("Dynamic fee calculation error")]
    DynamicFeeError,
    #[msg("No position found")]
    NoPositionFound,
    #[msg("Insufficient shares")]
    InsufficientShares,
    #[msg("Zero shares calculated")]
    ZeroShares,
    #[msg("Zero redemption value")]
    ZeroRedemptionValue,
    #[msg("Invalid token for share type")]
    InvalidTokenForShareType,
    #[msg("Insufficient flash loan liquidity")]
    InsufficientFlashLoanLiquidity,
    #[msg("Invalid pool for flash loan")]
    InvalidPoolForFlashLoan,
    #[msg("Invalid operation on finalized updates")]
    InvalidOperation,
    #[msg("Tick array router is full - maximum arrays reached")]
    RouterFull,
    #[msg("Hook registry is full - maximum hooks per type reached")]
    HookRegistryFull,
    #[msg("Volume below threshold")]
    VolumeBelowThreshold,

    // Position Vault State
    #[msg("Insufficient POL")]
    InsufficientPOL,
    #[msg("Insufficient user deposits")]
    InsufficientUserDeposits,
    #[msg("Would breach baseline floor")]
    WouldBreachBaselineFloor,
    #[msg("JIT disabled")]
    JITDisabled,
    #[msg("Vault capacity exceeded")]
    VaultCapacityExceeded,
    #[msg("Zero redemption value")]
    ZeroRedemption,
    #[msg("Insufficient vault liquidity")]
    InsufficientVaultLiquidity,
    
    // Order Management Errors
    #[msg("Unauthorized order modification")]
    UnauthorizedOrderModification,
    #[msg("Invalid order ID")]
    InvalidOrderId,
    #[msg("Order is already closed")]
    OrderAlreadyClosed,
    #[msg("Invalid order type for this operation")]
    InvalidOrderType,
    #[msg("Cannot modify flash loan order")]
    CannotModifyFlashOrder,
    #[msg("Cannot modify completed swap")]
    CannotModifyCompletedSwap,
    #[msg("Not a liquidity order")]
    NotLiquidityOrder,
    #[msg("Not a limit order")]
    NotLimitOrder,

    // ========================================================================
    // Authority Errors (300-399): Access control and permission violations
    // ========================================================================

    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Unauthorized guardian")]
    UnauthorizedGuardian,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,

    // ========================================================================
    // Oracle Errors (400-499): Oracle and price feed related errors
    // ========================================================================

    #[msg("Oracle not initialized")]
    OracleNotInitialized,
    #[msg("Invalid oracle")]
    InvalidOracle,
    #[msg("No observations in oracle")]
    NoObservations,
    #[msg("Oracle update too frequent - minimum interval not met")]
    OracleUpdateTooFrequent,
    #[msg("Oracle data is stale - needs update")]
    StaleOracle,
    #[msg("Oracle price deviates too much from pool price")]
    OraclePriceDeviation,

    // ========================================================================
    // System Errors (500-599): Pool and system-level errors
    // ========================================================================

    #[msg("Invalid pool")]
    InvalidPool,
    #[msg("Invalid hook program - must be a valid executable program")]
    InvalidHookProgram,
    #[msg("Hook not found in registry")]
    HookNotFound,
    #[msg("Hook validation failed")]
    HookValidationFailed,
    #[msg("Invalid permission level")]
    InvalidPermission,
    #[msg("Message queue is full")]
    MessageQueueFull,
    
    // Keeper errors
    #[msg("No active ticks found")]
    NoActiveTicks,
    #[msg("No liquidity in pool")]
    NoLiquidity,
    #[msg("Invalid optimality bound")]
    InvalidOptimalityBound,
    #[msg("No tight points found for convex bound")]
    NoTightPoints,
    #[msg("Stale gradient update")]
    StaleGradientUpdate,
    #[msg("Excessive optimality gap")]
    ExcessiveOptimalityGap,
    #[msg("Invalid gradient value")]
    InvalidGradient,
    #[msg("Invalid tick index")]
    InvalidTickIndex,
    #[msg("Insufficient keeper stake")]
    InsufficientKeeperStake,
    #[msg("Too many keepers registered")]
    TooManyKeepers,
    #[msg("Keeper already registered")]
    KeeperAlreadyRegistered,
    #[msg("Keeper not found")]
    KeeperNotFound,
    #[msg("Stake is locked")]
    StakeLocked,
    #[msg("Exit cooldown not complete")]
    ExitCooldownNotComplete,
    #[msg("Keeper not exiting")]
    KeeperNotExiting,
    #[msg("Path too long")]
    PathTooLong,

    // ========================================================================
    // Reserved for Future Use (600-999)
    // ========================================================================
}


impl FeelsProtocolError {
    /// Get the error category for better error handling and logging
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Validation Errors (0-99)
            Self::NotImplemented
            | Self::InvalidMetadata
            | Self::InvalidAmount
            | Self::InvalidTokenAmount
            | Self::InputAmountZero
            | Self::InputAmountExceedsLiquidity
            | Self::OutputAmountZero
            | Self::InvalidPercentage
            | Self::InvalidTickerLength
            | Self::InvalidTickerFormat
            | Self::RestrictedTicker
            | Self::IncompatibleDecimals
            | Self::DecimalsTooLarge
            | Self::InvalidMint
            | Self::SameMintAsUnderlying
            | Self::InvalidTokenPair
            | Self::NotFeelsSOLPair
            | Self::InvalidFeelsSOL
            | Self::InvalidTokenName
            | Self::InvalidTokenSymbol
            | Self::TickerSymbolMismatch
            | Self::InvalidInitialSupply
            | Self::TickOutOfBounds
            | Self::TickNotAligned
            | Self::InvalidTickSpacing
            | Self::InvalidTickRange
            | Self::InvalidTickIndex
            | Self::RateOutOfBounds
            | Self::RateLimitTooAggressive
            | Self::RateLimitOutsideValidRange
            | Self::InvalidRateRange
            | Self::InvalidSqrtRate
            | Self::InvalidRateLimit
            | Self::InvalidLiquidity
            | Self::InvalidLiquidityAmount
            | Self::InvalidFeeRate
            | Self::InvalidSwapDirection
            | Self::SwapAmountTooSmall
            | Self::NonCanonicalTokenOrder
            | Self::InvalidLeverage
            | Self::LeverageExceedsCeiling
            | Self::LeverageCeilingExceedsMax
            | Self::LeverageTooHighForVolatility
            | Self::LeverageTooHighForLiquidity
            | Self::LeverageTooHighForImpact
            | Self::LeverageTooLow
            | Self::LeverageExceedsMaximum
            | Self::MarketConditionsPreventLeverage
            | Self::InvalidRiskProfile
            | Self::InvalidProtectionCurve
            | Self::InvalidObservationTimestamp
            | Self::InvalidPrice
            | Self::PriceManipulationDetected
            | Self::ExtremePrice
            | Self::LogarithmUndefined
            | Self::InvalidTimestamp
            | Self::InvalidDuration
            | Self::MinimumDurationNotMet
            | Self::FlashLoanNotRepaidInSameSlot
            | Self::FlashLoanRepaymentBeforeBorrow
            | Self::FlashLoanAlreadyRepaid => ErrorCategory::Validation,

            // Math Errors (100-199)
            Self::DivisionByZero
            | Self::ArithmeticOverflow
            | Self::ArithmeticUnderflow
            | Self::MathOverflow
            | Self::LiquidityOverflow
            | Self::LiquidityUnderflow => ErrorCategory::Math,

            // State Errors (200-299)
            Self::InvalidVersion
            | Self::InsufficientBalance
            | Self::InsufficientLiquidity
            | Self::PositionNotMatured
            | Self::TickNotFound
            | Self::TickNotInitialized
            | Self::TickArrayNotInitialized
            | Self::TickArrayNotEmpty
            | Self::PoolOperationsPaused
            | Self::PauseNotExpired
            | Self::EmergencyModeActive
            | Self::SlippageExceeded
            | Self::SlippageProtectionTriggered
            | Self::SwapResultsInZeroOutput
            | Self::ReentrancyDetected
            | Self::InvalidReentrancyState
            | Self::MintFailed
            | Self::BurnFailed
            | Self::TokensLocked
            | Self::InvalidTickArrayStart
            | Self::InvalidTickArrayCount
            | Self::InvalidTickArrayIndex
            | Self::InvalidTickArray
            | Self::InvalidTickArrayBoundary
            | Self::LeverageNotEnabled
            | Self::UpdateTooFrequent
            | Self::NoLeverageToRedenominate
            | Self::PositionVaultNotInitialized
            | Self::RedenominationRequired
            | Self::ValenceSessionNotInitialized
            | Self::DynamicFeeError
            | Self::NoPositionFound
            | Self::InsufficientShares
            | Self::ZeroShares
            | Self::ZeroRedemptionValue
            | Self::InvalidTokenForShareType
            | Self::InsufficientFlashLoanLiquidity
            | Self::InsufficientFlashLoanRepayment
            | Self::InvalidPoolForFlashLoan
            | Self::InvalidOperation
            | Self::RouterFull
            | Self::HookRegistryFull
            | Self::VolumeBelowThreshold
            | Self::InsufficientPOL
            | Self::InsufficientUserDeposits
            | Self::WouldBreachBaselineFloor
            | Self::JITDisabled
            | Self::VaultCapacityExceeded
            | Self::ZeroRedemption
            | Self::InsufficientVaultLiquidity
            | Self::UnauthorizedOrderModification
            | Self::InvalidOrderId
            | Self::OrderAlreadyClosed
            | Self::InvalidOrderType
            | Self::CannotModifyFlashOrder
            | Self::CannotModifyCompletedSwap
            | Self::NotLiquidityOrder
            | Self::NotLimitOrder
            | Self::UnauthorizedRedenomination
            | Self::InvalidRedenominationAmount
            | Self::RedenominationCooldownActive
            | Self::ExcessiveRedenominationLoss
            | Self::RedenominationOracleDeviation => ErrorCategory::State,

            // Authority Errors (300-399)
            Self::Unauthorized
            | Self::InvalidAuthority
            | Self::InvalidOwner
            | Self::UnauthorizedGuardian
            | Self::InvalidAccountOwner => ErrorCategory::Authority,

            // Oracle Errors (400-499)
            Self::OracleNotInitialized
            | Self::InvalidOracle
            | Self::NoObservations
            | Self::OracleUpdateTooFrequent
            | Self::StaleOracle
            | Self::OraclePriceDeviation => ErrorCategory::Oracle,

            // System Errors (500-599)
            Self::InvalidPool
            | Self::InvalidHookProgram
            | Self::HookNotFound
            | Self::HookValidationFailed
            | Self::InvalidPermission
            | Self::MessageQueueFull
            | Self::NoActiveTicks
            | Self::NoLiquidity
            | Self::InvalidOptimalityBound
            | Self::NoTightPoints
            | Self::StaleGradientUpdate
            | Self::ExcessiveOptimalityGap
            | Self::InvalidGradient
            | Self::InvalidTickIndex
            | Self::InsufficientKeeperStake
            | Self::TooManyKeepers
            | Self::KeeperAlreadyRegistered
            | Self::KeeperNotFound
            | Self::StakeLocked
            | Self::ExitCooldownNotComplete
            | Self::KeeperNotExiting
            | Self::PathTooLong => ErrorCategory::System,
        }
    }

    /// Check if the error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        match self.category() {
            ErrorCategory::Validation => false, // Input validation errors are not recoverable
            ErrorCategory::Math => false,       // Math errors indicate logic bugs
            ErrorCategory::Authority => false,  // Permission errors require intervention
            ErrorCategory::State => true,       // State errors might be temporary
            ErrorCategory::Oracle => true,      // Oracle errors might be temporary
            ErrorCategory::System => false,     // System errors require intervention
        }
    }

    /// Get error severity level for logging and monitoring
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical errors that indicate serious problems
            Self::MathOverflow
            | Self::ArithmeticOverflow
            | Self::LiquidityOverflow
            | Self::DivisionByZero => ErrorSeverity::Critical,

            // High severity errors that prevent operations
            Self::InsufficientLiquidity
            | Self::SlippageExceeded
            | Self::Unauthorized
            | Self::PoolOperationsPaused
            | Self::EmergencyModeActive => ErrorSeverity::High,

            // Medium severity errors that are expected in normal operation
            Self::InvalidAmount
            | Self::SwapAmountTooSmall
            | Self::InvalidTickRange
            | Self::TickNotInitialized => ErrorSeverity::Medium,

            // Low severity errors that are mostly validation failures
            _ => ErrorSeverity::Low,
        }
    }
}

/// Error categories for systematic error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Validation,
    Math,
    State,
    Authority,
    Oracle,
    System,
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        assert_eq!(FeelsProtocolError::InvalidAmount.category(), ErrorCategory::Validation);
        assert_eq!(FeelsProtocolError::MathOverflow.category(), ErrorCategory::Math);
        assert_eq!(FeelsProtocolError::InsufficientLiquidity.category(), ErrorCategory::State);
        assert_eq!(FeelsProtocolError::Unauthorized.category(), ErrorCategory::Authority);
        assert_eq!(FeelsProtocolError::InvalidOracle.category(), ErrorCategory::Oracle);
        assert_eq!(FeelsProtocolError::InvalidPool.category(), ErrorCategory::System);
    }

    #[test]
    fn test_error_recoverability() {
        // Validation errors are not recoverable
        assert!(!FeelsProtocolError::InvalidAmount.is_recoverable());
        // State errors might be recoverable
        assert!(FeelsProtocolError::InsufficientLiquidity.is_recoverable());
        // Math errors are not recoverable
        assert!(!FeelsProtocolError::MathOverflow.is_recoverable());
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(FeelsProtocolError::MathOverflow.severity(), ErrorSeverity::Critical);
        assert_eq!(FeelsProtocolError::InsufficientLiquidity.severity(), ErrorSeverity::High);
        assert_eq!(FeelsProtocolError::InvalidTickRange.severity(), ErrorSeverity::Medium);
        assert_eq!(FeelsProtocolError::InvalidTickerLength.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_type_consistency() {
        // Ensure error types are properly defined and accessible
        let error1 = FeelsProtocolError::InvalidAmount;
        let error2 = FeelsProtocolError::InvalidMint;
        
        assert_eq!(error1.category(), ErrorCategory::Validation);
        assert_eq!(error2.category(), ErrorCategory::Validation);
    }
}