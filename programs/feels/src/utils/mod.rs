/// Utility module providing mathematical primitives, constants, and helper functions.
/// Organized into specialized sub-modules for different mathematical domains:
/// big integers, liquidity math, tick conversions, and safe arithmetic. These
/// low-level utilities form the foundation for all protocol calculations.
pub mod constant;              // Mathematical constants
pub mod math_safe;            // Overflow-safe arithmetic traits
pub mod math_liquidity;       // Pure mathematical liquidity functions
pub mod math_fee;             // Fee growth Q128.128 arithmetic
pub mod math_u256;            // High-precision U256 operations
pub mod math_big_int;         // U256/U512 implementation
pub mod math_tick;            // Tick-price conversion utilities
pub mod math_general;         // General mathematical functions
pub mod seed;                 // PDA seed generation
pub mod error_handling;       // Error handling utilities

// Re-exports
pub use constant::{
    MIN_TICK, MAX_TICK, Q64, Q96,
    BASIS_POINTS_DENOMINATOR, MAX_PROTOCOL_FEE_RATE, VALID_FEE_TIERS,
};
pub use math_safe::*;
pub use math_liquidity::*;
pub use math_fee::*;
pub use math_u256::*;
pub use math_big_int::*;
pub use math_tick::{
    TickMath, MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64
};
pub use math_general::*;
pub use seed::*;
pub use error_handling::*;