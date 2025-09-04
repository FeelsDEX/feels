//! # Protocol Constants
//! 
//! Fundamental constants for the 3D AMM including:
//! - Mathematical constants (Q64, Q32, Q128)
//! - Market physics bounds (ticks, prices)
//! - Hub architecture limits (routes, segments)
//! - Fee structure parameters
//! - Duration and leverage constants
//! - Oracle and TWAP parameters
//! - Validation thresholds

// ============================================================================
// Mathematical Constants
// ============================================================================

/// Q64 fixed-point scale factor: 2^64
pub const Q64: u128 = 1u128 << 64;

/// Q32 format for intermediate calculations  
pub const Q32: u128 = 1u128 << 32;

/// Q96 format for sqrt price calculations
pub const Q96: u128 = 1u128 << 96;

/// Q128 lower half (64 bits)
pub const Q128_LO: u128 = 1u128 << 64;

/// Q128 upper half (would be 2^128, but that overflows u128)
/// This represents the maximum value for u128
pub const Q128_HI: u128 = u128::MAX;

/// Maximum value for u128 calculations
pub const U128_MAX: u128 = u128::MAX;

/// Basis points denominator (10,000 = 100%)
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Maximum percentage in basis points (100%)
pub const MAX_BPS: u64 = 10_000;

// ============================================================================
// Market Physics Constants
// ============================================================================

/// Minimum tick in 3D market space
pub const MIN_TICK: i32 = -887_272;

/// Maximum tick in 3D market space
pub const MAX_TICK: i32 = 887_272;

/// Tick at price 1.0001
pub const TICK_BASE: i32 = 1;

/// Maximum tick spacing
pub const MAX_TICK_SPACING: i16 = 32767;

/// Minimum tick spacing
pub const MIN_TICK_SPACING: i16 = 1;

/// Minimum sqrt price in Q64 format
pub const MIN_SQRT_PRICE_X64: u128 = 1u128 << 4;

/// Maximum sqrt price in Q64 format
pub const MAX_SQRT_PRICE_X64: u128 = 79228162514264337593543950335u128;

// ============================================================================
// Fee Structure Constants
// ============================================================================

/// Minimum fee (0.01%)
pub const MIN_FEE_BPS: u64 = 1;

/// Maximum instantaneous fee cap (2.5%)
pub const MAX_FEE_BPS: u64 = 250;

/// Default base fee rate (0.3%)
pub const DEFAULT_BASE_FEE_BPS: u16 = 30;

/// Default protocol fee share (10%)
pub const DEFAULT_PROTOCOL_FEE_SHARE: u16 = 1000;

/// Maximum protocol fee share (25%)
pub const MAX_PROTOCOL_FEE_SHARE: u16 = 2500;

/// Maximum rebate per transaction (basis points)
pub const MAX_REBATE_PER_TX_BPS: u32 = 100;

/// Maximum rebate per epoch (basis points)
pub const MAX_REBATE_PER_EPOCH_BPS: u32 = 500;

// ============================================================================
// Hub Architecture Constants
// ============================================================================

/// Maximum route hops (hub-and-spoke topology)
pub const MAX_ROUTE_HOPS: usize = 2;

/// Maximum segments per hop
pub const MAX_SEGMENTS_PER_HOP: usize = 10;

/// Maximum total segments per trade
pub const MAX_SEGMENTS_PER_TRADE: usize = 20;

// ============================================================================
// Tick Array Constants
// ============================================================================

/// Ticks per array for efficient storage
pub const TICK_ARRAY_SIZE: usize = 32;

/// Maximum tick arrays in router
pub const MAX_ROUTER_ARRAYS: usize = 8;

// ============================================================================
// Market Encoding Constants
// ============================================================================

/// Bits for rate dimension encoding
pub const RATE_BITS: u8 = 20;

/// Bits for time/duration dimension
pub const DURATION_BITS: u8 = 6;

/// Bits for leverage dimension
pub const LEVERAGE_BITS: u8 = 6;

// ============================================================================
// Field Commitment Constants
// ============================================================================

/// Default field commitment staleness threshold (30 minutes)
pub const DEFAULT_COMMITMENT_STALENESS: i64 = 1800;

/// Maximum field commitment staleness (2 hours)
pub const MAX_COMMITMENT_STALENESS: i64 = 7200;

/// Minimum sequence number
pub const MIN_SEQUENCE: u64 = 1;

/// Maximum volatility in basis points (1000%)
pub const MAX_VOLATILITY_BPS: u64 = 100_000;

// ============================================================================
// Account Size Constants
// ============================================================================

/// Anchor discriminator size (8 bytes)
pub const DISCRIMINATOR_SIZE: usize = 8;

/// Size of a Pubkey (32 bytes)
pub const PUBKEY_SIZE: usize = 32;

/// Size of a signature (64 bytes)
pub const SIGNATURE_SIZE: usize = 64;

// ============================================================================
// Work Calculation Constants
// ============================================================================

/// Maximum work value
pub const MAX_WORK: i128 = i128::MAX;

/// Minimum work value
pub const MIN_WORK: i128 = i128::MIN;

/// Work calculation precision (fixed-point)
pub const WORK_PRECISION: u32 = 18;

/// Default work decay rate (basis points)
pub const DEFAULT_WORK_DECAY_BPS: u64 = 100; // 1%

// ============================================================================
// Liquidity and Position Constants
// ============================================================================

/// Minimum liquidity amount
pub const MIN_LIQUIDITY: u128 = 1000;

/// Maximum liquidity per tick
pub const MAX_LIQUIDITY_PER_TICK: u128 = (u128::MAX) / 2;

/// Liquidity precision
pub const LIQUIDITY_PRECISION: u32 = 12;

/// Maximum liquidity change per operation
pub const MAX_LIQUIDITY_DELTA: i128 = i128::MAX / 2;

/// Maximum number of positions per user
pub const MAX_POSITIONS_PER_USER: u32 = 100;

/// Position NFT collection size
pub const POSITION_COLLECTION_SIZE: u64 = 1_000_000;

// ============================================================================
// Duration Constants (in seconds)
// ============================================================================

/// Flash duration (single transaction)
pub const FLASH_DURATION: i64 = 0;

/// Short duration (1 hour)
pub const SHORT_DURATION: i64 = 3600;

/// Medium duration (1 day)
pub const MEDIUM_DURATION: i64 = 86400;

/// Long duration (1 week)
pub const LONG_DURATION: i64 = 604800;

/// Extended duration (1 month, 30 days)
pub const EXTENDED_DURATION: i64 = 2592000;

// ============================================================================
// Leverage Constants
// ============================================================================

/// Leverage scale (1.000x = 1000)
pub const LEVERAGE_SCALE: u64 = 1000;

/// Maximum leverage (10.0x)
pub const MAX_LEVERAGE: u64 = 10000;

/// Minimum margin requirement (10%)
pub const MIN_MARGIN_REQUIREMENT_BPS: u64 = 1000;

/// Default liquidation threshold (80%)
pub const DEFAULT_LIQUIDATION_THRESHOLD_BPS: u64 = 8000;

// ============================================================================
// Oracle and TWAP Constants
// ============================================================================

/// TWAP window (5 minutes)
pub const TWAP_WINDOW_SECONDS: i64 = 300;

/// Maximum TWAP age before considered stale (30 minutes)
pub const MAX_TWAP_AGE: i64 = 1800;

/// TWAP update threshold (0.1% price change)
pub const TWAP_UPDATE_THRESHOLD_BPS: u64 = 10;

// ============================================================================
// Safety Bounds
// ============================================================================

/// Maximum scalar change per update (basis points)
pub const MAX_SCALAR_CHANGE_BPS: u32 = 200; // 2%

/// Minimum time between field updates (seconds)
pub const MIN_UPDATE_INTERVAL: i64 = 60;

/// Maximum update staleness (seconds)
pub const MAX_UPDATE_STALENESS: i64 = 300; // 5 minutes

// ============================================================================
// Network and Validation Constants
// ============================================================================

/// Default RPC timeout (30 seconds)
pub const DEFAULT_RPC_TIMEOUT: u64 = 30;

/// Maximum RPC retries
pub const MAX_RPC_RETRIES: u32 = 3;

/// Default keeper update interval (5 minutes)
pub const DEFAULT_KEEPER_UPDATE_INTERVAL: i64 = 300;

/// Maximum slippage tolerance (50%)
pub const MAX_SLIPPAGE_BPS: u64 = 5000;

/// Default slippage tolerance (1%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 100;

/// Price impact warning threshold (5%)
pub const PRICE_IMPACT_WARNING_BPS: u64 = 500;

/// Maximum price impact (20%)
pub const MAX_PRICE_IMPACT_BPS: u64 = 2000;

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert basis points to fraction
pub const fn bps_to_fraction(bps: u64) -> f64 {
    bps as f64 / BPS_DENOMINATOR as f64
}

/// Check if a number is a power of two
pub const fn is_power_of_two(n: i16) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Calculate percentage from basis points
pub const fn bps_to_percentage(bps: u64) -> f64 {
    bps as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_validity() {
        assert!(MIN_TICK < MAX_TICK);
        assert!(MIN_FEE_BPS < MAX_FEE_BPS);
        assert!(DEFAULT_PROTOCOL_FEE_SHARE <= MAX_PROTOCOL_FEE_SHARE);
        assert_eq!(Q64, 18446744073709551616u128);
        assert_eq!(BPS_DENOMINATOR, 10000);
    }

    #[test]
    fn test_helper_functions() {
        assert_eq!(bps_to_fraction(5000), 0.5);
        assert_eq!(bps_to_percentage(500), 5.0);
        assert!(is_power_of_two(8));
        assert!(!is_power_of_two(10));
    }

    #[test]
    fn test_duration_constants() {
        assert_eq!(FLASH_DURATION, 0);
        assert_eq!(SHORT_DURATION, 3600);
        assert!(MEDIUM_DURATION > SHORT_DURATION);
        assert!(LONG_DURATION > MEDIUM_DURATION);
        assert!(EXTENDED_DURATION > LONG_DURATION);
    }
}