//! # Mathematics Module - Thermodynamic AMM Calculations
//!
//! This module re-exports mathematical operations from the `feels-core` crate,
//! which returns anchor-compatible Result types when compiled with the "anchor" feature.
//!
//! ## **Re-exported from feels-core**
//! 
//! ### Safe Arithmetic Operations
//! - **Basic Operations**: add/sub/mul/div for u64, u128, i128 with overflow protection
//! - **Advanced Math**: mul_div operations using U256 intermediates, shift operations
//! - **Liquidity Delta**: add/sub_liquidity_delta for signed liquidity changes
//! - **Utilities**: sqrt functions, percentage calculations, safe price conversions
//!
//! ### AMM Mathematics  
//! - **Tick Math**: get_sqrt_price_at_tick, get_tick_at_sqrt_price conversions
//! - **Liquidity Math**: get_amount_0/1_delta, get_liquidity_for_amount_0/1
//! - **Big Integer**: U256 type and mul_div operations with Rounding modes
//!
//! ### Fee Mathematics
//! - **Fee Growth**: calculate_fee_growth_q64 with Q64 precision
//! - **Fee Tracking**: fee_growth_words conversions and wrapping subtraction
//!
//! All mathematical operations maintain bit-identical results between on-chain
//! and off-chain components through the shared feels-core implementation.

// Re-export commonly used types
pub use num_traits::{CheckedAdd, CheckedSub, CheckedMul, CheckedDiv, Zero, One};

// Re-export all math functions directly from feels-core
// The "anchor" feature ensures CoreResult<T> is compatible with anchor_lang::Result<T>
pub use feels_core::math::*;

// Create a safe module that re-exports the safe_* functions for the naming pattern used in on-chain code
pub mod safe {
    pub use super::{
        safe_add_u64 as add_u64,
        safe_sub_u64 as sub_u64, 
        safe_mul_u64 as mul_u64,
        safe_div_u64 as div_u64,
        safe_add_u128 as add_u128,
        safe_sub_u128 as sub_u128,
        safe_mul_u128 as mul_u128, 
        safe_div_u128 as div_u128,
        safe_add_i128 as add_i128,
        safe_sub_i128 as sub_i128,
        safe_mul_i128 as mul_i128,
        safe_div_i128 as div_i128,
        safe_shl_u128 as shl_u128,
        safe_shr_u128 as shr_u128,
        safe_mul_div_u64 as mul_div_u64,
        safe_mul_div_u128 as mul_div_u128,
        safe_add_liquidity_delta as add_liquidity_delta,
        safe_sub_liquidity_delta as sub_liquidity_delta,
        safe_calculate_percentage as calculate_percentage,
        safe_sqrt_price_to_price as sqrt_price_to_price_safe,
        sqrt_u64,
        sqrt_u128,
    };
}

// ============================================================================
// Fee Types and Calculations
// ============================================================================

/// Breakdown of fee calculation components
#[derive(Clone, Debug)]
pub struct FeeBreakdown {
    pub base_fee: u64,
    pub work_surcharge: u64,
    pub total_fee: u64,
    pub rebate_amount: u64,
}

impl FeeBreakdown {
    pub fn new(base_fee: u64, work_surcharge: u64, rebate_amount: u64) -> Self {
        let total_before_rebate = base_fee.saturating_add(work_surcharge);
        Self {
            base_fee,
            work_surcharge,
            total_fee: total_before_rebate.saturating_sub(rebate_amount),
            rebate_amount,
        }
    }
}

/// Fee configuration structure
#[derive(Clone, Debug)]
pub struct FeeConfig {
    pub base_fee_rate: u16, // in basis points
    pub max_surcharge_bps: u16, // in basis points
    pub max_instantaneous_fee: u16, // in basis points
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            base_fee_rate: 30, // 0.3%
            max_surcharge_bps: 100, // 1% max dynamic surcharge
            max_instantaneous_fee: 250, // 2.5% total cap
        }
    }
}

// ============================================================================
// Fee Growth Calculations (Q64 Native)
// ============================================================================

pub mod fee_math {
    use crate::error::FeelsProtocolError;
    use anchor_lang::prelude::*;

    /// Calculate fee growth using native Q64 precision and library operations
    pub fn calculate_fee_growth_q64(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
        feels_core::math::calculate_fee_growth_q64(fee_amount, liquidity)
            .map_err(|_| FeelsProtocolError::InvalidLiquidity.into())
    }
}

// ============================================================================
// Fee Growth Math - Re-exported from feels-core
// ============================================================================

/// Fee growth math utilities for tick operations
pub struct FeeGrowthMath;

impl FeeGrowthMath {
    /// Convert [u64; 4] fee growth to u128 (using lower 128 bits)
    pub fn words_to_u128(words: [u64; 4]) -> u128 {
        feels_core::math::fee_growth_words_to_u128(words)
    }
    
    /// Convert u128 to [u64; 4] fee growth
    pub fn u128_to_words(value: u128) -> [u64; 4] {
        feels_core::math::fee_growth_u128_to_words(value)
    }
    
    /// Subtract fee growth values with overflow handling (u128 version)
    pub fn sub_fee_growth(a: u128, b: u128) -> Result<u128, FeelsProtocolError> {
        Ok(feels_core::math::sub_fee_growth(a, b))
    }
    
    /// Subtract fee growth values with overflow handling ([u64; 4] version)
    pub fn sub_fee_growth_words(a: [u64; 4], b: [u64; 4]) -> Result<[u64; 4], FeelsProtocolError> {
        Ok(feels_core::math::sub_fee_growth_words(a, b))
    }
}