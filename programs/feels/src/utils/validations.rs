//! Common validation utilities
//!
//! Shared validation logic used across instructions

use crate::{
    error::FeelsError,
    state::{Market, TickArray, TICK_ARRAY_SIZE},
};
use anchor_lang::prelude::*;

/// Validate that an amount is non-zero
pub fn validate_amount(amount: u64) -> Result<()> {
    require!(amount > 0, FeelsError::ZeroAmount);
    Ok(())
}

/// Validate that amounts for liquidity operations are non-zero
pub fn validate_liquidity_amounts(amount_0: u64, amount_1: u64) -> Result<()> {
    require!(amount_0 > 0 || amount_1 > 0, FeelsError::ZeroAmount);
    Ok(())
}

/// Validate slippage constraints
pub fn validate_slippage(actual: u64, minimum: u64) -> Result<()> {
    require!(actual >= minimum, FeelsError::SlippageExceeded);
    Ok(())
}

/// Validate that a market is operational
pub fn validate_market_active(market: &Market) -> Result<()> {
    require!(market.is_initialized, FeelsError::MarketNotInitialized);
    require!(!market.is_paused, FeelsError::MarketPaused);
    Ok(())
}

/// Validate fee bounds
pub fn validate_fee(fee_bps: u16, max_fee_bps: u16) -> Result<()> {
    require!(
        fee_bps > 0 && fee_bps <= max_fee_bps,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validate tick spacing
pub fn validate_tick_spacing(tick_spacing: u16, max_tick_spacing: u16) -> Result<()> {
    require!(
        tick_spacing > 0 && tick_spacing <= max_tick_spacing,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validate that ticks are properly ordered and aligned
pub fn validate_tick_range(tick_lower: i32, tick_upper: i32, tick_spacing: u16) -> Result<()> {
    require!(tick_lower < tick_upper, FeelsError::InvalidTickRange);
    require!(
        tick_lower % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    require!(
        tick_upper % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    require!(
        tick_lower >= crate::constants::MIN_TICK,
        FeelsError::InvalidTick
    );
    require!(
        tick_upper <= crate::constants::MAX_TICK,
        FeelsError::InvalidTick
    );
    Ok(())
}

/// Calculate expected tick array start index for a given tick
pub fn get_tick_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
    let ticks_per_array = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    let array_index = tick_index.div_euclid(ticks_per_array);
    array_index * ticks_per_array
}

/// Validate that a tick array matches the expected tick
pub fn validate_tick_array_for_tick(
    tick_array: &TickArray,
    tick_index: i32,
    tick_spacing: u16,
) -> Result<()> {
    let expected_start = get_tick_array_start_index(tick_index, tick_spacing);
    require!(
        tick_array.start_tick_index == expected_start,
        FeelsError::InvalidTickArray
    );
    Ok(())
}

/// Validate distribution for token minting
pub fn validate_distribution(
    distribution_total: u64,
    total_supply: u64,
    min_reserve: u64,
) -> Result<()> {
    require!(
        distribution_total <= total_supply - min_reserve,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validates that a pool includes FeelsSOL as one side
pub fn validate_pool_includes_feelssol(
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
    feelssol_mint: &Pubkey,
) -> Result<()> {
    require!(
        token_0_mint == feelssol_mint || token_1_mint == feelssol_mint,
        FeelsError::InvalidRoute
    );
    Ok(())
}
