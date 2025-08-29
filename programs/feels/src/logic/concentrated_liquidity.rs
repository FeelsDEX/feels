/// Core mathematical operations for concentrated liquidity positions and swaps.
/// Calculates token amounts from liquidity, liquidity from token amounts, and
/// rate movements during swaps. Implements Uniswap V3 math with Q64.96 precision
/// for accurate pricing across the full range of possible token ratios.
use anchor_lang::prelude::*;
use crate::state::FeelsProtocolError;
use crate::utils::{
    get_amount_0_delta, get_amount_1_delta, get_liquidity_for_amount_0, get_liquidity_for_amount_1,
    get_next_sqrt_rate_from_amount_0_rounding_up, get_next_sqrt_rate_from_amount_1_rounding_down,
    MAX_SQRT_RATE_X96, MIN_SQRT_RATE_X96,
};

// ============================================================================
// Concentrated Liquidity Math Implementation
// ============================================================================

/// Concentrated liquidity math operations for AMM positions
pub struct ConcentratedLiquidityMath;

impl ConcentratedLiquidityMath {
    /// Calculate token amounts for a given concentrated liquidity amount
    /// This is the core calculation for concentrated liquidity position management
    pub fn get_amounts_for_concentrated_liquidity(
        sqrt_price_current: u128,
        sqrt_price_lower: u128,
        sqrt_price_upper: u128,
        concentrated_liquidity: u128,
    ) -> Result<(u64, u64)> {
        require!(
            sqrt_price_lower < sqrt_price_upper,
            FeelsProtocolError::InvalidPriceRange
        );

        let amount_a;
        let amount_b;

        if sqrt_price_current <= sqrt_price_lower {
            // All token A
            amount_a = get_amount_0_delta(
                sqrt_price_lower,
                sqrt_price_upper,
                concentrated_liquidity,
                true,
            )?;
            amount_b = 0;
        } else if sqrt_price_current < sqrt_price_upper {
            // Both tokens
            amount_a = get_amount_0_delta(
                sqrt_price_current,
                sqrt_price_upper,
                concentrated_liquidity,
                true,
            )?;
            amount_b = get_amount_1_delta(
                sqrt_price_lower,
                sqrt_price_current,
                concentrated_liquidity,
                true,
            )?
        } else {
            // All token B
            amount_a = 0;
            amount_b = get_amount_1_delta(
                sqrt_price_lower,
                sqrt_price_upper,
                concentrated_liquidity,
                true,
            )?;
        }

        let amount_a_u64 = amount_a
            .try_into()
            .map_err(|_| FeelsProtocolError::ArithmeticOverflow)?;
        let amount_b_u64 = amount_b
            .try_into()
            .map_err(|_| FeelsProtocolError::ArithmeticOverflow)?;

        Ok((amount_a_u64, amount_b_u64))
    }

    /// Calculate concentrated liquidity for given token amounts
    /// Used when adding concentrated liquidity to determine how much to mint
    pub fn get_concentrated_liquidity_for_amounts(
        sqrt_price_current: u128,
        sqrt_price_lower: u128,
        sqrt_price_upper: u128,
        amount_0: u64,
        amount_1: u64,
    ) -> Result<u128> {
        require!(
            sqrt_price_lower < sqrt_price_upper,
            FeelsProtocolError::InvalidPriceRange
        );

        if sqrt_price_current <= sqrt_price_lower {
            get_liquidity_for_amount_0(sqrt_price_lower, sqrt_price_upper, amount_0 as u128)
        } else if sqrt_price_current < sqrt_price_upper {
            let liquidity_0 =
                get_liquidity_for_amount_0(sqrt_price_current, sqrt_price_upper, amount_0 as u128)?;
            let liquidity_1 =
                get_liquidity_for_amount_1(sqrt_price_lower, sqrt_price_current, amount_1 as u128)?;
            Ok(liquidity_0.min(liquidity_1))
        } else {
            get_liquidity_for_amount_1(sqrt_price_lower, sqrt_price_upper, amount_1 as u128)
        }
    }

    /// Calculate the next sqrt rate after a swap
    /// This is used during swap execution to update pool state
    pub fn get_next_sqrt_rate_from_input(
        sqrt_price: u128,
        liquidity: u128,
        amount_in: u64,
        zero_for_one: bool,
    ) -> Result<u128> {
        // Validate upper bounds for sqrt_rate
        require!(
            (MIN_SQRT_RATE_X96..=MAX_SQRT_RATE_X96).contains(&sqrt_price),
            FeelsProtocolError::RateOutOfBounds
        );
        require!(liquidity > 0, FeelsProtocolError::InsufficientLiquidity);
        require!(amount_in > 0, FeelsProtocolError::InvalidAmount);

        if zero_for_one {
            get_next_sqrt_rate_from_amount_0_rounding_up(
                sqrt_price,
                liquidity,
                amount_in as u128,
                true,
            )
        } else {
            get_next_sqrt_rate_from_amount_1_rounding_down(
                sqrt_price,
                liquidity,
                amount_in as u128,
                true,
            )
        }
    }

    /// Calculate the next sqrt rate from output amount
    pub fn get_next_sqrt_rate_from_output(
        sqrt_price: u128,
        liquidity: u128,
        amount_out: u64,
        zero_for_one: bool,
    ) -> Result<u128> {
        // Validate upper bounds for sqrt_rate (same as input version)
        require!(
            (MIN_SQRT_RATE_X96..=MAX_SQRT_RATE_X96).contains(&sqrt_price),
            FeelsProtocolError::RateOutOfBounds
        );
        require!(liquidity > 0, FeelsProtocolError::InsufficientLiquidity);
        require!(amount_out > 0, FeelsProtocolError::InvalidAmount);

        if zero_for_one {
            get_next_sqrt_rate_from_amount_1_rounding_down(
                sqrt_price,
                liquidity,
                amount_out as u128,
                false,
            )
        } else {
            get_next_sqrt_rate_from_amount_0_rounding_up(
                sqrt_price,
                liquidity,
                amount_out as u128,
                false,
            )
        }
    }

    /// Core swap logic: calculate the output amount for a given input amount
    pub fn get_amount_out(
        sqrt_price_current: u128,
        sqrt_price_target: u128,
        liquidity: u128,
        zero_for_one: bool,
    ) -> Result<u64> {
        require!(
            (zero_for_one && sqrt_price_target < sqrt_price_current)
                || (!zero_for_one && sqrt_price_target > sqrt_price_current),
            FeelsProtocolError::InvalidPriceRange
        );

        if zero_for_one {
            get_amount_1_delta(sqrt_price_target, sqrt_price_current, liquidity, false).and_then(
                |amount| {
                    amount
                        .try_into()
                        .map_err(|_| FeelsProtocolError::ArithmeticOverflow.into())
                },
            )
        } else {
            get_amount_0_delta(sqrt_price_current, sqrt_price_target, liquidity, false).and_then(
                |amount| {
                    amount
                        .try_into()
                        .map_err(|_| FeelsProtocolError::ArithmeticOverflow.into())
                },
            )
        }
    }
}

/// Concentrated Liquidity Manager for liquidity calculations
pub struct ConcentratedLiquidityManager;

impl ConcentratedLiquidityManager {
    /// Calculate liquidity from token amounts
    pub fn calculate_liquidity_from_amounts(
        sqrt_rate_current: u128,
        sqrt_rate_lower: u128,
        sqrt_rate_upper: u128,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<u128> {
        ConcentratedLiquidityMath::get_concentrated_liquidity_for_amounts(
            sqrt_rate_current,
            sqrt_rate_lower,
            sqrt_rate_upper,
            amount_a,
            amount_b,
        )
    }
}
