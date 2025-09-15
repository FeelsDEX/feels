//! Position fee calculation logic
//!
//! Handles fee accrual calculations for liquidity positions

use crate::error::FeelsError;
use crate::state::Tick;

/// Position fee accrual result
#[derive(Debug, Clone, Copy)]
pub struct PositionFeeAccrual {
    /// Current fee growth inside for token 0
    pub fee_growth_inside_0: u128,
    /// Current fee growth inside for token 1
    pub fee_growth_inside_1: u128,
    /// Incremental fees owed for token 0
    pub tokens_owed_0_increment: u64,
    /// Incremental fees owed for token 1
    pub tokens_owed_1_increment: u64,
}

/// Calculate position fee accrual
/// 
/// Given the market globals, tick fee growth outside values, and position's last tracked
/// fee growth inside values, this function computes:
/// - Current fee growth inside the position's range
/// - Incremental fees owed since last update
/// 
/// This follows the Uniswap V3 formula for tracking fees within a position's range.
#[allow(clippy::too_many_arguments)]
pub fn calculate_position_fee_accrual(
    current_tick: i32,
    position_tick_lower: i32,
    position_tick_upper: i32,
    position_liquidity: u128,
    fee_growth_global_0: u128,
    fee_growth_global_1: u128,
    lower_tick: &Tick,
    upper_tick: &Tick,
    last_fee_growth_inside_0: u128,
    last_fee_growth_inside_1: u128,
) -> Result<PositionFeeAccrual, FeelsError> {
    // Get outside fee growth from ticks
    let lower_outside_0 = lower_tick.fee_growth_outside_0_x64;
    let lower_outside_1 = lower_tick.fee_growth_outside_1_x64;
    let upper_outside_0 = upper_tick.fee_growth_outside_0_x64;
    let upper_outside_1 = upper_tick.fee_growth_outside_1_x64;

    // Calculate fee growth inside based on current tick position
    // CRITICAL: This follows Uniswap V3 formula exactly. Do NOT use wrapping arithmetic here!
    // The fee growth inside represents actual accumulated fees within the position range.
    let (fee_growth_inside_0, fee_growth_inside_1) = if current_tick < position_tick_lower {
        // Current price is below the range
        // All fee growth happened above this range
        (
            lower_outside_0.saturating_sub(upper_outside_0),
            lower_outside_1.saturating_sub(upper_outside_1),
        )
    } else if current_tick >= position_tick_upper {
        // Current price is above the range
        // All fee growth happened below this range
        (
            upper_outside_0.saturating_sub(lower_outside_0),
            upper_outside_1.saturating_sub(lower_outside_1),
        )
    } else {
        // Current price is inside the range
        // Fee growth inside = total - below - above
        // Use saturating_sub to prevent underflow
        let fee_inside_0 = fee_growth_global_0
            .saturating_sub(lower_outside_0)
            .saturating_sub(upper_outside_0);
        let fee_inside_1 = fee_growth_global_1
            .saturating_sub(lower_outside_1)
            .saturating_sub(upper_outside_1);
        (fee_inside_0, fee_inside_1)
    };

    // Calculate incremental fees owed since last update
    // NOTE: We use wrapping_sub here because fee growth can legitimately wrap around u128
    // over the lifetime of a pool. The delta calculation handles this correctly.
    let tokens_owed_0_increment = if position_liquidity > 0 {
        let fee_growth_delta_0 = fee_growth_inside_0.wrapping_sub(last_fee_growth_inside_0);
        (fee_growth_delta_0.saturating_mul(position_liquidity) >> 64) as u64
    } else {
        0
    };

    let tokens_owed_1_increment = if position_liquidity > 0 {
        let fee_growth_delta_1 = fee_growth_inside_1.wrapping_sub(last_fee_growth_inside_1);
        (fee_growth_delta_1.saturating_mul(position_liquidity) >> 64) as u64
    } else {
        0
    };

    Ok(PositionFeeAccrual {
        fee_growth_inside_0,
        fee_growth_inside_1,
        tokens_owed_0_increment,
        tokens_owed_1_increment,
    })
}