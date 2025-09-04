//! # Thermodynamic Work and Fee Calculation
//! 
//! This module implements the core thermodynamic model of the Feels Protocol,
//! combining physics-based work calculations with fee/rebate determination.
//! 
//! ## Theory
//! 
//! The protocol models trading as movement through a 3D state space with coordinates:
//! - S (Spot): Standard AMM trading
//! - T (Time): Duration-weighted positions  
//! - L (Leverage): Leveraged positions
//! 
//! Work W is calculated as the line integral along trading paths:
//! W = ∫ ∇V · dr where V is the potential function
//! 
//! ## Fee Model
//! 
//! - **W > 0 (uphill)**: Trader pays fee = W / Π_in
//! - **W < 0 (downhill)**: Trader receives rebate = min(|W| / Π_out, κ × price_improvement)
//! 
//! Where κ is the price improvement clamp factor preventing excessive rebates.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;
use crate::state::{MarketField, BufferAccount, UnifiedOracle, FieldCommitment};
use feels_core::constants::{Q64, BASIS_POINTS_DENOMINATOR};
use crate::utils::math::safe;

// Import core types from feels-core
use feels_core::types::{Position3D, WorkResult, PathSegment, TradeDimension};
use feels_core::physics::Gradient3D;

// ============================================================================
// Work Calculation Core Functions
// ============================================================================

/// Calculate total work along a trading path using thermodynamic principles
pub fn calculate_path_work(
    segments: &[PathSegment],
    market_field: &MarketField,
) -> Result<WorkResult> {
    if segments.is_empty() {
        return Ok(WorkResult {
            total_work: 0,
            net_work: 0,
            weighted_work: 0,
            segments: 0,
        });
    }
    
    let mut total_work = 0u128;
    let mut weighted_work = 0u128;
    let mut total_distance = 0u128;
    
    // Calculate work for each segment
    for segment in segments {
        // Calculate gradient ∇V at midpoint
        let gradient = calculate_gradient(
            &segment.start,
            &segment.end,
            market_field,
        )?;
        
        // Work = ∇V · displacement
        let segment_work = calculate_segment_work(
            segment,
            &gradient,
            market_field,
        )?;
        
        // Accumulate work
        total_work = total_work.saturating_add(segment_work);
        
        // Weight by distance for averaging
        weighted_work = weighted_work.saturating_add(
            segment_work.saturating_mul(segment.distance)
        );
        total_distance = total_distance.saturating_add(segment.distance);
    }
    
    // Calculate weighted average
    let avg_work = if total_distance > 0 {
        weighted_work / total_distance
    } else {
        0
    };
    
    // Calculate net work (considering direction)
    let net_work = calculate_net_work(segments, market_field)?;
    
    Ok(WorkResult {
        total_work,
        net_work,
        weighted_work: avg_work,
        segments: segments.len() as u32,
    })
}

/// Calculate gradient of potential function V at a position
fn calculate_gradient(
    start: &Position3D,
    end: &Position3D,
    market_field: &MarketField,
) -> Result<Gradient3D> {
    // Use finite differences to approximate gradient
    // ∇V ≈ (V(end) - V(start)) / ||end - start||
    
    let dS = if end.S > start.S {
        end.S - start.S
    } else {
        start.S - end.S
    } as i128;
    
    let dT = if end.T > start.T {
        end.T - start.T
    } else {
        start.T - end.T
    } as i128;
    
    let dL = if end.L > start.L {
        end.L - start.L  
    } else {
        start.L - end.L
    } as i128;
    
    // Apply field weights to get gradient components
    let grad_S = (dS * market_field.S as i128) / Q64 as i128;
    let grad_T = (dT * market_field.T as i128) / Q64 as i128;
    let grad_L = (dL * market_field.L as i128) / Q64 as i128;
    
    Ok(Gradient3D {
        grad_s: grad_S,
        grad_t: grad_T,
        grad_l: grad_L,
    })
}

/// Calculate work for a single path segment
fn calculate_segment_work(
    segment: &PathSegment,
    gradient: &Gradient3D,
    _market_field: &MarketField,
) -> Result<u128> {
    // Calculate displacement vector
    let delta_S = (segment.end.S as i128) - (segment.start.S as i128);
    let delta_T = (segment.end.T as i128) - (segment.start.T as i128);
    let delta_L = (segment.end.L as i128) - (segment.start.L as i128);
    
    // Work = ∇V · Δposition
    let work_S = gradient.grad_s.saturating_mul(delta_S);
    let work_T = gradient.grad_t.saturating_mul(delta_T);
    let work_L = gradient.grad_l.saturating_mul(delta_L);
    
    let total_work_signed = work_S
        .saturating_add(work_T)
        .saturating_add(work_L);
    
    // Return absolute value as work is always positive for fee calculation
    Ok(total_work_signed.unsigned_abs() / Q64)
}

/// Calculate net work considering direction
fn calculate_net_work(
    segments: &[PathSegment],
    market_field: &MarketField,
) -> Result<i128> {
    let mut net_work = 0i128;
    
    for segment in segments {
        // Calculate potential difference
        let v_start = calculate_potential(&segment.start, market_field)?;
        let v_end = calculate_potential(&segment.end, market_field)?;
        
        let delta_v = v_end - v_start;
        net_work = net_work.saturating_add(delta_v);
    }
    
    Ok(net_work)
}

/// Calculate potential V at a position (simplified)
fn calculate_potential(
    position: &Position3D,
    market_field: &MarketField,
) -> Result<i128> {
    // V = S·σ_S + T·σ_T + L·σ_L (linearized version)
    let v_s = (position.S as i128 * market_field.S as i128) / Q64 as i128;
    let v_t = (position.T as i128 * market_field.T as i128) / Q64 as i128;
    let v_l = (position.L as i128 * market_field.L as i128) / Q64 as i128;
    
    Ok(v_s + v_t + v_l)
}

/// Simplified work calculation for swaps
pub fn calculate_swap_work(
    sqrt_price_start: u128,
    sqrt_price_end: u128,
    liquidity: u128,
    amount: u64,
) -> Result<u128> {
    // Simplified work = |ΔP| * L * amount / Q64
    let price_change = if sqrt_price_end > sqrt_price_start {
        sqrt_price_end - sqrt_price_start
    } else {
        sqrt_price_start - sqrt_price_end
    };
    
    let work = safe::mul_div_u128(
        price_change,
        safe::mul_u128(liquidity, amount as u128)?,
        Q64 * Q64
    )?;
    
    Ok(work)
}

// ============================================================================
// Fee Calculation Types
// ============================================================================

/// Extended fee parameters with on-chain specific fields
#[derive(Debug, Clone)]
pub struct ThermodynamicFeeParams {
    /// Work performed (can be negative for downhill)
    pub work: i128,
    /// Amount being traded
    pub amount_in: u64,
    /// Execution price achieved
    pub execution_price: u128,
    /// Oracle reference price
    pub oracle_price: u128,
    /// Base fee rate in basis points
    pub base_fee_bps: u16,
    /// Price improvement clamp factor
    pub kappa: u32,
    /// Maximum rebate allowed
    pub max_rebate_bps: u64,
    /// Is this a buy order
    pub is_buy: bool,
    /// Buffer state for rebate capacity
    pub buffer: Option<BufferAccount>,
}

/// Price improvement data for fee/rebate calculation
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PriceImprovement {
    /// Oracle price (safe TWAP)
    pub oracle_price: u128,
    /// Execution price
    pub execution_price: u128,
    /// Improvement in basis points (positive = better than oracle)
    pub improvement_bps: u64,
    /// Is buy order (affects improvement direction)
    pub is_buy: bool,
}

/// Result of thermodynamic fee calculation
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ThermodynamicFeeResult {
    /// Fee amount charged
    pub fee_amount: u64,
    /// Rebate amount paid
    pub rebate_amount: u64,
    /// Price improvement in basis points
    pub price_improvement_bps: u64,
    /// Effective fee rate in basis points
    pub effective_fee_bps: u64,
    /// Work performed
    pub work_performed: i128,
}

/// Result of order fee calculation
#[derive(Debug)]
pub struct OrderFeeResult {
    /// Amount in
    pub amount_in: u64,
    /// Amount out after fees
    pub amount_out: u64,
    /// Fee charged
    pub fee_amount: u64,
    /// Rebate paid (if any)
    pub rebate_amount: u64,
    /// Price improvement data
    pub price_improvement: Option<PriceImprovement>,
}

// ============================================================================
// Core Thermodynamic Fee Calculation
// ============================================================================

/// Calculate fees based on thermodynamic work and price improvement
/// This is the authoritative fee calculation function
pub fn calculate_thermodynamic_fee(
    params: ThermodynamicFeeParams,
) -> Result<ThermodynamicFeeResult> {
    // Calculate base fee from work
    let base_fee = if params.work > 0 {
        // Uphill work: trader pays fee
        work_to_fee(params.work as u128, params.base_fee_bps)?
    } else {
        0 // Downhill work: no base fee, may receive rebate
    };
    
    // Calculate price improvement
    let improvement_bps = calculate_price_improvement_bps(
        params.execution_price,
        params.oracle_price,
        params.is_buy,
    );
    
    // Apply κ-clamping: fee = max(0, W - κ × price_improvement)
    let improvement_discount = safe::mul_div_u64(
        improvement_bps,
        params.kappa as u64,
        BASIS_POINTS_DENOMINATOR as u64
    )?;
    
    // Calculate final fee (can be negative, indicating rebate)
    let final_fee = if base_fee > improvement_discount {
        base_fee - improvement_discount
    } else {
        0
    };
    
    // Calculate rebate if price improvement exceeds work
    let mut rebate_amount = 0u64;
    if improvement_discount > base_fee && params.work < 0 {
        rebate_amount = (improvement_discount - base_fee).min(params.max_rebate_bps);
        
        // Check buffer capacity if provided
        if let Some(buffer) = &params.buffer {
            rebate_amount = rebate_amount.min(buffer.available_rebate());
        }
    }
    
    Ok(ThermodynamicFeeResult {
        fee_amount: final_fee,
        rebate_amount,
        price_improvement_bps: improvement_bps,
        effective_fee_bps: calculate_effective_fee_bps(final_fee, params.amount_in),
        work_performed: params.work,
    })
}

/// Convert work to fee amount using base fee rate
pub fn work_to_fee(work: u128, base_fee_rate_bps: u16) -> Result<u64> {
    // Fee = work * base_fee_rate / 10000
    let fee = safe::mul_div_u128(
        work,
        base_fee_rate_bps as u128,
        BASIS_POINTS_DENOMINATOR as u128
    )?;
    
    // Ensure it fits in u64
    if fee > u64::MAX as u128 {
        return Err(FeelsProtocolError::MathOverflow.into());
    }
    
    Ok(fee as u64)
}

/// Calculate price improvement in basis points
pub fn calculate_price_improvement_bps(
    execution_price: u128,
    oracle_price: u128,
    is_buy: bool,
) -> u64 {
    if oracle_price == 0 {
        return 0;
    }
    
    let improvement_bps = if is_buy {
        // Buy order: lower execution price is better
        if execution_price < oracle_price {
            safe::mul_div_u128(
                oracle_price - execution_price,
                BASIS_POINTS_DENOMINATOR as u128,
                oracle_price
            ).unwrap_or(0)
        } else {
            0
        }
    } else {
        // Sell order: higher execution price is better  
        if execution_price > oracle_price {
            safe::mul_div_u128(
                execution_price - oracle_price,
                BASIS_POINTS_DENOMINATOR as u128,
                oracle_price
            ).unwrap_or(0)
        } else {
            0
        }
    };
    
    improvement_bps.min(BASIS_POINTS_DENOMINATOR as u128) as u64
}

/// Calculate effective fee rate in basis points
pub fn calculate_effective_fee_bps(fee_amount: u64, amount_in: u64) -> u64 {
    if amount_in == 0 {
        return 0;
    }
    
    safe::mul_div_u64(
        fee_amount,
        BASIS_POINTS_DENOMINATOR as u64,
        amount_in
    ).unwrap_or(u64::MAX)
}

// ============================================================================
// Order Fee Calculation
// ============================================================================

/// Calculate order fees using the thermodynamic model
pub fn calculate_order_fees(
    amount_in: u64,
    amount_out_raw: u64,
    work: u128,
    oracle_price: u128,
    execution_price: u128,
    base_fee_rate: u16,
    kappa: u32,
    is_buy: bool,
    buffer: Option<&BufferAccount>,
) -> Result<OrderFeeResult> {
    // Build fee parameters
    let params = ThermodynamicFeeParams {
        work: work as i128,
        amount_in,
        execution_price,
        oracle_price,
        base_fee_bps: base_fee_rate,
        kappa,
        max_rebate_bps: amount_out_raw / 100, // Cap at 1%
        is_buy,
        buffer: buffer.cloned(),
    };
    
    // Calculate fees
    let fee_result = calculate_thermodynamic_fee(params)?;
    
    // Build price improvement data
    let price_improvement = PriceImprovement {
        oracle_price,
        execution_price,
        improvement_bps: fee_result.price_improvement_bps,
        is_buy,
    };
    
    // Calculate final amount out
    let amount_out = amount_out_raw
        .saturating_sub(fee_result.fee_amount)
        .saturating_add(fee_result.rebate_amount);
    
    Ok(OrderFeeResult {
        amount_in,
        amount_out,
        fee_amount: fee_result.fee_amount,
        rebate_amount: fee_result.rebate_amount,
        price_improvement: Some(price_improvement),
    })
}

// ============================================================================
// 3D Fee Calculation
// ============================================================================

/// Calculate fees for 3D multi-dimensional orders
pub fn calculate_3d_fees(
    position_start: Position3D,
    position_end: Position3D,
    amount: u64,
    market_field: &MarketField,
    oracle: &UnifiedOracle,
) -> Result<OrderFeeResult> {
    // Create path segment
    let segment = PathSegment {
        start: position_start,
        end: position_end,
        liquidity: market_field.S, // Use spot scalar as proxy
        distance: amount as u128,
        dimension: TradeDimension::Mixed, // 3D movement
    };
    
    // Calculate work using thermodynamic principles
    let work_result = calculate_path_work(&[segment], market_field)?;
    
    // Get prices for improvement calculation
    let oracle_price = oracle.get_safe_twap_a();
    let execution_price = position_end.S; // Use spot dimension as price
    
    // Calculate fees
    calculate_order_fees(
        amount,
        amount, // Simplified - would calculate actual output
        work_result.weighted_work,
        oracle_price,
        execution_price,
        market_field.base_fee_rate,
        market_field.kappa_fee as u32,
        true, // Assume buy for now
        None, // Buffer passed separately
    )
}

// ============================================================================
// Swap Fee Calculation
// ============================================================================

/// Calculate fees for a single swap step
pub fn calculate_swap_step_fees(
    amount_in: u64,
    amount_out_raw: u64,
    sqrt_price_start: u128,
    sqrt_price_end: u128,
    liquidity: u128,
    oracle: &UnifiedOracle,
    commitment: &FieldCommitment,
    buffer: &BufferAccount,
    zero_for_one: bool,
) -> Result<(u64, u64, u64)> {
    // Calculate work for this swap step
    let work = calculate_swap_work(
        sqrt_price_start,
        sqrt_price_end,
        liquidity,
        amount_in,
    )?;
    
    // Get oracle price
    let oracle_price = if zero_for_one {
        oracle.get_safe_twap_a()
    } else {
        oracle.get_safe_twap_b()
    };
    
    // Calculate fees
    let fee_result = calculate_order_fees(
        amount_in,
        amount_out_raw,
        work,
        oracle_price,
        sqrt_price_end,
        commitment.base_fee_bps as u16,
        commitment.kappa as u32,
        !zero_for_one, // Buy if selling token 1
        Some(buffer),
    )?;
    
    Ok((
        amount_in,
        fee_result.amount_out,
        fee_result.fee_amount,
    ))
}

// ============================================================================
// Rebate Management
// ============================================================================

/// Validate rebate capacity in buffer
pub fn validate_rebate_capacity(
    buffer: &BufferAccount,
    rebate_amount: u64,
    current_time: i64,
) -> Result<()> {
    // Check per-transaction limit
    let max_per_tx = buffer.get_max_rebate(
        rebate_amount as u128,
        crate::state::buffer::MAX_REBATE_PER_TX_BPS,
    );
    
    require!(
        rebate_amount <= max_per_tx,
        FeelsProtocolError::InsufficientBuffer
    );
    
    // Check epoch limit
    if buffer.check_epoch_reset(current_time) {
        // Epoch was reset, rebate is allowed
        return Ok(());
    }
    
    let epoch_remaining = buffer.max_rebate_per_epoch
        .saturating_sub(buffer.epoch_rebates_paid);
    
    require!(
        rebate_amount <= epoch_remaining,
        FeelsProtocolError::InsufficientBuffer
    );
    
    Ok(())
}

/// Process rebate payment from buffer
pub fn process_rebate_payment(
    buffer: &mut BufferAccount,
    rebate_amount: u64,
    current_time: i64,
) -> Result<()> {
    // Validate capacity
    validate_rebate_capacity(buffer, rebate_amount, current_time)?;
    
    // Update buffer state using state module function
    crate::state::buffer::pay_rebate(buffer, rebate_amount, current_time)
}

/// Distribute collected fees to buffer
pub fn distribute_fees_to_buffer(
    buffer: &mut BufferAccount,
    fee_amount: u64,
    token_index: u8,
    current_time: i64,
) -> Result<()> {
    // Use state module function for consistency
    crate::state::buffer::collect_fee(buffer, fee_amount, token_index, current_time)
}

// ============================================================================
// Price Improvement Calculation
// ============================================================================

/// Calculate price improvement between execution and oracle price
pub fn calculate_price_improvement(
    execution_price: u128,
    oracle: &UnifiedOracle,
    amount_in: u64,
    is_buy: bool,
) -> Result<PriceImprovement> {
    // Get safe oracle price (1hr TWAP for stability)
    let oracle_price = oracle.twap_1hr_a;
    require!(oracle_price > 0, FeelsProtocolError::InvalidAmount);
    
    // Calculate improvement
    let improvement_bps = calculate_price_improvement_bps(
        execution_price,
        oracle_price,
        is_buy,
    );
    
    Ok(PriceImprovement {
        oracle_price,
        execution_price,
        improvement_bps,
        is_buy,
    })
}

// ============================================================================
// Fallback Fee Calculation
// ============================================================================

/// Simple percentage-based fee for fallback mode
pub fn calculate_fallback_fees(
    amount_in: u64,
    base_fee_rate: u16,
) -> Result<u64> {
    safe::mul_div_u64(
        amount_in,
        base_fee_rate as u64,
        BASIS_POINTS_DENOMINATOR as u64
    )
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Apply thermodynamic fees to an order
pub fn apply_thermodynamic_fees(
    amount_in: u64,
    amount_out_before_fees: u64,
    fee_result: &ThermodynamicFeeResult,
) -> Result<(u64, u64)> {
    // Apply fee
    let amount_out_after_fee = amount_out_before_fees
        .saturating_sub(fee_result.fee_amount);
    
    // Apply rebate
    let final_amount_out = amount_out_after_fee
        .saturating_add(fee_result.rebate_amount);
    
    Ok((amount_in, final_amount_out))
}