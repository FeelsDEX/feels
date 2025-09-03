/// SDK-only mathematical functions for off-chain computation.
/// 
/// This module contains transcendental functions (ln, exp, pow, sqrt) that are too
/// computationally expensive for on-chain execution. These functions should only be
/// used by:
/// - SDK/client code for fee calculation
/// - Keeper services for field computation
/// - Tests and simulations
/// 
/// IMPORTANT: This file should NOT be included in the on-chain program build.
/// Add to Cargo.toml with cfg(not(target_os = "solana")) to exclude from BPF.

use crate::error::FeelsProtocolError;
use anchor_lang::prelude::*;
use fixed::types::I64F64;
use micromath::F32Ext;
use num_traits::{Zero, One};

/// Fixed-point number type
pub type FixedPoint = I64F64;

/// Natural logarithm using micromath (SDK only)
pub fn ln_fixed(x: FixedPoint) -> Result<FixedPoint> {
    if !x.is_positive() {
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    
    if x == FixedPoint::ONE {
        return Ok(FixedPoint::ZERO);
    }
    
    let x_f32 = x.to_num::<f32>();
    let ln_result = x_f32.ln();
    
    if !ln_result.is_finite() {
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    
    Ok(FixedPoint::from_num(ln_result))
}

/// Exponential function using micromath (SDK only)
pub fn exp_fixed(x: FixedPoint) -> Result<FixedPoint> {
    let x_f32 = x.to_num::<f32>();
    let exp_result = x_f32.exp();
    
    if !exp_result.is_finite() || exp_result <= 0.0 {
        return Err(FeelsProtocolError::MathOverflow.into());
    }
    
    Ok(FixedPoint::from_num(exp_result))
}

/// Power function (SDK only)
pub fn pow_fixed(base: FixedPoint, exponent: FixedPoint) -> Result<FixedPoint> {
    if base.is_zero() {
        return if exponent.is_positive() {
            Ok(FixedPoint::ZERO)
        } else {
            Err(FeelsProtocolError::DivisionByZero.into())
        };
    }
    
    if exponent.is_zero() { return Ok(FixedPoint::ONE); }
    if exponent == FixedPoint::ONE { return Ok(base); }
    
    let ln_base = ln_fixed(base)?;
    let y_ln_x = exponent.saturating_mul(ln_base);
    exp_fixed(y_ln_x)
}

/// Square root (SDK only)
pub fn sqrt_fixed(x: FixedPoint) -> Result<FixedPoint> {
    if x.is_negative() {
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    Ok(x.sqrt())
}

/// Logarithm ratio ln(a/b) (SDK only)
pub fn ln_ratio(a: FixedPoint, b: FixedPoint) -> Result<FixedPoint> {
    let ln_a = ln_fixed(a)?;
    let ln_b = ln_fixed(b)?;
    Ok(ln_a.saturating_sub(ln_b))
}

/// Calculate exact exponential rebase factor (SDK only)
/// Returns g = e^(rate * time_elapsed / seconds_per_year)
pub fn calculate_exact_rebase_factor(
    rate_bps: u64,
    time_elapsed: i64,
    seconds_per_year: i64,
) -> Result<FixedPoint> {
    const BPS_DENOMINATOR: u64 = 10_000;
    
    // Calculate exponent: rate * time / (bps * seconds_per_year)
    let rate_fp = FixedPoint::from_num(rate_bps) / FixedPoint::from_num(BPS_DENOMINATOR);
    let time_fp = FixedPoint::from_num(time_elapsed);
    let year_fp = FixedPoint::from_num(seconds_per_year);
    
    let exponent = rate_fp.saturating_mul(time_fp) / year_fp;
    
    // Return e^exponent
    exp_fixed(exponent)
}

/// Compute exact exponential rebase factors for lending/funding
/// Returns (g_lenders, g_borrowers, g_buffer) that satisfy conservation
pub fn compute_lending_rebase_factors(
    lender_rate_bps: u64,
    borrower_rate_bps: u64,
    time_elapsed: i64,
    lender_weight: u32,
    borrower_weight: u32,
    buffer_weight: u32,
) -> Result<(u128, u128, u128, i64)> {
    const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;
    const Q64: u128 = 1 << 64;
    
    // Compute exact exponential factors
    let g_lender_fp = calculate_exact_rebase_factor(lender_rate_bps, time_elapsed, SECONDS_PER_YEAR)?;
    let g_borrower_fp = calculate_exact_rebase_factor(borrower_rate_bps, time_elapsed, SECONDS_PER_YEAR)?;
    
    // Convert to Q64 format
    let g_lender = (g_lender_fp * FixedPoint::from_num(Q64)).to_num::<u128>();
    let g_borrower = (g_borrower_fp * FixedPoint::from_num(Q64)).to_num::<u128>();
    
    // Solve for buffer factor using conservation law
    let weights = vec![
        FixedPoint::from_num(lender_weight) / FixedPoint::from_num(10000),
        FixedPoint::from_num(borrower_weight) / FixedPoint::from_num(10000),
        FixedPoint::from_num(buffer_weight) / FixedPoint::from_num(10000),
    ];
    
    let factors = vec![
        Some(g_lender_fp),
        Some(g_borrower_fp),
        None, // Solve for buffer
    ];
    
    let result = solve_conservation_rebase(&weights, &factors)?;
    let g_buffer = (result[2] * FixedPoint::from_num(Q64)).to_num::<u128>();
    
    // Compute weighted log sum for verification
    let ln_g_lender = ln_fixed(g_lender_fp)?;
    let ln_g_borrower = ln_fixed(g_borrower_fp)?;
    let ln_g_buffer = ln_fixed(result[2])?;
    
    let weighted_sum = weights[0] * ln_g_lender + weights[1] * ln_g_borrower + weights[2] * ln_g_buffer;
    let weighted_sum_i64 = (weighted_sum * FixedPoint::from_num(Q64)).to_num::<i64>();
    
    Ok((g_lender, g_borrower, g_buffer, weighted_sum_i64))
}

/// Compute exact exponential rebase factors for leverage funding
/// Returns (g_long, g_short) that satisfy conservation
pub fn compute_leverage_rebase_factors(
    funding_rate_bps: i64,
    time_elapsed: i64,
    long_weight: u32,
    short_weight: u32,
) -> Result<(u128, u128, i64)> {
    const SECONDS_PER_YEAR: i64 = 365 * 24 * 60 * 60;
    const Q64: u128 = 1 << 64;
    
    // For leverage, one side gains what the other loses
    // If funding_rate > 0: longs pay shorts
    // If funding_rate < 0: shorts pay longs
    
    let rate_abs = funding_rate_bps.abs() as u64;
    let g_positive_fp = calculate_exact_rebase_factor(rate_abs, time_elapsed, SECONDS_PER_YEAR)?;
    
    // Solve for the other factor using conservation
    let weights = vec![
        FixedPoint::from_num(long_weight) / FixedPoint::from_num(10000),
        FixedPoint::from_num(short_weight) / FixedPoint::from_num(10000),
    ];
    
    let (factors, g_long, g_short) = if funding_rate_bps > 0 {
        // Longs pay: g_long < 1, g_short > 1
        // g_long = 1/g_positive (approximately)
        let g_long_fp = FixedPoint::ONE / g_positive_fp;
        let factors = vec![Some(g_long_fp), None];
        let result = solve_conservation_rebase(&weights, &factors)?;
        let g_long = (g_long_fp * FixedPoint::from_num(Q64)).to_num::<u128>();
        let g_short = (result[1] * FixedPoint::from_num(Q64)).to_num::<u128>();
        (result, g_long, g_short)
    } else {
        // Shorts pay: g_short < 1, g_long > 1
        let g_short_fp = FixedPoint::ONE / g_positive_fp;
        let factors = vec![None, Some(g_short_fp)];
        let result = solve_conservation_rebase(&weights, &factors)?;
        let g_long = (result[0] * FixedPoint::from_num(Q64)).to_num::<u128>();
        let g_short = (g_short_fp * FixedPoint::from_num(Q64)).to_num::<u128>();
        (result, g_long, g_short)
    };
    
    // Compute weighted log sum for verification
    let ln_g_long = ln_fixed(factors[0])?;
    let ln_g_short = ln_fixed(factors[1])?;
    
    let weighted_sum = weights[0] * ln_g_long + weights[1] * ln_g_short;
    let weighted_sum_i64 = (weighted_sum * FixedPoint::from_num(Q64)).to_num::<i64>();
    
    Ok((g_long, g_short, weighted_sum_i64))
}

/// Solve conservation law for rebasing (SDK only)
/// Given weights and known factors, solve for unknown factor such that:
/// Σ w_i * ln(g_i) = 0
pub fn solve_conservation_rebase(
    weights: &[FixedPoint],
    target_factors: &[Option<FixedPoint>],
) -> Result<Vec<FixedPoint>> {
    if weights.len() != target_factors.len() || weights.is_empty() {
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    
    let solve_index = target_factors.iter().position(|f| f.is_none())
        .ok_or(FeelsProtocolError::InvalidInput)?;
    
    if weights[solve_index].is_zero() {
        return Err(FeelsProtocolError::DivisionByZero.into());
    }
    
    let mut sum = FixedPoint::ZERO;
    for (i, (weight, factor_opt)) in weights.iter().zip(target_factors.iter()).enumerate() {
        if i != solve_index {
            if let Some(factor) = factor_opt {
                let ln_factor = ln_fixed(*factor)?;
                let weighted = weight.saturating_mul(ln_factor);
                sum = sum.saturating_add(weighted);
            }
        }
    }
    
    let target_ln = sum.saturating_neg() / weights[solve_index];
    let solved_factor = exp_fixed(target_ln)?;
    
    let mut result = Vec::new();
    for (i, factor_opt) in target_factors.iter().enumerate() {
        if i == solve_index {
            result.push(solved_factor);
        } else {
            result.push(factor_opt.unwrap());
        }
    }
    
    Ok(result)
}

/// Verify conservation law holds within tolerance (SDK only)
pub fn verify_conservation(
    weights: &[FixedPoint],
    factors: &[FixedPoint],
    tolerance: FixedPoint,
) -> Result<bool> {
    if weights.len() != factors.len() {
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    
    let mut sum = FixedPoint::ZERO;
    for (weight, factor) in weights.iter().zip(factors.iter()) {
        if factor.is_positive() {
            let ln_factor = ln_fixed(*factor)?;
            let weighted = weight.saturating_mul(ln_factor);
            sum = sum.saturating_add(weighted);
        }
    }
    
    Ok(sum.abs() <= tolerance)
}

/// Calculate work done along a path (SDK only)
pub fn calculate_ln_ratio(a: u128, b: u128) -> Result<i128> {
    let a_fp = FixedPoint::from_num(a);
    let b_fp = FixedPoint::from_num(b);
    
    let ln_a = ln_fixed(a_fp)?;
    let ln_b = ln_fixed(b_fp)?;
    
    Ok(ln_a.saturating_sub(ln_b).to_bits())
}

/// Solve for weight rebase factors when domain weights change
/// 
/// When weights change from (w_S, w_T, w_L, w_tau) to (w'_S, w'_T, w'_L, w'_tau),
/// we need factors (h_S, h_T, h_L, h_tau) such that:
/// 
/// 1. Conservation: S'^(w'_S) * T'^(w'_T) * L'^(w'_L) * tau'^(w'_tau) = K_acct
///    where S' = S * h_S, etc.
/// 
/// 2. Price continuity: S_a / S_b remains unchanged
///    This gives us: h_S = 1 (spot doesn't rebase)
/// 
/// 3. Additional constraint: We can fix h_tau = 1 (buffer doesn't rebase)
///    or another domain based on policy
/// 
/// This leaves us solving for h_T and h_L using Newton's method.
pub fn solve_weight_rebase(
    old_weights: &[u32; 4], // [w_S, w_T, w_L, w_tau] in basis points
    new_weights: &[u32; 4], // [w'_S, w'_T, w'_L, w'_tau] in basis points
    domain_values: &[u128; 4], // [S, T, L, tau] current values
    fix_spot: bool, // If true, h_S = 1 for price continuity
) -> Result<([u128; 4], i64)> {
    const Q64: u128 = 1 << 64;
    const MAX_ITERATIONS: usize = 50;
    const TOLERANCE: f64 = 1e-10;
    
    // Convert to normalized weights
    let old_w: Vec<f64> = old_weights.iter().map(|&w| w as f64 / 10000.0).collect();
    let new_w: Vec<f64> = new_weights.iter().map(|&w| w as f64 / 10000.0).collect();
    
    // Convert domain values to f64 for computation
    let values: Vec<f64> = domain_values.iter().map(|&v| v as f64).collect();
    
    // Compute target invariant K_acct with old weights
    let ln_K = old_w[0] * values[0].ln() + 
               old_w[1] * values[1].ln() + 
               old_w[2] * values[2].ln() + 
               old_w[3] * values[3].ln();
    
    // Initialize factors
    let mut h = vec![1.0; 4];
    
    // Fix spot for price continuity if requested
    if fix_spot {
        h[0] = 1.0;
        
        // Also fix buffer (tau) to simplify solving
        h[3] = 1.0;
        
        // Solve for h_T and h_L using Newton's method
        for _ in 0..MAX_ITERATIONS {
            // Current invariant with new weights
            let current_ln_K = new_w[0] * (values[0] * h[0]).ln() +
                              new_w[1] * (values[1] * h[1]).ln() +
                              new_w[2] * (values[2] * h[2]).ln() +
                              new_w[3] * (values[3] * h[3]).ln();
            
            let error = current_ln_K - ln_K;
            if error.abs() < TOLERANCE {
                break;
            }
            
            // Gradient with respect to h_T and h_L
            let dK_dh_T = new_w[1] / h[1];
            let dK_dh_L = new_w[2] / h[2];
            
            // Update h_T and h_L to reduce error
            // Simple gradient descent (could use full Newton)
            let step_size = 0.1;
            h[1] *= (1.0 - step_size * error * dK_dh_T);
            h[2] *= (1.0 - step_size * error * dK_dh_L);
            
            // Ensure factors stay positive
            h[1] = h[1].max(0.01);
            h[2] = h[2].max(0.01);
        }
    } else {
        // General case: solve for all factors
        // This is more complex and typically requires additional constraints
        return Err(FeelsProtocolError::InvalidInput.into());
    }
    
    // Convert to Q64 format
    let h_Q64 = [
        (h[0] * Q64 as f64) as u128,
        (h[1] * Q64 as f64) as u128,
        (h[2] * Q64 as f64) as u128,
        (h[3] * Q64 as f64) as u128,
    ];
    
    // Compute weighted log sum for verification
    let weighted_sum = new_w[0] * h[0].ln() + 
                      new_w[1] * h[1].ln() + 
                      new_w[2] * h[2].ln() + 
                      new_w[3] * h[3].ln();
    let weighted_sum_i64 = (weighted_sum * Q64 as f64) as i64;
    
    Ok((h_Q64, weighted_sum_i64))
}