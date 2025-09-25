//! Parameter validation utilities
//!
//! Provides comprehensive validation for instruction parameters to prevent
//! malicious or invalid inputs that could compromise market integrity.

use crate::{constants::*, error::FeelsError};
use anchor_lang::prelude::*;

/// Validate base fee in basis points
pub fn validate_base_fee_bps(fee_bps: u16) -> Result<()> {
    // Minimum fee to prevent zero-fee exploitation
    const MIN_BASE_FEE_BPS: u16 = 1; // 0.01%

    require!(
        fee_bps >= MIN_BASE_FEE_BPS && fee_bps <= MAX_FEE_BPS,
        FeelsError::InvalidPrice
    );

    // Warn if fee is unusually high
    if fee_bps > 100 {
        // 1%
        msg!("Warning: High base fee of {}bps", fee_bps);
    }

    Ok(())
}

/// Validate tick spacing parameters
pub fn validate_tick_spacing_param(tick_spacing: u16) -> Result<()> {
    // Valid tick spacings are powers of 2 for efficiency
    const VALID_TICK_SPACINGS: [u16; 8] = [1, 2, 4, 6, 8, 10, 16, 32];

    require!(
        VALID_TICK_SPACINGS.contains(&tick_spacing),
        FeelsError::InvalidTickSpacing
    );

    Ok(())
}

/// Validate initial sqrt price
pub fn validate_initial_sqrt_price(sqrt_price: u128) -> Result<()> {
    // Minimum sqrt price (prevents extreme prices)
    const MIN_SQRT_PRICE: u128 = 4295048016; // ~1e-9 price
                                             // Maximum sqrt price (prevents overflow)
    const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055; // ~1e9 price

    require!(
        sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
        FeelsError::InvalidPrice
    );

    Ok(())
}

/// Validate tick range parameters
pub fn validate_tick_range_params(
    tick_lower: i32,
    tick_upper: i32,
    tick_spacing: u16,
) -> Result<()> {
    // Validate bounds
    require!(
        tick_lower >= MIN_TICK && tick_lower <= MAX_TICK,
        FeelsError::InvalidTickRange
    );
    require!(
        tick_upper >= MIN_TICK && tick_upper <= MAX_TICK,
        FeelsError::InvalidTickRange
    );

    // Validate ordering
    require!(tick_lower < tick_upper, FeelsError::InvalidTickRange);

    // Validate alignment to tick spacing
    require!(
        tick_lower % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    require!(
        tick_upper % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );

    // Validate minimum range width (prevent sandwich attacks)
    let min_ticks = (tick_spacing as i32) * 10; // At least 10 tick spacings
    require!(
        tick_upper - tick_lower >= min_ticks,
        FeelsError::InvalidTickRange
    );

    Ok(())
}

/// Validate liquidity amount
pub fn validate_liquidity_amount(liquidity: u128) -> Result<()> {
    require!(
        liquidity >= MIN_LIQUIDITY,
        FeelsError::LiquidityBelowMinimum
    );

    // Check for overflow risk
    // Check for reasonable upper bound
    // MAX_LIQUIDITY could be defined in constants if needed
    require!(
        liquidity <= u128::MAX / 2, // Leave room for calculations
        FeelsError::MathOverflow
    );

    Ok(())
}

/// Validate swap amount
pub fn validate_swap_amount(amount: u64, is_exact_out: bool) -> Result<()> {
    require!(amount > 0, FeelsError::ZeroAmount);

    // For exact out swaps, limit to prevent excessive slippage
    if is_exact_out {
        const MAX_EXACT_OUT: u64 = 1_000_000_000_000; // 1M tokens with 6 decimals
        require!(amount <= MAX_EXACT_OUT, FeelsError::AmountOverflow);
    }

    Ok(())
}

/// Validate slippage tolerance
pub fn validate_slippage_tolerance(amount_min: u64, amount_expected: u64) -> Result<()> {
    // Maximum allowed slippage: 50%
    const MAX_SLIPPAGE_BPS: u64 = 5000;

    if amount_expected > 0 {
        let slippage_bps = ((amount_expected - amount_min) * 10_000) / amount_expected;

        require!(
            slippage_bps <= MAX_SLIPPAGE_BPS,
            FeelsError::SlippageExceeded
        );

        // Warn on high slippage
        if slippage_bps > 1000 {
            // 10%
            msg!("Warning: High slippage tolerance of {}bps", slippage_bps);
        }
    }

    Ok(())
}

/// Validate POMM parameters
pub fn validate_pomm_tick_width(tick_width: i32, tick_spacing: u16) -> Result<()> {
    require!(tick_width > 0, FeelsError::InvalidTickRange);

    // Must be aligned to tick spacing
    require!(
        tick_width % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );

    // Validate within POMM bounds
    require!(
        tick_width >= POMM_MIN_WIDTH && tick_width <= POMM_MAX_WIDTH,
        FeelsError::InvalidTickRange
    );

    Ok(())
}

/// Validate floor tick parameters
pub fn validate_floor_tick(floor_tick: i32, current_tick: i32, buffer_ticks: u16) -> Result<()> {
    // Floor tick must be below current price with buffer
    require!(
        floor_tick < current_tick - buffer_ticks as i32,
        FeelsError::InvalidPrice
    );

    // Floor tick must be within valid range
    require!(
        floor_tick >= MIN_TICK && floor_tick <= MAX_TICK,
        FeelsError::InvalidTickRange
    );

    Ok(())
}

/// Validate protocol fee distribution
pub fn validate_fee_distribution(
    buffer_tau_bps: u16,
    treasury_bps: u16,
    creator_bps: u16,
) -> Result<()> {
    // Total must equal 100%
    let total_bps = buffer_tau_bps as u32 + treasury_bps as u32 + creator_bps as u32;
    require!(total_bps == 10_000, FeelsError::InvalidPrice);

    // Each component must be reasonable
    require!(
        buffer_tau_bps >= 2000 && buffer_tau_bps <= 8000, // 20-80%
        FeelsError::InvalidPrice
    );

    require!(
        treasury_bps <= 3000, // Max 30%
        FeelsError::InvalidPrice
    );

    require!(
        creator_bps <= 1000, // Max 10%
        FeelsError::InvalidPrice
    );

    Ok(())
}
