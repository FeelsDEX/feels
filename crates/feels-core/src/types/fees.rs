//! # Fee Types
//! 
//! Types for thermodynamic fee calculations.

use crate::constants::{BASIS_POINTS_DENOMINATOR, Q64};

/// Parameters for instantaneous fee calculation
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct InstantaneousFeeParams {
    /// Amount being swapped
    pub amount_in: u64,
    /// Work performed (from physics calculation)
    pub work: u128,
    /// Base fee rate in basis points
    pub base_fee_rate: u16,
    /// Price improvement clamp factor κ (basis points)
    pub kappa: Option<u32>,
    /// Input token price Π_in (Q64 format)
    pub pi_in: Option<u128>,
    /// Output token price Π_out (Q64 format)
    pub pi_out: Option<u128>,
    /// Available buffer for rebates
    pub available_buffer: Option<u64>,
}

/// Result of instantaneous fee calculation
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone)]
pub struct InstantaneousFeeResult {
    /// Total fee to be charged
    pub fee_amount: u64,
    /// Rebate to be paid (if any)
    pub rebate_amount: u64,
    /// Base fee before adjustments
    pub base_fee: u64,
    /// Price improvement (if calculated)
    pub price_improvement: Option<u64>,
}

/// Calculate thermodynamic fee (pure function)
pub fn calculate_thermodynamic_fee(
    work: u128,
    pi_in: u128,
    pi_out: u128,
    is_uphill: bool,
) -> u64 {
    if is_uphill {
        // Fee = W / Π_in
        let fee = work / pi_in.max(1);
        fee.min(u64::MAX as u128) as u64
    } else {
        // Rebate = |W| / Π_out
        let rebate = work / pi_out.max(1);
        rebate.min(u64::MAX as u128) as u64
    }
}

/// Convert sqrt price to token price (Π = P²)
pub fn sqrt_price_to_token_price(sqrt_price: u128) -> u128 {
    // Π = (sqrt_price)² / Q64
    let price = sqrt_price.saturating_mul(sqrt_price);
    price / Q64
}

/// Get input token price from sqrt prices and direction
pub fn get_pi_in(sqrt_price: u128, zero_for_one: bool) -> u128 {
    if zero_for_one {
        // Token0 -> Token1: Π_in = 1 (token0 is numeraire)
        Q64
    } else {
        // Token1 -> Token0: Π_in = P² (token1 price in token0)
        sqrt_price_to_token_price(sqrt_price)
    }
}

/// Get output token price from sqrt prices and direction
pub fn get_pi_out(sqrt_price: u128, zero_for_one: bool) -> u128 {
    if zero_for_one {
        // Token0 -> Token1: Π_out = P² (token1 price in token0)
        sqrt_price_to_token_price(sqrt_price)
    } else {
        // Token1 -> Token0: Π_out = 1 (token0 is numeraire)
        Q64
    }
}

/// Apply κ clamp to rebate amount
pub fn apply_kappa_clamp(rebate: u64, amount: u64, kappa_bps: u32) -> u64 {
    let max_rebate = (amount as u128 * kappa_bps as u128) / BASIS_POINTS_DENOMINATOR as u128;
    rebate.min(max_rebate as u64)
}