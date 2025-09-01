/// Potential field calculations for the market physics model.
/// The potential V = -ln(K_trade) creates the energy landscape that determines fees.
use anchor_lang::prelude::*;
use crate::state::{MarketState, DomainWeights};
use crate::error::FeelsProtocolError;

// ============================================================================
// Fixed-Point Math Types
// ============================================================================

/// Fixed-point number with Q64 precision
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedPoint {
    /// Value scaled by 2^64
    pub value: i128,
}

impl FixedPoint {
    pub const SCALE: i128 = 1 << 64;
    pub const ZERO: Self = Self { value: 0 };
    pub const ONE: Self = Self { value: Self::SCALE };
    
    /// Create from raw scaled value
    pub fn from_scaled(value: i128) -> Self {
        Self { value }
    }
    
    /// Create from integer
    pub fn from_int(value: i64) -> Self {
        Self {
            value: (value as i128) * Self::SCALE,
        }
    }
    
    /// Add two fixed-point numbers
    pub fn add(&self, other: Self) -> Result<Self> {
        Ok(Self {
            value: self.value
                .checked_add(other.value)
                .ok_or(FeelsProtocolError::MathOverflow)?,
        })
    }
    
    /// Subtract two fixed-point numbers
    pub fn sub(&self, other: Self) -> Result<Self> {
        Ok(Self {
            value: self.value
                .checked_sub(other.value)
                .ok_or(FeelsProtocolError::MathOverflow)?,
        })
    }
    
    /// Multiply two fixed-point numbers
    pub fn mul(&self, other: Self) -> Result<Self> {
        let product = self.value
            .checked_mul(other.value)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(Self {
            value: product
                .checked_div(Self::SCALE)
                .ok_or(FeelsProtocolError::DivisionByZero)?,
        })
    }
    
    /// Divide two fixed-point numbers
    pub fn div(&self, other: Self) -> Result<Self> {
        require!(other.value != 0, FeelsProtocolError::DivisionByZero);
        
        let scaled = self.value
            .checked_mul(Self::SCALE)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(Self {
            value: scaled
                .checked_div(other.value)
                .ok_or(FeelsProtocolError::DivisionByZero)?,
        })
    }
    
    /// Negate the value
    pub fn neg(&self) -> Result<Self> {
        Ok(Self {
            value: self.value
                .checked_neg()
                .ok_or(FeelsProtocolError::MathOverflow)?,
        })
    }
    
    /// Convert to u64 (losing precision)
    pub fn to_u64(&self) -> u64 {
        (self.value / Self::SCALE).max(0) as u64
    }
    
    /// Check if positive
    pub fn is_positive(&self) -> bool {
        self.value > 0
    }
    
    /// Check if negative
    pub fn is_negative(&self) -> bool {
        self.value < 0
    }
    
    /// Absolute value
    pub fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
        }
    }
}

// ============================================================================
// Potential Field Calculation
// ============================================================================

/// Calculate the potential field V = -ln(K_trade)
/// where K_trade = S^ŵ_s * T^ŵ_t * L^ŵ_l
pub fn calculate_potential(state: &MarketState) -> Result<FixedPoint> {
    // Get normalized hat weights
    let (w_hat_s, w_hat_t, w_hat_l) = state.get_weights().get_hat_weights();
    
    // V = -ŵ_s * ln(S) - ŵ_t * ln(T) - ŵ_l * ln(L)
    let v_s = calculate_weighted_log_term(state.S, w_hat_s)?;
    let v_t = calculate_weighted_log_term(state.T, w_hat_t)?;
    let v_l = calculate_weighted_log_term(state.L, w_hat_l)?;
    
    // Sum and negate
    v_s.add(v_t)?
        .add(v_l)?
        .neg()
}

/// Calculate a single term: -w * ln(x)
fn calculate_weighted_log_term(x: u128, weight: u32) -> Result<FixedPoint> {
    // Skip if weight is zero
    if weight == 0 || x == 0 {
        return Ok(FixedPoint::ZERO);
    }
    
    // Calculate ln(x)
    let ln_x = ln_fixed(x)?;
    
    // Apply weight: w * ln(x)
    let weighted = ln_x.mul(FixedPoint::from_scaled(
        (weight as i128 * FixedPoint::SCALE) / DomainWeights::SCALE as i128
    ))?;
    
    Ok(weighted)
}

/// Calculate natural logarithm of a u128 value
/// Returns ln(x) as a FixedPoint
pub fn ln_fixed(x: u128) -> Result<FixedPoint> {
    require!(x > 0, FeelsProtocolError::InvalidInput);
    
    const Q64: u128 = 1 << 64;
    
    // Handle x = Q64 (ln(1) = 0)
    if x == Q64 {
        return Ok(FixedPoint::ZERO);
    }
    
    // For x close to Q64, use Taylor series: ln(x) ≈ (x-1) - (x-1)²/2 + (x-1)³/3
    if x > Q64 / 2 && x < Q64 * 2 {
        let x_minus_1 = if x > Q64 {
            FixedPoint::from_scaled((x - Q64) as i128)
        } else {
            FixedPoint::from_scaled(-((Q64 - x) as i128))
        };
        
        // First term: (x-1)
        let mut result = x_minus_1;
        
        // Second term: -(x-1)²/2
        let x_minus_1_sq = x_minus_1.mul(x_minus_1)?;
        result = result.sub(x_minus_1_sq.div(FixedPoint::from_int(2))?)?;
        
        // Third term: +(x-1)³/3 (only if x is very close to 1)
        if x_minus_1.abs().value < FixedPoint::SCALE / 10 {
            let x_minus_1_cubed = x_minus_1_sq.mul(x_minus_1)?;
            result = result.add(x_minus_1_cubed.div(FixedPoint::from_int(3))?)?;
        }
        
        return Ok(result);
    }
    
    // For larger values, use change of base: ln(x) = ln(x/Q64) + ln(Q64)
    // Since Q64 = 2^64, ln(Q64) = 64 * ln(2) ≈ 64 * 0.693147
    if x > Q64 {
        let ratio = x / Q64;
        let ln_ratio = ln_fixed(ratio)?;
        let ln_q64 = FixedPoint::from_scaled(44_361_286_125_870_736); // 64 * ln(2) in Q64
        return ln_ratio.add(ln_q64);
    }
    
    // For smaller values, use ln(x) = -ln(1/x)
    let inv_x = (Q64 * Q64) / x;
    let ln_inv = ln_fixed(inv_x)?;
    ln_inv.neg()
}

/// Calculate exponential e^x
/// Returns e^x as a FixedPoint
pub fn exp_fixed(x: FixedPoint) -> Result<u128> {
    // Handle x = 0 (e^0 = 1)
    if x.value == 0 {
        return Ok(1u128 << 64);
    }
    
    // Use Taylor series: e^x = 1 + x + x²/2! + x³/3! + ...
    let mut result = FixedPoint::ONE;
    
    // Add x
    result = result.add(x)?;
    
    // Add x²/2
    let x_squared = x.mul(x)?;
    result = result.add(x_squared.div(FixedPoint::from_int(2))?)?;
    
    // Add x³/6 (only for small x)
    if x.abs().value < FixedPoint::SCALE / 10 {
        let x_cubed = x_squared.mul(x)?;
        result = result.add(x_cubed.div(FixedPoint::from_int(6))?)?;
    }
    
    // Result must be positive
    require!(result.value > 0, FeelsProtocolError::InvalidState);
    
    Ok(result.value as u128)
}

// ============================================================================
// Normalized Weight Calculation
// ============================================================================

/// Normalize a weight excluding tau participation
pub fn normalize_weight(weight: u32, w_tau: u32) -> Result<FixedPoint> {
    let total_trade_weight = (DomainWeights::SCALE - w_tau) as i128;
    
    require!(total_trade_weight > 0, FeelsProtocolError::InvalidWeights);
    
    Ok(FixedPoint::from_scaled(
        (weight as i128 * FixedPoint::SCALE) / total_trade_weight
    ))
}

// ============================================================================
// Potential Derivatives
// ============================================================================

/// Calculate the derivative of potential with respect to S
/// ∂V/∂S = -ŵ_s / S
pub fn dV_dS(state: &MarketState) -> Result<FixedPoint> {
    require!(state.S > 0, FeelsProtocolError::DivisionByZero);
    
    let (w_hat_s, _, _) = state.get_weights().get_hat_weights();
    let w_hat_s_fixed = normalize_weight(w_hat_s, state.w_tau)?;
    
    // -ŵ_s / S
    w_hat_s_fixed
        .div(FixedPoint::from_scaled(state.S as i128))?
        .neg()
}

/// Calculate the derivative of potential with respect to T
/// ∂V/∂T = -ŵ_t / T
pub fn dV_dT(state: &MarketState) -> Result<FixedPoint> {
    require!(state.T > 0, FeelsProtocolError::DivisionByZero);
    
    let (_, w_hat_t, _) = state.get_weights().get_hat_weights();
    let w_hat_t_fixed = normalize_weight(w_hat_t, state.w_tau)?;
    
    // -ŵ_t / T
    w_hat_t_fixed
        .div(FixedPoint::from_scaled(state.T as i128))?
        .neg()
}

/// Calculate the derivative of potential with respect to L
/// ∂V/∂L = -ŵ_l / L
pub fn dV_dL(state: &MarketState) -> Result<FixedPoint> {
    require!(state.L > 0, FeelsProtocolError::DivisionByZero);
    
    let (_, _, w_hat_l) = state.get_weights().get_hat_weights();
    let w_hat_l_fixed = normalize_weight(w_hat_l, state.w_tau)?;
    
    // -ŵ_l / L
    w_hat_l_fixed
        .div(FixedPoint::from_scaled(state.L as i128))?
        .neg()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_point_arithmetic() {
        let a = FixedPoint::from_int(2);
        let b = FixedPoint::from_int(3);
        
        let sum = a.add(b).unwrap();
        assert_eq!(sum.to_u64(), 5);
        
        let product = a.mul(b).unwrap();
        assert_eq!(product.to_u64(), 6);
        
        let quotient = b.div(a).unwrap();
        assert_eq!(quotient.value, (3 * FixedPoint::SCALE) / 2);
    }
    
    #[test]
    fn test_ln_fixed() {
        // ln(1) = 0
        let ln_one = ln_fixed(1u128 << 64).unwrap();
        assert_eq!(ln_one.value, 0);
        
        // ln(e) ≈ 1
        let e_approx = (2.718281828 * (1u64 << 32) as f64) as u128 * (1u32 << 32) as u128;
        let ln_e = ln_fixed(e_approx).unwrap();
        let one_q64 = FixedPoint::SCALE;
        assert!((ln_e.value - one_q64).abs() < one_q64 / 100); // Within 1%
    }
}