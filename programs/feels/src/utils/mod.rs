/// Utility module providing mathematical primitives, constants, CPI helpers, and utility functions.
/// Organized into specialized sub-modules for different mathematical domains and common operations.
pub mod math;                 // Unified mathematics module
pub mod seed;                 // PDA seed generation and derivation
pub mod error_handling;       // Error handling utilities
pub mod token_validate;       // Token ticker validation
pub mod types;                // Common parameter and result types
pub mod cpi_helpers;          // Cross-Program Invocation helpers

// Re-exports from constant module
pub use crate::constant::MIN_TICK;
pub use crate::constant::MAX_TICK;
pub use crate::constant::Q64;
pub use crate::constant::Q96;
pub use crate::constant::BASIS_POINTS_DENOMINATOR;
pub use crate::constant::MAX_FEE_RATE;
pub use crate::constant::MAX_PROTOCOL_FEE_RATE;
pub use crate::constant::VALID_FEE_TIERS;
pub use crate::constant::TICK_ARRAY_SIZE;
pub use crate::constant::TICK_ARRAY_SIZE_BITS;
pub use crate::constant::MAX_ROUTER_ARRAYS;
pub use crate::constant::MAX_TICK_ARRAYS_PER_SWAP;
pub use crate::constant::RATE_BITS;
pub use crate::constant::DURATION_BITS;
pub use crate::constant::LEVERAGE_BITS;
pub use crate::constant::MAX_TICK_UPDATES;
pub use crate::constant::MAX_HOOKS_PER_TYPE;
pub use crate::constant::MAX_LIQUIDITY_DELTA;
pub use crate::constant::MIN_SQRT_PRICE_X96;
pub use crate::constant::MAX_SQRT_PRICE_X96;

// Re-exports from unified math module
pub use math::{
    // Big integer types
    U256, U512,
    // Big integer functions
    big_int::{Rounding, mul_div, mul_div_rounding_up},
    // Safe arithmetic functions
    safe::{
        add_u64 as safe_add_u64, sub_u64 as safe_sub_u64, 
        mul_u64 as safe_mul_u64, div_u64 as safe_div_u64,
        add_u128 as safe_add_u128, sub_u128 as safe_sub_u128,
        mul_u128 as safe_mul_u128, div_u128 as safe_div_u128,
        add_i128 as safe_add_i128, sub_i128 as safe_sub_i128,
        add_liquidity_delta, sub_liquidity_delta,
        calculate_percentage, mul_div_u64,
    },
    // AMM math types and functions
    amm::{
        TickMath,
        FeeBreakdown, FeeMath, FeeConfig, FeeGrowthMath,
        get_amount_0_delta, get_amount_1_delta,
        get_liquidity_for_amount_0, get_liquidity_for_amount_1,
        get_next_sqrt_price_from_amount_0_rounding_up, get_next_sqrt_price_from_amount_1_rounding_down,
        q96_to_q64, q64_to_q96,
    },
    // Q96 fixed-point math
    q96::{calculate_fee_growth_q128, calculate_fee_growth_q128 as calculate_fee_growth_delta},
};
pub use seed::*;
pub use error_handling::*;
// token_validate functions are used directly by specific modules, not re-exported globally