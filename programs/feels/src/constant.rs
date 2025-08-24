/// Defines protocol-wide constants including tick bounds, fixed-point precision values,
/// and fee parameters. These constants ensure consistent calculations across all
/// protocol operations and match Uniswap V3 specifications for compatibility.
/// Critical for maintaining numerical stability and preventing overflows.
// Tick Constants
// Note: While theoretical range is ±887272, implementation supports ±443636
pub const MIN_TICK: i32 = -443_636;           // Minimum supported tick value
pub const MAX_TICK: i32 = 443_636;            // Maximum supported tick value
pub const MIN_SQRT_PRICE_X96: u128 = 18447090763469684736; // Actual minimum sqrt price for tick -443636
pub const MAX_SQRT_PRICE_X96: u128 = 340_275_971_719_517_849_884_101_479_037_289_023_427; // Actual maximum sqrt price for tick 443636

// Fixed-Point Arithmetic Constants
pub const Q96: u128 = 1u128 << 96; // 2^96 for fixed point math
pub const Q64: u128 = 1u128 << 64; // 2^64 for fixed point math

// Fee Calculation Constants
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;
pub const MAX_FEE_RATE: u16 = 1_000;          // 10% maximum fee rate
pub const MAX_PROTOCOL_FEE_RATE: u16 = 2_500; // 25% maximum protocol share

/// Valid fee tiers (basis points)
pub const VALID_FEE_TIERS: &[u16] = &[1, 5, 30, 100]; // Basis points

// Tick Array Constants
pub const TICK_ARRAY_SIZE: usize = 32;   // 32 ticks per array
pub const TICK_ARRAY_SIZE_BITS: u32 = 5; // log2(32)
pub const MAX_ROUTER_ARRAYS: usize = 8;  // Maximum number of tick arrays in router
pub const MAX_TICK_ARRAYS_PER_SWAP: usize = 100; // Maximum tick arrays traversed in one swap

// Pool Constants
pub const RATE_BITS: u8 = 20;      // Bits for encoding fee rate
pub const DURATION_BITS: u8 = 6;   // Bits for encoding duration
pub const LEVERAGE_BITS: u8 = 6;   // Bits for encoding leverage
pub const MAX_TICK_UPDATES: usize = 20; // Maximum tick updates in a batch

// Hook Constants
pub const MAX_HOOKS_PER_TYPE: usize = 4; // Maximum hooks per hook type
// Size of each hook config in bytes (kept for future use)
#[allow(dead_code)]
pub const HOOK_CONFIG_SIZE: usize = 64;

// Liquidity Constants
pub const MAX_LIQUIDITY_DELTA: i128 = i128::MAX / 2; // Half of max to leave room for operations