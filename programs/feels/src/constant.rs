/// Defines protocol-wide constants including tick bounds, fixed-point precision values,
/// and fee parameters. These constants ensure consistent calculations across all
/// protocol operations. Critical for maintaining numerical stability and preventing overflows.

// Tick Constants
pub const MIN_TICK: i32 = -887_272; // Minimum supported tick value
pub const MAX_TICK: i32 = 887_272; // Maximum supported tick value

// Fixed-Point Arithmetic Constants  
pub const Q64: u128 = 1u128 << 64; // 2^64 for fixed point math

// Safe shift constants to prevent arithmetic overflow
pub const Q32: u128 = 1u128 << 32;  // 2^32
pub const Q96: u128 = (1u128 << 32) * (1u128 << 32) * (1u128 << 32); // 2^96 without overflow
pub const Q128_SAFE: U256 = U256::from_limbs([0, 0, 1, 0]); // 2^128 as U256

// Import U256 for safe constants
use crate::utils::math::U256;

// Sqrt Price Constants (Q64.64 format)
/// Minimum sqrt price in Q64.64 format (approximately 2^-60)
pub const MIN_SQRT_RATE_X64: u128 = 1u128 << 4; // Minimum non-zero sqrt price
/// Maximum sqrt price in Q64.64 format (safe u128)
pub const MAX_SQRT_RATE_X64: u128 = 79228162514264337593543950335u128; // Maximum sqrt price

// Fee Calculation Constants
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;
pub const BPS_DENOMINATOR: u64 = 10_000; // Alias for compatibility
pub const MIN_FEE_BPS: u64 = 1; // Minimum fee of 0.01%
pub const MAX_FEE_BPS: u64 = 250; // Maximum fee of 2.5%

// Tick Array Constants
pub const TICK_ARRAY_SIZE: usize = 32; // 32 ticks per array
pub const MAX_ROUTER_ARRAYS: usize = 8; // Maximum number of tick arrays in router

// Pool Constants
pub const RATE_BITS: u8 = 20; // Bits for encoding fee rate
pub const DURATION_BITS: u8 = 6; // Bits for encoding duration
pub const LEVERAGE_BITS: u8 = 6; // Bits for encoding leverage

// Hook Constants
pub const MAX_HOOKS_PER_POOL: usize = 8; // Maximum hooks per pool

// Liquidity Constants
pub const MAX_LIQUIDITY_DELTA: i128 = i128::MAX / 2; // Half of max to leave room for operations

// Routing Constants
pub const MAX_ROUTE_HOPS: usize = 2; // Maximum number of hops in any route (hub-and-spoke bound)
pub const MAX_SEGMENTS_PER_HOP: usize = 10; // Maximum segments within a single hop
pub const MAX_SEGMENTS_PER_TRADE: usize = 20; // Maximum total segments across all hops
