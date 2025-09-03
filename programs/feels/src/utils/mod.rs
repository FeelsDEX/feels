/// Utility module providing mathematical primitives, constants, CPI helpers, and utility functions.
/// Organized into specialized sub-modules for different mathematical domains and common operations.
/// Includes account pattern abstractions, deterministic seed generation, and specialized math functions.

// ============================================================================
// Module Declarations
// ============================================================================

pub mod account_pattern;        // Reusable account pattern abstractions
pub mod bitmap;                // Centralized bitmap operations
pub mod clock;                 // Clock and timestamp utilities
pub mod conservation;          // Centralized conservation law primitives
pub mod cpi_helpers;           // Cross-Program Invocation helpers
pub mod deterministic_seed;    // PDA seed generation and derivation
pub mod error_handling;        // Error handling utilities
pub mod instruction_pattern;   // Standardized instruction handler patterns
pub mod routing;               // Routing validation and hub constraint enforcement
pub mod segment_validation;    // Segment count validation and caps enforcement
pub mod vault_balance;         // SPL token vault balance queries
pub mod math;                  // Unified mathematics module
pub mod security;              // Centralized security utilities and macros
pub mod staleness_errors;      // Enhanced staleness error logging
pub mod token_validation;      // Token ticker validation
pub mod types;                 // Common parameter and result types

// ============================================================================
// Re-exports from Constants
// ============================================================================

pub use crate::constant::BASIS_POINTS_DENOMINATOR;
pub use crate::constant::DURATION_BITS;
pub use crate::constant::LEVERAGE_BITS;
pub use crate::constant::MAX_LIQUIDITY_DELTA;
pub use crate::constant::MAX_ROUTER_ARRAYS;
pub use crate::constant::MAX_TICK;
pub use crate::constant::MIN_TICK;
pub use crate::constant::Q64;
pub use crate::constant::RATE_BITS;
pub use crate::constant::TICK_ARRAY_SIZE;

// ============================================================================
// Re-exports from Math Module
// ============================================================================

pub use error_handling::{
    ErrorHandling, create_error_with_context, handle_anchor_error,
};
pub use math::{
    // Safe arithmetic operations
    safe,
    // Fee calculation types
    FeeBreakdown, FeeConfig, FeeGrowthMath,
    // AMM math types and functions
    amm::{
        get_amount_0_delta, get_amount_1_delta, 
        TickMath,
    },
    // Big integer functions
    big_int::{mul_div, Rounding},
    // Q64 native fee math
    fee_math::{calculate_fee_growth_q64},
    // Fixed-point math has been moved to sdk_math.rs for off-chain use only
    // Safe arithmetic functions
    safe::{
        add_i128 as safe_add_i128, add_liquidity_delta, add_u128 as safe_add_u128,
        add_u64 as safe_add_u64, calculate_percentage, div_u128 as safe_div_u128,
        div_u64 as safe_div_u64, mul_div_u64, mul_u128 as safe_mul_u128, mul_u64 as safe_mul_u64,
        sqrt_u128, sqrt_u64, sub_i128 as safe_sub_i128, sub_liquidity_delta,
        sub_u128 as safe_sub_u128, sub_u64 as safe_sub_u64,
        safe_mul_div_u128, safe_mul_div_u64,
    },
    // Big integer types
    U256,
};


// Re-export CanonicalSeeds explicitly
pub use deterministic_seed::CanonicalSeeds;

// Re-export U512 for big integer operations
pub use math::U512;

// Re-export types
// Temporarily commented out due to FixedPoint dependencies
/*
pub use types::{
    U256Wrapper, Position3D, PositionDelta3D, TradeDimension, CellIndex3D,
};
*/

// ============================================================================
// Re-exports from Account Patterns
// ============================================================================

pub use account_pattern::{
    // Core patterns
    PoolWithVaults, UserTokenAccounts, TickArrayPair,
    // Composite patterns  
    LiquidityOperationContext, SwapContext,
    // Authority patterns
    PoolAuthorityContext, ProtocolAuthorityContext,
    // Position patterns
    ValidatedPosition,
    // Phase 2 patterns
    OracleContext, FeelsSOLContext,
    // Program collections
    BasicPrograms, ExtendedPrograms,
    // Enhanced validation patterns
    ValidatedTickArrayPair, UserOwnedPosition, HookExecutionContext,
    UserTokenPair, PoolWithValidatedVaults, CompleteLiquidityContext,
};

// ============================================================================
// Time-Weighted Averages now consolidated in state/twap_oracle.rs
// ============================================================================

// Note: TWAP/TWAV functionality is now provided by state/twap_oracle module

// ============================================================================
// Re-exports from Instruction Patterns
// ============================================================================

pub use instruction_pattern::{
    InstructionHandler, ValidationUtils,
    SwapPattern, LiquidityPattern, AdminPattern, StateLoader,
};

// ============================================================================
// Re-exports from Security Module
// ============================================================================

pub use security::{
    validate_initialized, validate_bounds, validate_freshness,
    validate_rate_of_change, validate_swap_params, validate_liquidity_params,
    ScopedSecurityGuard,
};

// Re-export vault balance utilities
pub use vault_balance::{get_vault_balance, get_vault_balances};
