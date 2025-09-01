/// Utility module providing mathematical primitives, constants, CPI helpers, and utility functions.
/// Organized into specialized sub-modules for different mathematical domains and common operations.
/// Includes account pattern abstractions, deterministic seed generation, and specialized math functions.

// ============================================================================
// Module Declarations
// ============================================================================

pub mod account_pattern;        // Reusable account pattern abstractions
pub mod cpi_helpers;           // Cross-Program Invocation helpers
pub mod deterministic_seed;    // PDA seed generation and derivation
pub mod error_handling;        // Error handling utilities
pub mod instruction_pattern;   // Standardized instruction handler patterns
pub mod math;                  // Unified mathematics module
pub mod time_weighted_average; // Time-weighted average buffer for TWAP/TWAV
pub mod token_validation;      // Token ticker validation
pub mod types;                 // Common parameter and result types

// ============================================================================
// Re-exports from Constants
// ============================================================================

pub use crate::constant::BASIS_POINTS_DENOMINATOR;
pub use crate::constant::DURATION_BITS;
pub use crate::constant::LEVERAGE_BITS;
pub use crate::constant::MAX_FEE_RATE;
pub use crate::constant::MAX_HOOKS_PER_TYPE;
pub use crate::constant::MAX_LIQUIDITY_DELTA;
pub use crate::constant::MAX_PROTOCOL_FEE_RATE;
pub use crate::constant::MAX_ROUTER_ARRAYS;
pub use crate::constant::MAX_SQRT_RATE_X96;
pub use crate::constant::MAX_TICK;
pub use crate::constant::MAX_TICK_ARRAYS_PER_SWAP;
pub use crate::constant::MAX_TICK_UPDATES;
pub use crate::constant::MIN_SQRT_RATE_X96;
pub use crate::constant::MIN_TICK;
pub use crate::constant::MIN_SQRT_PRICE_X96;
pub use crate::constant::MAX_SQRT_PRICE_X96;
pub use crate::constant::Q64;
pub use crate::constant::Q96;
pub use crate::constant::RATE_BITS;
pub use crate::constant::TICK_ARRAY_SIZE;
pub use crate::constant::TICK_ARRAY_SIZE_BITS;
pub use crate::constant::VALID_FEE_TIERS;

// ============================================================================
// Re-exports from Math Module
// ============================================================================

pub use error_handling::*;
pub use math::{
    // AMM math types and functions
    amm::{
        get_amount_0_delta, get_amount_1_delta, get_liquidity_for_amount_0,
        get_liquidity_for_amount_1, get_next_sqrt_rate_from_amount_0_rounding_up,
        get_next_sqrt_rate_from_amount_1_rounding_down, q64_to_q96, q96_to_q64, FeeBreakdown,
        FeeConfig, FeeGrowthMath, FeeMath, TickMath,
    },
    // Big integer functions
    big_int::{mul_div, mul_div_rounding_up, Rounding},
    // Q96 fixed-point math
    q96::{calculate_fee_growth_q128, calculate_fee_growth_q128 as calculate_fee_growth_delta},
    // Safe arithmetic functions
    safe::{
        add_i128 as safe_add_i128, add_liquidity_delta, add_u128 as safe_add_u128,
        add_u64 as safe_add_u64, calculate_percentage, div_u128 as safe_div_u128,
        div_u64 as safe_div_u64, mul_div_u64, mul_u128 as safe_mul_u128, mul_u64 as safe_mul_u64,
        sqrt_u128, sqrt_u64, sub_i128 as safe_sub_i128, sub_liquidity_delta,
        sub_u128 as safe_sub_u128, sub_u64 as safe_sub_u64,
    },
    // Big integer types
    U256,
    U512,
};

// Re-export CanonicalSeeds explicitly
pub use deterministic_seed::CanonicalSeeds;

// Re-export types
pub use types::U256Wrapper;

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
// Re-exports from Time-Weighted Averages
// ============================================================================

// Note: token_validate functions are used directly by specific modules, not re-exported globally

pub use time_weighted_average::{
    TimeWeightedObservation, TimeWeightedAverageBuffer,
    PriceObservation, FlashVolumeObservation,
    TimeWeightedMetrics,
};

// ============================================================================
// Re-exports from Instruction Patterns
// ============================================================================

pub use instruction_pattern::{
    InstructionHandler, ValidationUtils, EventBuilder,
    SwapPattern, LiquidityPattern, AdminPattern, StateLoader,
};
