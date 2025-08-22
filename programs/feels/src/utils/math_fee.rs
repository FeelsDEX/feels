/// Implements Q128.128 fixed-point arithmetic for cumulative fee growth tracking.
/// Fee growth represents accumulated fees per unit of liquidity, requiring high
/// precision to fairly distribute fees among LPs. Delegates to U256 operations
/// to handle values that exceed u128 range after years of fee accumulation.

use anchor_lang::prelude::*;
use crate::state::PoolError;

// ============================================================================
// Type Definitions
// ============================================================================

/// 
/// Fee growth is stored as a Q128.128 fixed-point number representing
/// fees per unit of liquidity. This allows precise tracking of fee
/// accrual without rounding errors.
/// 
/// This module delegates to the production-grade U256 arithmetic from math_u256.rs
pub struct FeeGrowthMath;

// ============================================================================
// Implementation
// ============================================================================

impl FeeGrowthMath {
    /// Add fee growth values (both Q128.128)
    /// Delegates to production-grade U256 arithmetic
    pub fn add_fee_growth(
        fee_growth_a: [u64; 4],
        fee_growth_b: [u64; 4],
    ) -> Result<[u64; 4]> {
        super::math_u256::add_u256(fee_growth_a, fee_growth_b)
    }
    
    /// Subtract fee growth values (both Q128.128)
    /// Delegates to production-grade U256 arithmetic
    pub fn sub_fee_growth(
        fee_growth_a: [u64; 4],
        fee_growth_b: [u64; 4],
    ) -> Result<[u64; 4]> {
        // Now that sub_u256 returns Result, we get proper error handling
        super::math_u256::sub_u256(fee_growth_a, fee_growth_b)
    }
    
    /// Convert fee amount to fee growth delta
    /// Delegates to production-grade U256 arithmetic
    pub fn fee_to_fee_growth(fee_amount: u64, liquidity: u128) -> Result<[u64; 4]> {
        super::math_u256::calculate_fee_growth_delta(fee_amount, liquidity)
    }
}