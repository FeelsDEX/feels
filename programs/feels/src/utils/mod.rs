//! # Utility Infrastructure
//! 
//! Support utilities for 3D thermodynamic AMM:
//! 
//! 1. **Math**: Safe arithmetic, fixed-point Q64, AMM math
//! 2. **Physics**: Conservation checks, routing validation
//! 3. **Security**: Reentrancy, staleness, validation
//! 4. **Operations**: Token handling, PDAs, CPI helpers
//! 5. **Development**: Patterns, clock, bitmaps

// ============================================================================
// 1. MATHEMATICAL FOUNDATION - Core Arithmetic & Physics Math
// ============================================================================

/// Math utilities
pub mod math;

/// Conservation law validation
pub mod conservation;

// ============================================================================
// 2. PHYSICS-SPECIFIC UTILITIES - 3D Market Support
// ============================================================================

/// Route validation
pub mod route_validation;

// ============================================================================
// 3. SECURITY & SAFETY INFRASTRUCTURE
// ============================================================================

/// Security utilities
pub mod security;

/// Error handling  
pub mod error_handling;

/// Pattern utilities for account and instruction handling
pub mod patterns;

// ============================================================================
// 4. OPERATIONAL SUPPORT - Token & Account Management
// ============================================================================

/// Token validation
pub mod token_validation;

/// Vault balances
pub mod vault_balance;

/// PDA generation
pub mod deterministic_seed;

/// CPI helpers
pub mod cpi_helpers;

/// Bitmap operations
pub mod bitmap;

// ============================================================================
// Re-exports from Constants
// ============================================================================

pub use feels_core::constants::BASIS_POINTS_DENOMINATOR;
pub use feels_core::constants::DURATION_BITS;
pub use feels_core::constants::LEVERAGE_BITS;
pub use feels_core::constants::MAX_LIQUIDITY_DELTA;
pub use feels_core::constants::MAX_ROUTER_ARRAYS;
pub use feels_core::constants::MAX_TICK;
pub use feels_core::constants::MIN_TICK;
pub use feels_core::constants::Q64;
pub use feels_core::constants::RATE_BITS;
pub use feels_core::constants::TICK_ARRAY_SIZE;

// ============================================================================
// Re-exports from Math Module
// ============================================================================

pub use error_handling::{
    ErrorHandling, create_error_with_context, handle_anchor_error,
};
// Re-export math types and functions that are still in local math module
pub use math::{
    // Safe arithmetic operations module
    safe,
    // Fee calculation types
    FeeBreakdown, FeeConfig, FeeGrowthMath,
    // Fee math module
    fee_math,
};

// Safe math functions are available through math::safe module which handles error conversion

// Re-export from feels-core
pub use feels_core::math::{
    // Big integer types and functions
    U256, Rounding, mul_div,
    // Tick math functions
    get_sqrt_price_at_tick, get_tick_at_sqrt_price,
    // Liquidity math functions  
    get_amount_0_delta, get_amount_1_delta,
    get_liquidity_for_amount_0, get_liquidity_for_amount_1,
    // Fee math functions
    calculate_fee_growth_q64,
};


// Re-export PDA functions from deterministic_seed (which re-exports from state::pda)
pub use deterministic_seed::*;

// Re-export types
// Temporarily commented out due to FixedPoint dependencies
/*
pub use types::{
    U256Wrapper, Position3D, PositionDelta3D, TradeDimension, CellIndex3D,
};
*/

// ============================================================================
// Re-exports from Patterns
// ============================================================================

pub use patterns::{
    // Validation helpers
    validate_amount, validate_bps,
    validate_token_account_owner, validate_token_account_mint,
    validate_pda,
    // Execution helper
    execute_with_phases,
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

// Re-export safe module for convenience
pub use math::safe;

// Re-export commonly used safe math functions with safe_ prefix for backward compatibility
pub use math::safe::{
    add_u64 as safe_add_u64, sub_u64 as safe_sub_u64, mul_u64 as safe_mul_u64, div_u64 as safe_div_u64,
    add_u128 as safe_add_u128, sub_u128 as safe_sub_u128, mul_u128 as safe_mul_u128, div_u128 as safe_div_u128,
    add_i128 as safe_add_i128, sub_i128 as safe_sub_i128, mul_i128 as safe_mul_i128, div_i128 as safe_div_i128,
    add_liquidity_delta, sub_liquidity_delta,
};
