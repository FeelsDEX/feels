//! Error definitions

use anchor_lang::prelude::*;

#[error_code]
pub enum FeelsError {
    // Market errors
    #[msg("Market is not initialized")]
    MarketNotInitialized,

    #[msg("Market is paused")]
    MarketPaused,

    #[msg("Invalid market authority")]
    InvalidAuthority,

    #[msg("Invalid market")]
    InvalidMarket,

    // Math errors
    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Division by zero")]
    DivisionByZero,

    #[msg("Invalid price")]
    InvalidPrice,

    // Routing errors
    #[msg("Invalid route: All swaps must route through FeelsSOL hub. Direct swaps between non-FeelsSOL tokens are not supported.")]
    InvalidRoute,

    #[msg("Route too long - maximum 2 hops")]
    RouteTooLong,

    // Token errors
    #[msg("Invalid token mint")]
    InvalidMint,

    #[msg("Invalid token order: FeelsSOL must be token_0. When creating a market, FeelsSOL should always be the first token (token_0) in the pair.")]
    InvalidTokenOrder,

    #[msg("Insufficient balance")]
    InsufficientBalance,

    // Swap errors
    #[msg("Slippage exceeded")]
    SlippageExceeded,

    #[msg("Invalid swap direction")]
    InvalidSwapDirection,

    #[msg("Zero amount")]
    ZeroAmount,

    // Buffer errors
    #[msg("Insufficient buffer balance")]
    InsufficientBufferBalance,

    // Liquidity errors
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,

    // Tick errors
    #[msg("Tick must be a multiple of tick spacing")]
    TickNotSpaced,

    #[msg("Invalid tick range")]
    InvalidTickRange,

    // Invalid vault
    #[msg("Invalid vault")]
    InvalidVault,

    #[msg("Invalid buffer")]
    InvalidBuffer,

    // Position errors
    #[msg("Invalid position")]
    InvalidPosition,

    #[msg("Invalid tick")]
    InvalidTick,

    #[msg("Invalid tick spacing")]
    InvalidTickSpacing,

    #[msg("Zero liquidity")]
    ZeroLiquidity,
    #[msg("Liquidity below minimum threshold")]
    LiquidityBelowMinimum,

    #[msg("Invalid tick array")]
    InvalidTickArray,

    #[msg("Tick array not found for required tick range")]
    TickArrayNotFound,

    // Oracle errors
    #[msg("Oracle not initialized")]
    OracleNotInitialized,

    #[msg("Invalid timestamp")]
    InvalidTimestamp,

    #[msg("Insufficient oracle data")]
    OracleInsufficientData,

    #[msg("Insufficient TWAP duration - minimum 60 seconds required")]
    InsufficientTWAPDuration,

    #[msg("Invalid oracle cardinality")]
    InvalidOracleCardinality,

    #[msg("Invalid oracle account")]
    InvalidOracle,

    #[msg("Too many swap steps exceeded. Try reducing swap amount or providing more tick arrays")]
    TooManySteps,

    #[msg("Too many ticks crossed. Maximum allowed is 200 ticks per swap")]
    TooManyTicksCrossed,

    #[msg("Missing tick array coverage for swap path. Please provide additional tick arrays in the expected price range")]
    MissingTickArrayCoverage,

    #[msg("Vaults have already been initialized")]
    VaultsAlreadyInitialized,

    #[msg("Too many tick arrays provided. Maximum allowed is 10 per swap")]
    TooManyTickArrays,

    #[msg("Re-entrancy detected. Another operation is in progress")]
    ReentrancyDetected,

    #[msg("Position must be empty (liquidity = 0) before it can be closed")]
    PositionNotEmpty,

    #[msg("Position has unclaimed fees that must be collected before closing")]
    UnclaimedFees,

    #[msg("Cannot close position account with uncollected fees. Call collect_fees first or use close_account: false")]
    CannotCloseWithFees,

    // Initialization errors
    #[msg("Vaults not initialized")]
    VaultsNotInitialized,

    #[msg("Oracle already initialized")]
    OracleAlreadyInitialized,

    #[msg("Unauthorized signer - only market authority can perform this operation")]
    UnauthorizedSigner,

    #[msg("Lower tick fee update required before upper tick")]
    LowerTickNotUpdated,

    #[msg("No tokens owed to collect")]
    NoTokensOwed,

    #[msg("Token-2022 is not supported in this version")]
    Token2022NotSupported,

    #[msg("Invalid token program ID")]
    InvalidTokenProgramId,

    #[msg("Only protocol-minted tokens can create markets")]
    TokenNotProtocolMinted,

    #[msg("Invalid market: One token must be FeelsSOL. All markets require FeelsSOL as one of the tokens due to the hub-and-spoke architecture.")]
    RequiresFeelsSOLPair,

    #[msg("Token not found in protocol registry")]
    TokenNotInRegistry,

    #[msg("Token has not expired yet")]
    TokenNotExpired,

    #[msg("Pool already exists in registry")]
    PoolAlreadyExists,

    #[msg("Pool not found in registry")]
    PoolNotFound,

    #[msg("Market already has active liquidity")]
    MarketAlreadyActive,

    #[msg("Invalid account")]
    InvalidAccount,

    #[msg("Market already initialized")]
    MarketAlreadyInitialized,

    // Fee cap errors
    #[msg("Computed fee exceeds caller-provided maximum total fee bps")]
    FeeCapExceeded,

    // Rate limit errors
    #[msg("Rate limit exceeded for this slot")]
    RateLimitExceeded,

    // Vault validation errors
    #[msg("Invalid vault mint")]
    InvalidVaultMint,

    // Project mint errors
    #[msg("Invalid project mint")]
    InvalidProjectMint,
    
    // POMM errors
    #[msg("POMM cooldown is active, please wait before next operation")]
    PommCooldownActive,
    
    #[msg("Insufficient buffer fees for POMM operation")]
    InsufficientBufferFees,
    
    #[msg("Invalid position index")]
    InvalidPositionIndex,
    
    #[msg("Feature not implemented")]
    NotImplemented,
    
    // Phase errors
    #[msg("Invalid market phase")]
    InvalidPhase,
    
    #[msg("Invalid phase transition")]
    InvalidPhaseTransition,
    
    #[msg("Graduation criteria not met")]
    GraduationCriteriaNotMet,
    
    #[msg("Invalid protocol configuration")]
    InvalidProtocol,
    
    #[msg("Invalid buffer vault")]
    InvalidBufferVault,
    
    #[msg("Unauthorized access")]
    Unauthorized,
}
