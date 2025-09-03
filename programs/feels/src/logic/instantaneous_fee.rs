/// Instantaneous fee model implementation with price improvement clamping.
/// Implements the formula: fee = max(0, W - κ * price_improvement)
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{
    BufferAccount, TwapOracle, MarketField, FieldCommitment,
    buffer::{/* PriceImprovement, */ calculate_price_improvement, calculate_instantaneous_fee},
};
use crate::constant::Q64;

// ============================================================================
// Order Result with Price Improvement
// ============================================================================

/// Enhanced order result tracking price improvement and instantaneous fees
#[derive(Clone, Debug, Default)]
pub struct InstantaneousOrderResult {
    /// Standard order results
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_rate_after: u128,
    
    /// Price improvement tracking
    pub oracle_price: u128,
    pub execution_price: u128,
    pub price_improvement_bps: u64,
    
    /// Work and fee/rebate amounts
    pub work_done: i128,
    pub fee_amount: u64,
    pub rebate_amount: u64,
    
    /// Fee breakdown
    pub base_work: i128,
    pub kappa_discount: u64,
    pub effective_fee_bps: u64,
}

// ============================================================================
// Fee Calculation Functions
// ============================================================================

/// Calculate instantaneous fees for an order using work and price improvement
pub fn calculate_order_fees_instantaneous(
    amount_in: u64,
    amount_out: u64,
    is_token_a_to_b: bool,
    work: i128,
    oracle: &TwapOracle,
    buffer: &BufferAccount,
) -> Result<InstantaneousOrderResult> {
    // Calculate execution price using safe math to prevent overflow
    let execution_price = if is_token_a_to_b {
        // Token 0 -> Token 1: price = amount_out / amount_in
        let shifted_out = crate::utils::math::safe::safe_shl_u128(amount_out as u128, 64)?;
        crate::utils::math::safe::div_u128(shifted_out, (amount_in as u128).max(1))?
    } else {
        // Token 1 -> Token 0: price = amount_in / amount_out
        let shifted_in = crate::utils::math::safe::safe_shl_u128(amount_in as u128, 64)?;
        crate::utils::math::safe::div_u128(shifted_in, (amount_out as u128).max(1))?
    };
    
    // Get oracle price (TWAP) using safe math for division
    let oracle_price = if is_token_a_to_b {
        oracle.twap_1_per_0
    } else {
        // Calculate A per B from B per A using safe division
        if oracle.twap_1_per_0 > 0 {
            crate::utils::math::safe::div_u128(crate::constant::Q64, oracle.twap_1_per_0)?
        } else {
            crate::constant::Q64 // Default to 1:1 if no price
        }
    };
    
    // Calculate price improvement
    let price_improvement = calculate_price_improvement(
        oracle_price,
        execution_price,
        is_token_a_to_b, // is_buy from user perspective
    );
    
    // Calculate instantaneous fee/rebate
    let (fee_amount, rebate_amount) = calculate_instantaneous_fee(
        work,
        &price_improvement,
        buffer,
    )?;
    
    // Calculate effective fee in basis points using safe math
    let effective_fee_bps = if amount_in > 0 {
        let scaled_fee = crate::utils::math::safe::mul_u128(fee_amount as u128, 10000)?;
        crate::utils::math::safe::div_u128(scaled_fee, amount_in as u128)? as u64
    } else {
        0
    };
    
    // Calculate κ discount amount
    let kappa_discount = ((buffer.kappa as u128)
        .saturating_mul(price_improvement.improvement_bps as u128)
        .saturating_div(10000)) as u64;
    
    Ok(InstantaneousOrderResult {
        amount_in,
        amount_out,
        sqrt_rate_after: 0, // To be filled by caller
        oracle_price,
        execution_price,
        price_improvement_bps: price_improvement.improvement_bps,
        work_done: work,
        fee_amount,
        rebate_amount,
        base_work: work,
        kappa_discount,
        effective_fee_bps,
    })
}

/// Calculate fees for 3D orders using field commitment and work
pub fn calculate_3d_fees_instantaneous(
    amount_in: u64,
    is_token_a_to_b: bool,
    work: i128,
    field_commitment: &FieldCommitment,
    oracle: &TwapOracle,
    buffer: &BufferAccount,
) -> Result<InstantaneousOrderResult> {
    // Estimate output amount using field commitment data using safe math
    // This is a simplified calculation - actual implementation would use
    // the full 3D AMM math
    let estimated_rate = if is_token_a_to_b {
        crate::utils::math::safe::div_u128(field_commitment.twap_1, field_commitment.twap_0.max(1))?
    } else {
        crate::utils::math::safe::div_u128(field_commitment.twap_0, field_commitment.twap_1.max(1))?
    };
    
    let amount_out = ((amount_in as u128)
        .saturating_mul(estimated_rate)
        .saturating_div(Q64)) as u64;
    
    // Use standard instantaneous fee calculation
    calculate_order_fees_instantaneous(
        amount_in,
        amount_out,
        is_token_a_to_b,
        work,
        oracle,
        buffer,
    )
}

// ============================================================================
// Integration Helpers
// ============================================================================

/// Apply instantaneous fees to an order
pub fn apply_instantaneous_fees(
    amount_in: u64,
    amount_out: u64,
    fee_amount: u64,
    rebate_amount: u64,
    is_exact_in: bool,
) -> Result<(u64, u64)> {
    if is_exact_in {
        // Exact in: reduce output by fee, increase by rebate
        let adjusted_out = amount_out
            .saturating_sub(fee_amount)
            .saturating_add(rebate_amount);
        Ok((amount_in, adjusted_out))
    } else {
        // Exact out: increase input by fee, decrease by rebate
        let adjusted_in = amount_in
            .saturating_add(fee_amount)
            .saturating_sub(rebate_amount);
        Ok((adjusted_in, amount_out))
    }
}

/// Validate buffer has sufficient funds for rebate
pub fn validate_rebate_capacity(
    buffer: &BufferAccount,
    rebate_amount: u64,
    current_time: i64,
) -> Result<()> {
    // Check epoch boundary
    let in_current_epoch = current_time < buffer.epoch_start + buffer.epoch_duration;
    
    // Get available rebate capacity
    let epoch_remaining = if in_current_epoch {
        buffer.rebate_cap_epoch.saturating_sub(buffer.rebate_paid_epoch)
    } else {
        buffer.rebate_cap_epoch
    };
    
    let tx_cap = buffer.rebate_cap_tx;
    let available_tau = buffer.get_available_tau()?;
    
    // Check all constraints
    let max_rebate = rebate_amount
        .min(tx_cap)
        .min(epoch_remaining)
        .min(available_tau);
    
    require!(
        max_rebate >= rebate_amount,
        FeelsProtocolError::InsufficientBuffer
    );
    
    Ok(())
}

// ============================================================================
// Fee Distribution
// ============================================================================

/// Distribute collected fees to buffer and update statistics
pub fn distribute_fees_to_buffer(
    buffer: &mut BufferAccount,
    fee_amount: u64,
    dimension_weights: (u32, u32, u32), // (spot, time, leverage)
) -> Result<()> {
    // Collect fee into buffer
    buffer.collect_fees(fee_amount as u128)?;
    
    // Update fee shares based on dimension weights
    let (w_spot, w_time, w_leverage) = dimension_weights;
    let total_weight = (w_spot + w_time + w_leverage) as u64;
    
    if total_weight > 0 {
        // Calculate fee distribution using safe math to prevent overflow
        let scaled_spot = crate::utils::math::safe::mul_u128(fee_amount as u128, w_spot as u128)?;
        let spot_fees = crate::utils::math::safe::div_u128(scaled_spot, total_weight as u128)? as u64;
        
        let scaled_time = crate::utils::math::safe::mul_u128(fee_amount as u128, w_time as u128)?;
        let time_fees = crate::utils::math::safe::div_u128(scaled_time, total_weight as u128)? as u64;
        
        let leverage_fees = fee_amount.saturating_sub(spot_fees).saturating_sub(time_fees);
        
        buffer.update_fee_shares(
            spot_fees,
            time_fees,
            leverage_fees,
            Clock::get()?.unix_timestamp,
        )?;
    }
    
    Ok(())
}

/// Process rebate payment from buffer
pub fn process_rebate_payment(
    buffer: &mut BufferAccount,
    rebate_amount: u64,
    current_time: i64,
) -> Result<u64> {
    // Pay rebate with all caps applied
    let actual_rebate = buffer.pay_rebate(rebate_amount, current_time)?;
    
    msg!("Rebate processed: requested={}, paid={}", rebate_amount, actual_rebate);
    
    Ok(actual_rebate)
}

// ============================================================================
// Fallback Mode
// ============================================================================

/// Calculate fees in fallback mode when field commitments are stale
pub fn calculate_fallback_fees(
    amount_in: u64,
    market_field: &MarketField,
    fees_policy: &crate::state::FeesPolicy,
    current_volatility_bps: Option<u64>,
) -> Result<u64> {
    // Use fallback fee from policy
    let base_fee_bps = fees_policy.fallback_fee_bps;
    
    // Adjust for volatility if available
    let volatility_multiplier = if let Some(vol_bps) = current_volatility_bps {
        // Scale volatility: 1000 bps (10%) = 1x, 5000 bps (50%) = 3x
        let multiplier = 10000 + (vol_bps.saturating_sub(1000) * 2).min(20000);
        multiplier / 10000
    } else {
        // Use field commitment volatility as fallback
        let sigma_normalized = (market_field.sigma_price as u128 * 10000 / Q64) as u64; // Convert to bps
        (10000u64 + sigma_normalized.min(20000)) / 10000
    };
    
    // Apply time-based decay multiplier
    let time_multiplier = calculate_time_decay_multiplier(market_field)?;
    
    // Calculate fee with multipliers
    let fee = (amount_in as u128)
        .saturating_mul(base_fee_bps as u128)
        .saturating_mul(volatility_multiplier as u128)
        .saturating_mul(time_multiplier as u128)
        .saturating_div(10000) // basis points
        .saturating_div(10000) // volatility scale
        .saturating_div(10000) // time scale
        .min(u64::MAX as u128) as u64;
    
    // Ensure minimum fee
    let min_fee = (amount_in as u128)
        .saturating_mul(fees_policy.min_base_fee_bps as u128)
        .saturating_div(10000)
        .min(u64::MAX as u128) as u64;
    
    let final_fee = fee.max(min_fee);
    
    msg!(
        "Fallback fee: base={} bps, vol_mult={}x, time_mult={}x, fee={}",
        base_fee_bps,
        volatility_multiplier,
        time_multiplier,
        final_fee
    );
    
    Ok(final_fee)
}

/// Calculate time-based decay multiplier for fallback fees
fn calculate_time_decay_multiplier(market_field: &MarketField) -> Result<u64> {
    // Higher T scalar indicates more time dimension activity
    // Scale from 0.5x to 2x based on T value
    let t_normalized = ((market_field.T as u128 * 10000) / Q64) as u64;
    
    // Linear scaling: 0 -> 5000 (0.5x), 10000 -> 20000 (2x)
    let multiplier = 5000u64 + t_normalized.min(10000);
    
    Ok(multiplier)
}

// ============================================================================
// Order Integration
// ============================================================================

/// Calculate work for a swap step (simplified version for on-chain efficiency)
/// Full path integration is available in the SDK at crates/sdk/src/work_calc.rs
pub fn calculate_swap_work_simplified(
    sqrt_rate_current: u128,
    sqrt_rate_next: u128,
    liquidity: u128,
    zero_for_one: bool,
    market_field: &MarketField,
) -> Result<i128> {
    // Simplified work calculation based on price movement
    // W ≈ liquidity * |ln(P_end/P_start)| * weight
    
    // Convert sqrt rates to prices using safe math to prevent overflow
    let price_current = {
        let squared = crate::utils::math::safe::mul_u128(sqrt_rate_current, sqrt_rate_current)?;
        crate::utils::math::safe::safe_shr_u128(squared, 96)?
    };
    let price_next = {
        let squared = crate::utils::math::safe::mul_u128(sqrt_rate_next, sqrt_rate_next)?;
        crate::utils::math::safe::safe_shr_u128(squared, 96)?
    };
    
    // Calculate price ratio (avoiding ln on-chain) using safe math
    let price_ratio = if zero_for_one {
        // Price decreasing
        let scaled_current = crate::utils::math::safe::mul_u128(price_current, crate::constant::Q64)?;
        crate::utils::math::safe::div_u128(scaled_current, price_next.max(1))?
    } else {
        // Price increasing
        let scaled_next = crate::utils::math::safe::mul_u128(price_next, crate::constant::Q64)?;
        crate::utils::math::safe::div_u128(scaled_next, price_current.max(1))?
    };
    
    // Approximate ln(ratio) using linear approximation around 1 with safe math
    // ln(x) ≈ (x - 1) for x near 1
    let ln_approx = if price_ratio > crate::constant::Q64 {
        let diff = crate::utils::math::safe::sub_u128(price_ratio, crate::constant::Q64)?;
        let scaled = crate::utils::math::safe::mul_u128(diff, crate::constant::Q64)?;
        crate::utils::math::safe::div_u128(scaled, crate::constant::Q64)? as i128
    } else {
        let diff = crate::utils::math::safe::sub_u128(crate::constant::Q64, price_ratio)?;
        let scaled = crate::utils::math::safe::mul_u128(diff, crate::constant::Q64)?;
        -(crate::utils::math::safe::div_u128(scaled, crate::constant::Q64)? as i128)
    };
    
    // Apply liquidity and weight
    let work = (liquidity as i128)
        .saturating_mul(ln_approx.abs())
        .saturating_div(Q64 as i128)
        .saturating_mul(market_field.w_s as i128)
        .saturating_div(10000);
    
    // Work is positive for price improvement, negative for adverse selection
    if zero_for_one {
        Ok(if price_next < price_current { work } else { -work })
    } else {
        Ok(if price_next > price_current { work } else { -work })
    }
}

/// Enhanced swap step calculation with instantaneous fees
pub fn calculate_swap_step_with_instantaneous_fees(
    sqrt_rate_current: u128,
    sqrt_rate_next: u128,
    liquidity: u128,
    zero_for_one: bool,
    market_field: &MarketField,
    oracle: &TwapOracle,
    buffer: &BufferAccount,
) -> Result<(u64, u64, u64, u64)> { // (amount_in, amount_out, fee_amount, rebate_amount)
    use crate::utils::{get_amount_0_delta, get_amount_1_delta};
    
    // Calculate raw amounts
    let amount_in = if zero_for_one {
        get_amount_0_delta(sqrt_rate_next, sqrt_rate_current, liquidity, true)?
    } else {
        get_amount_1_delta(sqrt_rate_current, sqrt_rate_next, liquidity, true)?
    } as u64;
    
    let amount_out = if zero_for_one {
        get_amount_1_delta(sqrt_rate_next, sqrt_rate_current, liquidity, false)?
    } else {
        get_amount_0_delta(sqrt_rate_current, sqrt_rate_next, liquidity, false)?
    } as u64;
    
    // Calculate work for this step
    let work = calculate_swap_work_simplified(
        sqrt_rate_current,
        sqrt_rate_next,
        liquidity,
        zero_for_one,
        market_field,
    )?;
    
    // Calculate instantaneous fees
    let result = calculate_order_fees_instantaneous(
        amount_in,
        amount_out,
        zero_for_one,
        work,
        oracle,
        buffer,
    )?;
    
    Ok((amount_in, amount_out, result.fee_amount, result.rebate_amount))
}