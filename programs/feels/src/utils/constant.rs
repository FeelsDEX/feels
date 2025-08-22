/// Defines protocol-wide constants including tick bounds, fixed-point precision values,
/// and fee parameters. These constants ensure consistent calculations across all
/// protocol operations and match Uniswap V3 specifications for compatibility.
/// Critical for maintaining numerical stability and preventing overflows.
// ============================================================================
// Constants
// ============================================================================
// Tick Constants
pub const MIN_TICK: i32 = -887272;           // Minimum tick value
pub const MAX_TICK: i32 = 887272;            // Maximum tick value
pub const MIN_SQRT_PRICE: u128 = 4295128739; // sqrt(1.0001^-887272) * 2^96
pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055; // Reduced max sqrt price to fit u128

// Fixed-Point Arithmetic Constants
pub const Q96: u128 = 1u128 << 96;          // 2^96 for fixed point math
pub const Q64: u128 = 1u128 << 64;          // 2^64 for fixed point math

// Fee Calculation Constants
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;
pub const MAX_FEE_RATE: u16 = 1_000; // 10% maximum fee rate
pub const MAX_PROTOCOL_FEE_RATE: u16 = 2_500; // 25% maximum protocol share

/// Valid fee tiers (basis points)
pub const VALID_FEE_TIERS: &[u16] = &[1, 5, 30, 100];

// Tick Array Constants
pub const TICK_ARRAY_SIZE: usize = 32;
pub const TICK_ARRAY_SIZE_BITS: u32 = 5; // log2(32)