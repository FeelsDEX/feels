/// Work calculation for the market physics fee model.
/// Work done moving through position space determines fees (positive work) or rebates (negative work).
use anchor_lang::prelude::*;
use crate::state::{BufferAccount, gradient_cache::Gradient3D};
use crate::logic::market_physics::gradient::{PositionDelta3D, Position3D};
use crate::logic::market_physics::potential::FixedPoint;
use crate::error::FeelsProtocolError;

// ============================================================================
// Constants
// ============================================================================

/// Basis points denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Maximum rebate caps
pub const DEFAULT_TX_CAP: u64 = u64::MAX;
pub const DEFAULT_EPOCH_CAP: u64 = u64::MAX;

// ============================================================================
// Work Calculation
// ============================================================================

/// Calculate work done moving through position space
/// W = ∇V · ΔP (dot product of gradient and position change)
pub fn calculate_work(
    gradient: &Gradient3D,
    delta: &PositionDelta3D,
) -> Result<FixedPoint> {
    delta.dot(gradient)
}

/// Calculate work along a path with multiple segments
pub fn calculate_path_work(
    segments: &[PathSegment],
) -> Result<FixedPoint> {
    let mut total_work = FixedPoint::ZERO;
    
    for segment in segments {
        let segment_work = calculate_work(&segment.gradient, &segment.delta)?;
        total_work = total_work.add(segment_work)?;
    }
    
    Ok(total_work)
}

/// Path segment with gradient and position change
#[derive(Clone, Debug)]
pub struct PathSegment {
    /// Gradient at this segment
    pub gradient: Gradient3D,
    
    /// Position change in this segment
    pub delta: PositionDelta3D,
}

// ============================================================================
// Fee and Rebate Calculation
// ============================================================================

/// Calculate fee and rebate from work
pub fn calculate_fee_and_rebate(
    work: FixedPoint,
    pi_p: FixedPoint,      // Price map Π(P)
    eta: u32,              // Rebate participation rate
    caps: &RebateCaps,
) -> Result<(u64, u64)> {
    // Separate positive and negative work
    let (f_collected, r_star) = if work.is_positive() {
        // Positive work: fee collection
        let fee = work.mul(pi_p)?.to_u64();
        (fee, 0)
    } else {
        // Negative work: potential rebate
        let rebate_star = work.abs()
            .mul(pi_p)?
            .mul(FixedPoint::from_scaled((eta as i128 * FixedPoint::SCALE) / BPS_DENOMINATOR as i128))?
            .to_u64();
        (0, rebate_star)
    };
    
    // Apply rebate caps
    let r = apply_rebate_caps(r_star, caps);
    
    Ok((f_collected, r))
}

/// Rebate caps structure
#[derive(Clone, Debug)]
pub struct RebateCaps {
    /// Per-transaction cap
    pub tx_cap: u64,
    
    /// Per-epoch cap
    pub epoch_cap: u64,
    
    /// Available tau in buffer
    pub tau_available: u64,
    
    /// Price improvement limit
    pub price_improvement: u64,
}

impl Default for RebateCaps {
    fn default() -> Self {
        Self {
            tx_cap: DEFAULT_TX_CAP,
            epoch_cap: DEFAULT_EPOCH_CAP,
            tau_available: u64::MAX,
            price_improvement: u64::MAX,
        }
    }
}

/// Apply all rebate caps
fn apply_rebate_caps(r_star: u64, caps: &RebateCaps) -> u64 {
    r_star
        .min(caps.tx_cap)
        .min(caps.epoch_cap)
        .min(caps.tau_available)
        .min(caps.price_improvement)
}

// ============================================================================
// Price Map Calculation
// ============================================================================

/// Calculate price map Π(P) that converts work to token value
/// For now, returns the geometric mean of position values
pub fn calculate_price_map(position: &Position3D) -> Result<FixedPoint> {
    // Π(P) = (S * T * L)^(1/3)
    // Using arithmetic mean approximation for simplicity
    let sum = position.s
        .add(position.t)?
        .add(position.l)?;
    
    sum.div(FixedPoint::from_int(3))
}

/// Calculate price improvement from rebate
pub fn calculate_price_improvement(
    rebate: u64,
    trade_amount: u64,
) -> Result<u64> {
    if trade_amount == 0 {
        return Ok(0);
    }
    
    // Price improvement = rebate / trade_amount (in basis points)
    let improvement_bps = (rebate as u128)
        .checked_mul(BPS_DENOMINATOR as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(trade_amount as u128)
        .ok_or(FeelsProtocolError::DivisionByZero)?;
    
    Ok(improvement_bps.min(BPS_DENOMINATOR as u128) as u64)
}

// ============================================================================
// Work Integration
// ============================================================================

/// Calculate work using linear approximation within a cell
/// W = g · Δp (first-order approximation)
pub fn calculate_linear_work(
    gradient: &Gradient3D,
    delta: &PositionDelta3D,
) -> Result<FixedPoint> {
    calculate_work(gradient, delta)
}

/// Calculate work using quadratic approximation
/// W = g · Δp + ½ Δp · H · Δp
pub fn calculate_quadratic_work(
    gradient: &Gradient3D,
    hessian: &crate::state::gradient_cache::Hessian3x3,
    delta: &PositionDelta3D,
) -> Result<FixedPoint> {
    // Linear term: g · Δp
    let linear_work = calculate_linear_work(gradient, delta)?;
    
    // Quadratic term: ½ Δp · H · Δp
    let quadratic_work = calculate_quadratic_term(hessian, delta)?
        .div(FixedPoint::from_int(2))?;
    
    linear_work.add(quadratic_work)
}

/// Calculate quadratic form: Δp · H · Δp
fn calculate_quadratic_term(
    hessian: &crate::state::gradient_cache::Hessian3x3,
    delta: &PositionDelta3D,
) -> Result<FixedPoint> {
    // For diagonal Hessian: Δp · H · Δp = ΔS² * H_SS + ΔT² * H_TT + ΔL² * H_LL
    let ds_squared = delta.dS.mul(delta.dS)?;
    let dt_squared = delta.dT.mul(delta.dT)?;
    let dl_squared = delta.dL.mul(delta.dL)?;
    
    let h_ss = FixedPoint::from_scaled(hessian.d2V_dS2 as i128);
    let h_tt = FixedPoint::from_scaled(hessian.d2V_dT2 as i128);
    let h_ll = FixedPoint::from_scaled(hessian.d2V_dL2 as i128);
    
    let term_s = ds_squared.mul(h_ss)?;
    let term_t = dt_squared.mul(h_tt)?;
    let term_l = dl_squared.mul(h_ll)?;
    
    // Add cross terms if non-zero
    let mut result = term_s.add(term_t)?.add(term_l)?;
    
    if hessian.d2V_dSdT != 0 {
        let cross_st = delta.dS
            .mul(delta.dT)?
            .mul(FixedPoint::from_scaled(hessian.d2V_dSdT))?
            .mul(FixedPoint::from_int(2))?; // Factor of 2 for cross terms
        result = result.add(cross_st)?;
    }
    
    if hessian.d2V_dSdL != 0 {
        let cross_sl = delta.dS
            .mul(delta.dL)?
            .mul(FixedPoint::from_scaled(hessian.d2V_dSdL))?
            .mul(FixedPoint::from_int(2))?;
        result = result.add(cross_sl)?;
    }
    
    if hessian.d2V_dTdL != 0 {
        let cross_tl = delta.dT
            .mul(delta.dL)?
            .mul(FixedPoint::from_scaled(hessian.d2V_dTdL))?
            .mul(FixedPoint::from_int(2))?;
        result = result.add(cross_tl)?;
    }
    
    Ok(result)
}

// ============================================================================
// Fee Booking
// ============================================================================

/// Book fees and rebates to appropriate accounts
pub fn book_fees_and_rebates(
    f_collected: u64,
    r_paid: u64,
    buffer: &mut BufferAccount,
    dimension: TradeDimension,
) -> Result<()> {
    // Collect fees into buffer
    if f_collected > 0 {
        buffer.collect_fees(f_collected as u128)?;
    }
    
    // Pay rebates from buffer
    if r_paid > 0 {
        let actual_rebate = buffer.pay_rebate(r_paid, Clock::get()?.unix_timestamp)?;
        require!(
            actual_rebate == r_paid,
            FeelsProtocolError::InsufficientBuffer
        );
    }
    
    // Update fee shares for EWMA tracking
    match dimension {
        TradeDimension::Spot => {
            buffer.update_fee_shares(f_collected, 0, 0, Clock::get()?.unix_timestamp)?;
        }
        TradeDimension::Time => {
            buffer.update_fee_shares(0, f_collected, 0, Clock::get()?.unix_timestamp)?;
        }
        TradeDimension::Leverage => {
            buffer.update_fee_shares(0, 0, f_collected, Clock::get()?.unix_timestamp)?;
        }
        TradeDimension::Mixed => {
            // Distribute equally for mixed trades
            let per_dimension = f_collected / 3;
            buffer.update_fee_shares(
                per_dimension,
                per_dimension,
                f_collected - 2 * per_dimension,
                Clock::get()?.unix_timestamp
            )?;
        }
    }
    
    Ok(())
}

/// Trade dimension for fee attribution
#[derive(Clone, Copy, Debug)]
pub enum TradeDimension {
    Spot,
    Time,
    Leverage,
    Mixed,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_positive_work_creates_fee() {
        let gradient = Gradient3D {
            dV_dS: (1u128 << 63), // 0.5 in Q64
            dV_dT: 0,
            dV_dL: 0,
        };
        
        let delta = PositionDelta3D {
            dS: FixedPoint::from_int(10),
            dT: FixedPoint::ZERO,
            dL: FixedPoint::ZERO,
        };
        
        let work = calculate_work(&gradient, &delta).unwrap();
        assert!(work.is_positive());
        
        let pi_p = FixedPoint::from_int(100); // $100 price
        let caps = RebateCaps::default();
        
        let (fee, rebate) = calculate_fee_and_rebate(work, pi_p, 5000, &caps).unwrap();
        
        assert!(fee > 0);
        assert_eq!(rebate, 0);
    }
    
    #[test]
    fn test_negative_work_creates_rebate() {
        let gradient = Gradient3D {
            dV_dS: (1u128 << 63), // 0.5 in Q64
            dV_dT: 0,
            dV_dL: 0,
        };
        
        let delta = PositionDelta3D {
            dS: FixedPoint::from_int(-10), // Moving opposite to gradient
            dT: FixedPoint::ZERO,
            dL: FixedPoint::ZERO,
        };
        
        let work = calculate_work(&gradient, &delta).unwrap();
        assert!(work.is_negative());
        
        let pi_p = FixedPoint::from_int(100);
        let caps = RebateCaps::default();
        
        let (fee, rebate) = calculate_fee_and_rebate(work, pi_p, 5000, &caps).unwrap();
        
        assert_eq!(fee, 0);
        assert!(rebate > 0);
    }
    
    #[test]
    fn test_rebate_caps() {
        let r_star = 1000;
        let caps = RebateCaps {
            tx_cap: 500,
            epoch_cap: 800,
            tau_available: 600,
            price_improvement: 1000,
        };
        
        let capped = apply_rebate_caps(r_star, &caps);
        assert_eq!(capped, 500); // Limited by tx_cap
    }
}