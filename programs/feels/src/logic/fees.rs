//! Fee calculation utilities for MVP (base + impact only)
//!
//! Note: Keep code concise — no legacy/back-compat paths.

/// Minimum and maximum total fee bounds (in basis points)
pub const MIN_TOTAL_FEE_BPS: u16 = 10;   // 0.10%
pub const MAX_TOTAL_FEE_BPS: u16 = 2500; // 25.00%

/// Impact floor in basis points applied to realized impact
pub const IMPACT_FLOOR_BPS: u16 = 10; // 0.10%

/// Precomputed lookup table with enhanced granularity for common trade sizes
const TICK_TO_BPS_TABLE_SMALL: [u16; 11] = [
    0,   // 0 ticks
    10,  // 10 ticks
    20,  // 20 ticks
    30,  // 30 ticks
    40,  // 40 ticks
    50,  // 50 ticks
    60,  // 60 ticks
    70,  // 70 ticks
    81,  // 80 ticks
    91,  // 90 ticks
    100, // 100 ticks
];

/// Standard lookup table up to 2000 ticks
const TICK_TO_BPS_TABLE: [u16; 21] = [
    0,    // 0 ticks
    100,  // 100 ticks
    201,  // 200 ticks
    303,  // 300 ticks
    406,  // 400 ticks
    510,  // 500 ticks
    615,  // 600 ticks
    721,  // 700 ticks
    828,  // 800 ticks
    936,  // 900 ticks
    1046, // 1000 ticks
    1156, // 1100 ticks
    1268, // 1200 ticks
    1381, // 1300 ticks
    1495, // 1400 ticks
    1610, // 1500 ticks
    1726, // 1600 ticks
    1844, // 1700 ticks
    1963, // 1800 ticks
    2083, // 1900 ticks
    2204, // 2000 ticks
];

/// Convert realized tick movement to basis points using lookup tables
pub fn ticks_to_bps(ticks_moved: i32) -> u16 {
    if ticks_moved <= 0 {
        return 0;
    }
    if ticks_moved <= 100 {
        let index = (ticks_moved / 10) as usize;
        return TICK_TO_BPS_TABLE_SMALL[index];
    }
    if ticks_moved <= 2000 {
        let index = (ticks_moved / 100).min(20) as usize;
        return TICK_TO_BPS_TABLE[index];
    }
    // Cap at maximum
    2500
}

/// Calculate impact-only fee bps given realized ticks and floor
pub fn calculate_impact_bps(start_tick: i32, end_tick: i32) -> u16 {
    let moved = (end_tick - start_tick).unsigned_abs() as i32;
    let impact = ticks_to_bps(moved);
    impact.max(IMPACT_FLOOR_BPS)
}

/// Combine base and impact with bounds; returns (total_bps, impact_only_bps)
pub fn combine_base_and_impact(base_fee_bps: u16, impact_bps: u16) -> (u16, u16) {
    let total = (base_fee_bps as u32)
        .saturating_add(impact_bps as u32)
        .clamp(MIN_TOTAL_FEE_BPS as u32, MAX_TOTAL_FEE_BPS as u32) as u16;
    let impact_only = total.saturating_sub(base_fee_bps);
    (total, impact_only)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticks_to_bps_bounds() {
        assert_eq!(ticks_to_bps(0), 0);
        assert!(ticks_to_bps(10) >= 10);
        assert!(ticks_to_bps(100) >= 100);
        assert!(ticks_to_bps(2000) >= 2000);
        assert_eq!(ticks_to_bps(5000), 2500);
    }

    #[test]
    fn test_combine_base_and_impact_clamp() {
        let (t, i) = combine_base_and_impact(30, 0);
        assert_eq!(t, 30);
        assert_eq!(i, 0);
        let (t2, _i2) = combine_base_and_impact(5, 0);
        assert!(t2 >= MIN_TOTAL_FEE_BPS);
        let (t3, _i3) = combine_base_and_impact(2000, 1000);
        assert!(t3 <= MAX_TOTAL_FEE_BPS);
    }
}
