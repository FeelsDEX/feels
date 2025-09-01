/// Gradient calculations for the market physics fee model.
/// The gradient of the potential field ∇V determines instantaneous fees.
use anchor_lang::prelude::*;
use crate::state::{MarketState, Tick3D};
use crate::state::gradient_cache::Gradient3D;
use crate::logic::market_physics::potential::{FixedPoint, dV_dS, dV_dT, dV_dL};
use crate::error::FeelsProtocolError;

// ============================================================================
// 3D Position Representation
// ============================================================================

/// Position in 3D market space
#[derive(Clone, Copy, Debug, Default)]
pub struct Position3D {
    /// Spot dimension position
    pub s: FixedPoint,
    
    /// Time dimension position
    pub t: FixedPoint,
    
    /// Leverage dimension position
    pub l: FixedPoint,
}

impl Position3D {
    /// Create from market state
    pub fn from_market_state(state: &MarketState) -> Self {
        Self {
            s: FixedPoint::from_scaled(state.S as i128),
            t: FixedPoint::from_scaled(state.T as i128),
            l: FixedPoint::from_scaled(state.L as i128),
        }
    }
    
    /// Create from tick coordinates
    pub fn from_tick(tick: &Tick3D, tick_spacing: i32) -> Result<Self> {
        // Convert tick indices to actual positions
        // This is simplified - real implementation would use proper tick math
        let base = FixedPoint::from_scaled(1_000_100 * FixedPoint::SCALE / 1_000_000); // 1.0001
        
        let s = tick_to_position(tick.rate_tick, base)?;
        let t = tick_to_position(tick.duration_tick as i32, base)?;
        let l = tick_to_position(tick.leverage_tick as i32, base)?;
        
        Ok(Self { s, t, l })
    }
    
    /// Move in spot dimension
    pub fn move_spot(&self, delta: i128) -> Self {
        Self {
            s: FixedPoint::from_scaled(self.s.value + delta),
            t: self.t,
            l: self.l,
        }
    }
    
    /// Move in time dimension
    pub fn move_time(&self, delta: i128) -> Self {
        Self {
            s: self.s,
            t: FixedPoint::from_scaled(self.t.value + delta),
            l: self.l,
        }
    }
    
    /// Move in leverage dimension
    pub fn move_leverage(&self, delta: i128) -> Self {
        Self {
            s: self.s,
            t: self.t,
            l: FixedPoint::from_scaled(self.l.value + delta),
        }
    }
}

/// Position delta in 3D space
#[derive(Clone, Copy, Debug, Default)]
pub struct PositionDelta3D {
    /// Change in spot dimension
    pub dS: FixedPoint,
    
    /// Change in time dimension
    pub dT: FixedPoint,
    
    /// Change in leverage dimension
    pub dL: FixedPoint,
}

impl PositionDelta3D {
    /// Calculate delta between two positions
    pub fn between(from: &Position3D, to: &Position3D) -> Result<Self> {
        Ok(Self {
            dS: to.s.sub(from.s)?,
            dT: to.t.sub(from.t)?,
            dL: to.l.sub(from.l)?,
        })
    }
    
    /// Dot product with gradient
    pub fn dot(&self, gradient: &Gradient3D) -> Result<FixedPoint> {
        let work_s = self.dS.mul(FixedPoint::from_scaled(gradient.dV_dS as i128))?;
        let work_t = self.dT.mul(FixedPoint::from_scaled(gradient.dV_dT as i128))?;
        let work_l = self.dL.mul(FixedPoint::from_scaled(gradient.dV_dL as i128))?;
        
        work_s.add(work_t)?.add(work_l)
    }
}

// ============================================================================
// Gradient Calculation
// ============================================================================

/// Calculate the 3D gradient at a given market state
/// ∇V = (∂V/∂S, ∂V/∂T, ∂V/∂L)
pub fn calculate_gradient_3d(
    state: &MarketState,
    position: &Position3D,
) -> Result<Gradient3D> {
    // For now, use the state's current values
    // In full implementation, would interpolate based on position
    
    let grad_s = dV_dS(state)?;
    let grad_t = dV_dT(state)?;
    let grad_l = dV_dL(state)?;
    
    Ok(Gradient3D {
        dV_dS: grad_s.value as u128,
        dV_dT: grad_t.value as u128,
        dV_dL: grad_l.value as u128,
    })
}

/// Calculate gradient at a specific tick
pub fn calculate_gradient_at_tick(
    state: &MarketState,
    tick: &Tick3D,
) -> Result<Gradient3D> {
    let position = Position3D::from_tick(tick, 1)?;
    calculate_gradient_3d(state, &position)
}

/// Calculate normalized gradient (unit vector)
pub fn calculate_normalized_gradient(gradient: &Gradient3D) -> Result<Gradient3D> {
    let grad_s = FixedPoint::from_scaled(gradient.dV_dS as i128);
    let grad_t = FixedPoint::from_scaled(gradient.dV_dT as i128);
    let grad_l = FixedPoint::from_scaled(gradient.dV_dL as i128);
    
    // Calculate magnitude: |∇V| = sqrt(∂V/∂S² + ∂V/∂T² + ∂V/∂L²)
    let magnitude_sq = grad_s.mul(grad_s)?
        .add(grad_t.mul(grad_t)?)?
        .add(grad_l.mul(grad_l)?)?;
    
    let magnitude = sqrt_fixed(magnitude_sq)?;
    
    require!(magnitude.value > 0, FeelsProtocolError::DivisionByZero);
    
    // Normalize each component
    Ok(Gradient3D {
        dV_dS: grad_s.div(magnitude)?.value as u128,
        dV_dT: grad_t.div(magnitude)?.value as u128,
        dV_dL: grad_l.div(magnitude)?.value as u128,
    })
}

// ============================================================================
// Gradient Field Properties
// ============================================================================

/// Calculate the divergence of the gradient field (Laplacian of V)
/// ∇²V = ∂²V/∂S² + ∂²V/∂T² + ∂²V/∂L²
pub fn calculate_divergence(
    state: &MarketState,
    hessian: &crate::state::gradient_cache::Hessian3x3,
) -> Result<FixedPoint> {
    let d2v_ds2 = FixedPoint::from_scaled(hessian.d2V_dS2 as i128);
    let d2v_dt2 = FixedPoint::from_scaled(hessian.d2V_dT2 as i128);
    let d2v_dl2 = FixedPoint::from_scaled(hessian.d2V_dL2 as i128);
    
    d2v_ds2.add(d2v_dt2)?.add(d2v_dl2)
}

/// Calculate the curl of the gradient field (should be zero for conservative field)
/// ∇ × ∇V = 0 (always, since gradient fields are conservative)
pub fn verify_conservative_field(
    hessian: &crate::state::gradient_cache::Hessian3x3,
) -> Result<bool> {
    // Check if mixed partials are equal (Schwarz's theorem)
    // For a conservative field: ∂²V/∂x∂y = ∂²V/∂y∂x
    
    // In our case, cross-derivatives should be symmetric
    // Since we have a diagonal Hessian for our potential, they should all be zero
    let tolerance = FixedPoint::SCALE / 1_000_000; // 1e-6 tolerance
    
    Ok(
        hessian.d2V_dSdT.abs() < tolerance as i128 &&
        hessian.d2V_dSdL.abs() < tolerance as i128 &&
        hessian.d2V_dTdL.abs() < tolerance as i128
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert tick index to position using exponential spacing
fn tick_to_position(tick: i32, base: FixedPoint) -> Result<FixedPoint> {
    if tick == 0 {
        return Ok(FixedPoint::ONE);
    }
    
    // position = base^tick
    // For small ticks, use approximation: base^tick ≈ 1 + tick * ln(base)
    if tick.abs() < 100 {
        let ln_base = FixedPoint::from_scaled(100 * FixedPoint::SCALE / 1_000_000); // ln(1.0001) ≈ 0.0001
        let delta = ln_base.mul(FixedPoint::from_int(tick as i64))?;
        return FixedPoint::ONE.add(delta);
    }
    
    // For larger ticks, use iterative multiplication
    let mut result = FixedPoint::ONE;
    let abs_tick = tick.abs() as u32;
    
    for _ in 0..abs_tick.min(1000) {
        result = result.mul(base)?;
    }
    
    if tick < 0 {
        FixedPoint::ONE.div(result)
    } else {
        Ok(result)
    }
}

/// Square root of fixed-point number
fn sqrt_fixed(x: FixedPoint) -> Result<FixedPoint> {
    require!(x.value >= 0, FeelsProtocolError::InvalidInput);
    
    if x.value == 0 {
        return Ok(FixedPoint::ZERO);
    }
    
    // Newton's method: y_{n+1} = (y_n + x/y_n) / 2
    let mut y = x;
    
    for _ in 0..10 {
        let y_new = y.add(x.div(y)?)?.div(FixedPoint::from_int(2))?;
        
        // Check convergence
        if (y_new.value - y.value).abs() < FixedPoint::SCALE / 1_000_000 {
            return Ok(y_new);
        }
        
        y = y_new;
    }
    
    Ok(y)
}

// ============================================================================
// Gradient Interpolation
// ============================================================================

/// Interpolate gradient between tick boundaries
pub fn interpolate_gradient(
    lower_gradient: &Gradient3D,
    upper_gradient: &Gradient3D,
    position_ratio: FixedPoint, // 0 = lower, 1 = upper
) -> Result<Gradient3D> {
    let one_minus_ratio = FixedPoint::ONE.sub(position_ratio)?;
    
    // Linear interpolation: g = (1-t) * g_lower + t * g_upper
    let interp_s = one_minus_ratio
        .mul(FixedPoint::from_scaled(lower_gradient.dV_dS as i128))?
        .add(position_ratio.mul(FixedPoint::from_scaled(upper_gradient.dV_dS as i128))?)?;
    
    let interp_t = one_minus_ratio
        .mul(FixedPoint::from_scaled(lower_gradient.dV_dT as i128))?
        .add(position_ratio.mul(FixedPoint::from_scaled(upper_gradient.dV_dT as i128))?)?;
    
    let interp_l = one_minus_ratio
        .mul(FixedPoint::from_scaled(lower_gradient.dV_dL as i128))?
        .add(position_ratio.mul(FixedPoint::from_scaled(upper_gradient.dV_dL as i128))?)?;
    
    Ok(Gradient3D {
        dV_dS: interp_s.value as u128,
        dV_dT: interp_t.value as u128,
        dV_dL: interp_l.value as u128,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gradient_calculation() {
        let mut state = MarketState::default();
        state.S = 1u128 << 64; // 1.0
        state.T = 1u128 << 64; // 1.0
        state.L = 1u128 << 64; // 1.0
        state.w_s = 3333;
        state.w_t = 3333;
        state.w_l = 3334;
        state.w_tau = 0;
        
        let position = Position3D::from_market_state(&state);
        let gradient = calculate_gradient_3d(&state, &position).unwrap();
        
        // At equilibrium (all values = 1), gradients should be proportional to weights
        assert!(gradient.dV_dS > 0);
        assert!(gradient.dV_dT > 0);
        assert!(gradient.dV_dL > 0);
    }
    
    #[test]
    fn test_position_delta() {
        let from = Position3D {
            s: FixedPoint::from_int(100),
            t: FixedPoint::from_int(50),
            l: FixedPoint::from_int(10),
        };
        
        let to = Position3D {
            s: FixedPoint::from_int(110),
            t: FixedPoint::from_int(45),
            l: FixedPoint::from_int(12),
        };
        
        let delta = PositionDelta3D::between(&from, &to).unwrap();
        
        assert_eq!(delta.dS.to_u64(), 10);
        assert_eq!(delta.dT.to_u64(), 0); // Would be -5 but to_u64 clips to 0
        assert_eq!(delta.dL.to_u64(), 2);
    }
}