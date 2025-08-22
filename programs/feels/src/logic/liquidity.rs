/// Core mathematical operations for concentrated liquidity positions and swaps.
/// Calculates token amounts from liquidity, liquidity from token amounts, and
/// price movements during swaps. Implements Uniswap V3 math with Q64.96 precision
/// for accurate pricing across the full range of possible token ratios.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use crate::utils::math_liquidity::{
    get_amount_0_delta, get_amount_1_delta, get_liquidity_for_amount_0, get_liquidity_for_amount_1,
    get_next_sqrt_price_from_amount_0_rounding_up, get_next_sqrt_price_from_amount_1_rounding_down
};
use crate::utils::{MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64};

// ============================================================================
// Liquidity Math Implementation
// ============================================================================

/// Liquidity math operations for concentrated liquidity AMM
pub struct LiquidityMath;

impl LiquidityMath {
    /// Calculate token amounts for a given liquidity amount
    /// This is the core calculation for position management
    pub fn get_amounts_for_liquidity(
        sqrt_price_current: u128,
        sqrt_price_lower: u128,
        sqrt_price_upper: u128,
        liquidity: u128,
    ) -> Result<(u64, u64)> {
        require!(
            sqrt_price_lower < sqrt_price_upper,
            PoolError::InvalidPriceRange
        );

        let amount_a;
        let amount_b;

        if sqrt_price_current <= sqrt_price_lower {
            // All token A
            amount_a = get_amount_0_delta(sqrt_price_lower, sqrt_price_upper, liquidity, true)?;
            amount_b = 0;
        } else if sqrt_price_current < sqrt_price_upper {
            // Both tokens
            amount_a = get_amount_0_delta(sqrt_price_current, sqrt_price_upper, liquidity, true)?;
            amount_b = get_amount_1_delta(sqrt_price_lower, sqrt_price_current, liquidity, true)?;
        } else {
            // All token B
            amount_a = 0;
            amount_b = get_amount_1_delta(sqrt_price_lower, sqrt_price_upper, liquidity, true)?;
        }

        Ok((amount_a, amount_b))
    }

    /// Calculate liquidity for given token amounts
    /// Used when adding liquidity to determine how much liquidity to mint
    pub fn get_liquidity_for_amounts(
        sqrt_price_current: u128,
        sqrt_price_lower: u128,
        sqrt_price_upper: u128,
        amount_0: u64,
        amount_1: u64,
    ) -> Result<u128> {
        require!(
            sqrt_price_lower < sqrt_price_upper,
            PoolError::InvalidPriceRange
        );

        if sqrt_price_current <= sqrt_price_lower {
            get_liquidity_for_amount_0(sqrt_price_lower, sqrt_price_upper, amount_0)
        } else if sqrt_price_current < sqrt_price_upper {
            let liquidity_0 = get_liquidity_for_amount_0(sqrt_price_current, sqrt_price_upper, amount_0)?;
            let liquidity_1 = get_liquidity_for_amount_1(sqrt_price_lower, sqrt_price_current, amount_1)?;
            Ok(liquidity_0.min(liquidity_1))
        } else {
            get_liquidity_for_amount_1(sqrt_price_lower, sqrt_price_upper, amount_1)
        }
    }

    /// Calculate the next sqrt price after a swap
    /// This is used during swap execution to update pool state
    pub fn get_next_sqrt_price_from_input(
        sqrt_price: u128,
        liquidity: u128,
        amount_in: u64,
        zero_for_one: bool,
    ) -> Result<u128> {
        // V4.1 Fix: Validate upper bounds for sqrt_price
        require!(
            sqrt_price >= MIN_SQRT_PRICE_X64 && sqrt_price <= MAX_SQRT_PRICE_X64,
            PoolError::PriceOutOfBounds
        );
        require!(liquidity > 0, PoolError::InsufficientLiquidity);
        require!(amount_in > 0, PoolError::InvalidAmount);

        if zero_for_one {
            get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount_in, true)
        } else {
            get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount_in, true)
        }
    }

    /// Calculate the next sqrt price from output amount
    pub fn get_next_sqrt_price_from_output(
        sqrt_price: u128,
        liquidity: u128,
        amount_out: u64,
        zero_for_one: bool,
    ) -> Result<u128> {
        // V4.1 Fix: Validate upper bounds for sqrt_price (same as input version)
        require!(
            sqrt_price >= MIN_SQRT_PRICE_X64 && sqrt_price <= MAX_SQRT_PRICE_X64,
            PoolError::PriceOutOfBounds
        );
        require!(liquidity > 0, PoolError::InsufficientLiquidity);
        require!(amount_out > 0, PoolError::InvalidAmount);

        if zero_for_one {
            get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount_out, false)
        } else {
            get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount_out, false)
        }
    }

    /// Calculate the output amount for a given input amount
    /// Core swap calculation logic
    pub fn get_amount_out(
        sqrt_price_current: u128,
        sqrt_price_target: u128,
        liquidity: u128,
        zero_for_one: bool,
    ) -> Result<u64> {
        require!(
            (zero_for_one && sqrt_price_target < sqrt_price_current) ||
            (!zero_for_one && sqrt_price_target > sqrt_price_current),
            PoolError::InvalidPriceRange
        );

        if zero_for_one {
            get_amount_1_delta(sqrt_price_target, sqrt_price_current, liquidity, false)
        } else {
            get_amount_0_delta(sqrt_price_current, sqrt_price_target, liquidity, false)
        }
    }
}
