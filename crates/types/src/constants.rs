/// Protocol constants used across the Feels ecosystem

// ============================================================================
// Mathematical Constants
// ============================================================================

/// Q64 fixed-point scale factor: 2^64
pub const Q64: u128 = 1u128 << 64;

/// Q128 lower half (64 bits)
pub const Q128_LO: u128 = 1u128 << 64;

/// Q128 upper half (128 bits)  
pub const Q128_HI: u128 = 1u128 << 128;

/// Maximum value for u128 calculations
pub const U128_MAX: u128 = u128::MAX;

/// Basis points denominator (10,000 = 100%)
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Maximum percentage in basis points (100%)
pub const MAX_BPS: u64 = 10_000;

// ============================================================================
// Tick and Price Constants
// ============================================================================

/// Minimum tick value
pub const MIN_TICK: i32 = -443_636;

/// Maximum tick value  
pub const MAX_TICK: i32 = 443_636;

/// Tick at price 1.0001
pub const TICK_BASE: i32 = 1;

/// Maximum tick spacing
pub const MAX_TICK_SPACING: i16 = 32767;

/// Minimum tick spacing
pub const MIN_TICK_SPACING: i16 = 1;

// ============================================================================
// Fee Constants
// ============================================================================

/// Minimum fee in basis points (0.01%)
pub const MIN_FEE_BPS: u64 = 1;

/// Maximum fee in basis points (10%)
pub const MAX_FEE_BPS: u64 = 1000;

/// Default protocol fee share (10%)
pub const DEFAULT_PROTOCOL_FEE_SHARE: u16 = 1000;

/// Maximum protocol fee share (25%)
pub const MAX_PROTOCOL_FEE_SHARE: u16 = 2500;

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
// Liquidity Constants
// ============================================================================

/// Minimum liquidity amount
pub const MIN_LIQUIDITY: u128 = 1000;

/// Maximum liquidity per tick
pub const MAX_LIQUIDITY_PER_TICK: u128 = (u128::MAX) / 2;

/// Liquidity precision
pub const LIQUIDITY_PRECISION: u32 = 12;

// ============================================================================
// Position Constants
// ============================================================================

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
// Network Constants
// ============================================================================

/// Default RPC timeout (30 seconds)
pub const DEFAULT_RPC_TIMEOUT: u64 = 30;

/// Maximum RPC retries
pub const MAX_RPC_RETRIES: u32 = 3;

/// Default keeper update interval (5 minutes)
pub const DEFAULT_KEEPER_UPDATE_INTERVAL: i64 = 300;

// ============================================================================
// Validation Constants
// ============================================================================

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