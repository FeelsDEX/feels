/// Comprehensive error definitions for all protocol operations and edge cases.
/// Provides clear, actionable error messages for debugging transaction failures.
/// Organized into general protocol errors and pool-specific errors to help
/// developers quickly identify and resolve issues in their integrations.
use anchor_lang::prelude::*;

// ============================================================================
// General Protocol Errors
// ============================================================================

#[error_code]
pub enum FeelsError {
    #[msg("Invalid metadata format")]
    InvalidMetadata,
    #[msg("Insufficient token balance")]
    InsufficientBalance,
    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Invalid token amount")]
    InvalidAmount,
    #[msg("Token mint operation failed")]
    MintFailed,
    #[msg("Token burn operation failed")]
    BurnFailed,
}

// ============================================================================
// Pool-Specific Errors
// ============================================================================

#[error_code]
pub enum PoolError {
    // Version and Configuration Errors
    #[msg("Invalid pool version")]
    InvalidVersion,
    #[msg("Invalid FeelsSOL mint")]
    InvalidFeelsSOL,
    #[msg("Invalid tick spacing")]
    InvalidTickSpacing,
    #[msg("Invalid fee rate")]
    InvalidFeeRate,

    // Tick and Price Errors
    #[msg("Tick out of bounds")]
    TickOutOfBounds,
    #[msg("Tick not found")]
    TickNotFound,
    #[msg("Tick not aligned to spacing")]
    TickNotAligned,
    #[msg("Tick not initialized")]
    TickNotInitialized,
    #[msg("Price out of bounds")]
    PriceOutOfBounds,
    #[msg("Price limit too aggressive - would result in no liquidity")]
    PriceLimitTooAggressive,
    #[msg("Price limit outside valid protocol range")]
    PriceLimitOutsideValidRange,
    #[msg("Invalid price range")]
    InvalidPriceRange,
    #[msg("Invalid tick range")]
    InvalidTickRange,
    #[msg("Invalid tick index")]
    InvalidTickIndex,

    // Liquidity Errors
    #[msg("Liquidity overflow")]
    LiquidityOverflow,
    #[msg("Liquidity underflow")]
    LiquidityUnderflow,
    #[msg("Insufficient liquidity in pool for swap")]
    InsufficientLiquidity,
    #[msg("Invalid liquidity amount")]
    InvalidLiquidityAmount,
    #[msg("Input amount is zero")]
    InputAmountZero,
    #[msg("Input amount exceeds available liquidity")]
    InputAmountExceedsLiquidity,
    #[msg("Output amount is zero")]
    OutputAmountZero,
    #[msg("Invalid amount")]
    InvalidAmount,

    // Tick Array Errors
    #[msg("Invalid tick array start")]
    InvalidTickArrayStart,
    #[msg("Invalid tick array count")]
    InvalidTickArrayCount,
    #[msg("Tick array is not empty")]
    TickArrayNotEmpty,
    #[msg("Tick array not initialized")]
    TickArrayNotInitialized,
    #[msg("Invalid tick array index")]
    InvalidTickArrayIndex,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,

    // Swap Errors
    #[msg("Slippage exceeded - output amount below minimum")]
    SlippageExceeded,
    #[msg("Slippage protection triggered - price moved beyond limit")]
    SlippageProtectionTriggered,
    #[msg("Invalid swap direction")]
    InvalidSwapDirection,
    #[msg("Swap amount too small - below minimum threshold")]
    SwapAmountTooSmall,
    #[msg("Swap would result in zero output")]
    SwapResultsInZeroOutput,
    #[msg("Invalid token pair for swap")]
    InvalidTokenPair,

    // Math Errors
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Arithmetic underflow")]
    ArithmeticUnderflow,
    #[msg("Math overflow")]
    MathOverflow,

    // Access Control Errors
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Invalid pool")]
    InvalidPool,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Unauthorized guardian")]
    UnauthorizedGuardian,

    // Circuit Breaker Errors
    #[msg("Pool operations paused")]
    PoolOperationsPaused,
    #[msg("Pause not expired")]
    PauseNotExpired,
    #[msg("Emergency mode active")]
    EmergencyModeActive,

    // Fee and Flash Loan Errors
    #[msg("Not FeelsSOL pair")]
    NotFeelsSOLPair,
    #[msg("Insufficient flash loan liquidity")]
    InsufficientFlashLoanLiquidity,
    #[msg("Insufficient flash loan repayment")]
    InsufficientFlashLoanRepayment,
    #[msg("Invalid pool for flash loan")]
    InvalidPoolForFlashLoan,

    // Transient Update Errors
    #[msg("Transient updates batch is full")]
    TransientUpdatesFull,
    #[msg("Invalid operation on finalized updates")]
    InvalidOperation,
    #[msg("Updates already finalized")]
    UpdatesAlreadyFinalized,
    #[msg("Transient updates expired")]
    TransientUpdatesExpired,

    // Position Vault Errors (Phase 2)
    #[msg("Insufficient POL")]
    InsufficientPOL,
    #[msg("Insufficient user deposits")]
    InsufficientUserDeposits,
    #[msg("Would breach baseline floor")]
    WouldBreachBaselineFloor,
    #[msg("JIT disabled")]
    JITDisabled,
    #[msg("Volume below threshold")]
    VolumeBelowThreshold,
}
