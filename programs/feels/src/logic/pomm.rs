//! Pool-Owned Market Making (POMM) logic
//!
//! Opportunistic liquidity placement from pool buffer fees

use crate::{
    constants::MIN_LIQUIDITY,
    error::FeelsError,
    state::{Buffer, Market, OracleState},
    utils::{liquidity_from_amounts, sqrt_price_from_tick},
};
use anchor_lang::prelude::*;

/// Opportunistic upkeep: if pool buffer meets threshold, convert accounting τ into
/// additional floor liquidity at a wide range around current price/tick.
///
/// This function ensures POMM security by:
/// 1. Using only buffer fee accounting, never vault balances, to prevent flash loan attacks
/// 2. Using TWAP instead of spot price for liquidity placement to prevent price manipulation
///    attacks. An attacker cannot manipulate TWAP within a single transaction or block.
pub fn maybe_pomm_add_liquidity(
    market: &mut Account<Market>,
    buffer: &mut Buffer,
    oracle: &OracleState,
    now: i64,
) -> Result<()> {
    // Simple guard: avoid doing this too often
    if now <= buffer.last_floor_placement + 60 {
        // at most once per minute
        return Ok(());
    }

    // Check if buffer has accumulated enough fees to trigger floor placement
    // Use buffer's fee accounting directly instead of vault balances to prevent manipulation
    //
    // To avoid u128 saturation edge case where both fees are near u128::MAX,
    // we check if either individual fee amount exceeds the threshold first.
    // This prevents the extremely unlikely scenario where saturation at u128::MAX
    // would cause unintended frequent POMM executions.
    let threshold_u128 = buffer.floor_placement_threshold as u128;

    // If either individual fee exceeds threshold, we definitely have enough
    if buffer.fees_token_0 >= threshold_u128 || buffer.fees_token_1 >= threshold_u128 {
        // Proceed with POMM logic
    } else {
        // Otherwise, check the sum (using saturating_add for safety)
        let total_fees = buffer.fees_token_0.saturating_add(buffer.fees_token_1);
        if total_fees < threshold_u128 {
            return Ok(());
        }
    }

    // Use all available buffer liquidity based on what we actually have
    let amount_0 = buffer.fees_token_0.min(u64::MAX as u128) as u64;
    let amount_1 = buffer.fees_token_1.min(u64::MAX as u128) as u64;

    if amount_0 == 0 && amount_1 == 0 {
        return Ok(());
    }

    // Use TWAP price instead of spot price to prevent manipulation attacks
    // Calculate average price over the last 5 minutes (300 seconds)
    let twap_seconds_ago = 300u32;

    // Get TWAP tick from oracle
    let twap_tick = oracle.get_twap_tick(now, twap_seconds_ago)?;
    let twap_sqrt_price = sqrt_price_from_tick(twap_tick)?;

    // Use TWAP values instead of spot values for liquidity placement
    let current_tick = twap_tick;
    let current_sqrt_price = twap_sqrt_price;

    // Derive POMM range width from market's immutable tick spacing
    // This prevents manipulation via mutable buffer parameters
    // Formula: POMM width = market tick spacing * 20 (approximately ±2% for common tick spacings)
    // Examples:
    // - tick_spacing = 1: POMM width = 20 ticks ≈ 0.2%
    // - tick_spacing = 10: POMM width = 200 ticks ≈ 2%
    // - tick_spacing = 60: POMM width = 1200 ticks ≈ 12%
    let pomm_tick_width = (market.tick_spacing as i32)
        .saturating_mul(20)
        .clamp(10, 2000); // Width between 10-2000 ticks (0.1%-20%)

    // Log the derived width for transparency
    #[cfg(feature = "telemetry")]
    msg!(
        "POMM using derived tick width {} from market tick spacing {}",
        pomm_tick_width,
        market.tick_spacing
    );

    // Determine range based on which tokens we have:
    // - If only token_0: place below current price (will be bought as price rises)
    // - If only token_1: place above current price (will be bought as price falls)
    // - If both: place symmetric range around current price
    let (tick_lower, tick_upper, amount_0_used, amount_1_used) = if amount_0 > 0 && amount_1 == 0 {
        // Only token_0: place one-sided liquidity below current price
        let tick_upper = current_tick;
        let tick_lower = current_tick - pomm_tick_width;
        (tick_lower, tick_upper, amount_0, 0u64)
    } else if amount_0 == 0 && amount_1 > 0 {
        // Only token_1: place one-sided liquidity above current price
        let tick_lower = current_tick;
        let tick_upper = current_tick + pomm_tick_width;
        (tick_lower, tick_upper, 0u64, amount_1)
    } else {
        // Both tokens: place in symmetric range around current price
        let tick_lower = current_tick - pomm_tick_width;
        let tick_upper = current_tick + pomm_tick_width;
        (tick_lower, tick_upper, amount_0, amount_1)
    };

    // Calculate liquidity to add using actual amounts we'll deploy
    let sqrt_pl = sqrt_price_from_tick(tick_lower)?;
    let sqrt_pu = sqrt_price_from_tick(tick_upper)?;
    let liq = liquidity_from_amounts(
        current_sqrt_price,
        sqrt_pl,
        sqrt_pu,
        amount_0_used,
        amount_1_used,
    )?;

    // Check against minimum liquidity threshold to prevent dust positions
    if liq < MIN_LIQUIDITY {
        #[cfg(feature = "telemetry")]
        msg!(
            "POMM liquidity {} below minimum threshold {}, skipping",
            liq,
            MIN_LIQUIDITY
        );
        return Ok(());
    }

    // Update floor position and active liquidity
    // Note: These fields are overloaded - they track POMM liquidity placement
    // but also serve as global bounds. This will be refactored when POMM uses real positions.
    market.global_lower_tick = tick_lower;
    market.global_upper_tick = tick_upper;
    market.floor_liquidity = market
        .floor_liquidity
        .checked_add(liq)
        .ok_or(FeelsError::MathOverflow)?;

    // Add to active liquidity if current price is within the new range
    if current_tick >= tick_lower && current_tick <= tick_upper {
        market.liquidity = market
            .liquidity
            .checked_add(liq)
            .ok_or(FeelsError::MathOverflow)?;
    }

    // Adjust Buffer accounting - deduct only what we actually used
    buffer.fees_token_0 = buffer.fees_token_0.saturating_sub(amount_0_used as u128);
    buffer.fees_token_1 = buffer.fees_token_1.saturating_sub(amount_1_used as u128);
    buffer.tau_spot = buffer
        .tau_spot
        .saturating_sub((amount_0_used.saturating_add(amount_1_used)) as u128);
    buffer.last_floor_placement = now;
    buffer.total_distributed = buffer
        .total_distributed
        .saturating_add((amount_0_used.saturating_add(amount_1_used)) as u128);

    // Emit a lightweight event via log to avoid extra struct overhead (optional)
    // msg!("POMM added liquidity: liq={} a0={} a1={} range=[{},{}]", liq, amount_0_used, amount_1_used, tick_lower, tick_upper);

    Ok(())
}
