/// SDK rebate calculator with κ clamp and price improvement logic.
/// Provides client-side calculation of rebates based on work and price improvement.

use anchor_lang::prelude::*;
// Import constants from feels-core
use feels_core::constants::{Q64, BPS_DENOMINATOR};

// ============================================================================
// Rebate Calculation
// ============================================================================

/// Price improvement data for rebate calculation
#[derive(Clone, Debug, Default)]
pub struct PriceImprovement {
    /// Oracle/reference price (Q64)
    pub oracle_price: u128,
    /// Execution price (Q64)
    pub execution_price: u128,
    /// Price improvement amount (basis points)
    pub improvement_bps: u64,
    /// Is this a buy order (affects improvement calculation)
    pub is_buy: bool,
}

/// Buffer parameters for rebate calculation
#[derive(Clone, Debug)]
pub struct BufferParams {
    /// Rebate participation rate η [0, 10000]
    pub rebate_eta: u32,
    /// Price improvement clamp factor κ [0, 10000]
    pub kappa: u32,
    /// Available tau for rebates
    pub available_tau: u64,
    /// Per-transaction rebate cap
    pub rebate_cap_tx: u64,
    /// Remaining epoch rebate capacity
    pub rebate_cap_epoch_remaining: u64,
}

/// Rebate calculation result
#[derive(Clone, Debug)]
pub struct RebateResult {
    /// Base rebate from negative work
    pub base_rebate: u64,
    /// Price improvement bonus
    pub improvement_bonus: u64,
    /// Total rebate (capped)
    pub total_rebate: u64,
    /// Effective rebate rate (basis points)
    pub effective_rebate_bps: u64,
    /// Whether rebate was capped
    pub was_capped: bool,
    /// Cap reason if capped
    pub cap_reason: Option<String>,
}

/// Calculate instantaneous rebate with price improvement
pub fn calculate_rebate_with_improvement(
    work: i128,
    amount_in: u64,
    price_improvement: &PriceImprovement,
    buffer_params: &BufferParams,
) -> RebateResult {
    // Only calculate rebate for negative work
    if work >= 0 {
        return RebateResult {
            base_rebate: 0,
            improvement_bonus: 0,
            total_rebate: 0,
            effective_rebate_bps: 0,
            was_capped: false,
            cap_reason: None,
        };
    }
    
    let negative_work = (-work) as u128;
    
    // Calculate base rebate: R_base = |W| * η
    let base_rebate = (negative_work * buffer_params.rebate_eta as u128) / BPS_DENOMINATOR as u128;
    
    // Calculate price improvement bonus: κ * price_improvement
    let improvement_bonus = (buffer_params.kappa as u128 * price_improvement.improvement_bps as u128 * amount_in as u128) 
        / (BPS_DENOMINATOR as u128 * BPS_DENOMINATOR as u128);
    
    // Total uncapped rebate
    let total_uncapped = base_rebate.saturating_add(improvement_bonus);
    
    // Apply caps
    let (total_rebate, was_capped, cap_reason) = apply_rebate_caps(
        total_uncapped,
        buffer_params,
        amount_in,
    );
    
    // Calculate effective rebate rate
    let effective_rebate_bps = if amount_in > 0 {
        ((total_rebate as u128 * BPS_DENOMINATOR as u128) / amount_in as u128) as u64
    } else {
        0
    };
    
    RebateResult {
        base_rebate: base_rebate.min(u64::MAX as u128) as u64,
        improvement_bonus: improvement_bonus.min(u64::MAX as u128) as u64,
        total_rebate,
        effective_rebate_bps,
        was_capped,
        cap_reason,
    }
}

/// Apply rebate caps and return final amount
fn apply_rebate_caps(
    rebate_amount: u128,
    buffer_params: &BufferParams,
    amount_in: u64,
) -> (u64, bool, Option<String>) {
    let rebate_u64 = rebate_amount.min(u64::MAX as u128) as u64;
    
    // Check transaction cap
    if rebate_u64 > buffer_params.rebate_cap_tx {
        return (buffer_params.rebate_cap_tx, true, Some("Transaction cap".to_string()));
    }
    
    // Check epoch remaining capacity
    if rebate_u64 > buffer_params.rebate_cap_epoch_remaining {
        return (buffer_params.rebate_cap_epoch_remaining, true, Some("Epoch cap".to_string()));
    }
    
    // Check available tau
    if rebate_u64 > buffer_params.available_tau {
        return (buffer_params.available_tau, true, Some("Insufficient buffer".to_string()));
    }
    
    // Check percentage of transaction (e.g., max 10% rebate)
    let max_rebate_pct = amount_in / 10; // 10% max
    if rebate_u64 > max_rebate_pct {
        return (max_rebate_pct, true, Some("Percentage cap".to_string()));
    }
    
    (rebate_u64, false, None)
}

/// Calculate price improvement for a swap
pub fn calculate_price_improvement(
    oracle_price: u128,
    execution_price: u128,
    is_buy: bool,
) -> PriceImprovement {
    let improvement_bps = if is_buy {
        // For buys: improvement when execution price < oracle price
        if execution_price < oracle_price {
            let diff = oracle_price - execution_price;
            ((diff * BPS_DENOMINATOR as u128) / oracle_price) as u64
        } else {
            0
        }
    } else {
        // For sells: improvement when execution price > oracle price
        if execution_price > oracle_price {
            let diff = execution_price - oracle_price;
            ((diff * BPS_DENOMINATOR as u128) / oracle_price) as u64
        } else {
            0
        }
    };
    
    PriceImprovement {
        oracle_price,
        execution_price,
        improvement_bps,
        is_buy,
    }
}

/// Quote rebate for a potential trade
pub fn quote_rebate(
    expected_work: i128,
    amount_in: u64,
    oracle_price: u128,
    expected_execution_price: u128,
    is_buy: bool,
    buffer_params: &BufferParams,
) -> RebateResult {
    // Calculate expected price improvement
    let price_improvement = calculate_price_improvement(
        oracle_price,
        expected_execution_price,
        is_buy,
    );
    
    // Calculate rebate
    calculate_rebate_with_improvement(
        expected_work,
        amount_in,
        &price_improvement,
        buffer_params,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_price_improvement_calculation() {
        // Test buy with improvement
        let improvement = calculate_price_improvement(
            100 * Q64, // Oracle price: 100
            95 * Q64,  // Execution price: 95 (better for buyer)
            true,      // Buy order
        );
        assert_eq!(improvement.improvement_bps, 500); // 5% improvement
        
        // Test sell with improvement
        let improvement = calculate_price_improvement(
            100 * Q64, // Oracle price: 100
            105 * Q64, // Execution price: 105 (better for seller)
            false,     // Sell order
        );
        assert_eq!(improvement.improvement_bps, 500); // 5% improvement
    }
    
    #[test]
    fn test_rebate_calculation() {
        let buffer_params = BufferParams {
            rebate_eta: 5000,      // 50% participation
            kappa: 1000,           // 10% price improvement clamp
            available_tau: 1000,
            rebate_cap_tx: 500,
            rebate_cap_epoch_remaining: 10000,
        };
        
        let price_improvement = PriceImprovement {
            oracle_price: 100 * Q64,
            execution_price: 95 * Q64,
            improvement_bps: 500, // 5%
            is_buy: true,
        };
        
        // Negative work (rebate scenario)
        let result = calculate_rebate_with_improvement(
            -1000, // Negative work
            10000, // Amount in
            &price_improvement,
            &buffer_params,
        );
        
        assert!(result.base_rebate > 0);
        assert!(result.improvement_bonus > 0);
        assert!(result.total_rebate > 0);
    }
}