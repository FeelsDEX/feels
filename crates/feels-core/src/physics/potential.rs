//! # Potential Function Calculations
//! 
//! Implements the thermodynamic potential function:
//! V = -ŵₛ ln(S) - ŵₜ ln(T) - ŵₗ ln(L)

use crate::errors::CoreResult;
use crate::constants::Q64;
use crate::math::safe_math::{safe_add_i128, safe_sub_u128, safe_mul_u128, safe_div_u128, safe_mul_i128, safe_div_i128};
use crate::types::{Position3D, DomainWeights, Gradient3D};

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Market field representing the thermodynamic state
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
#[allow(non_snake_case)]
pub struct MarketField {
    /// Spot dimension scalar (Q64)
    pub S: u128,
    /// Time dimension scalar (Q64)
    pub T: u128,
    /// Leverage dimension scalar (Q64)
    pub L: u128,
    /// Domain weights
    pub weights: DomainWeights,
    /// Risk scalers (basis points)
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
}

/// Calculate potential at a position (linearized version)
/// V = S·σₛ + T·σₜ + L·σₗ
pub fn calculate_potential_linear(
    position: &Position3D,
    field: &MarketField,
) -> CoreResult<i128> {
    // V_s = position.S * field.S / Q64
    let v_s = safe_div_u128(
        safe_mul_u128(position.S, field.S)?,
        Q64
    )? as i128;
    
    // V_t = position.T * field.T / Q64
    let v_t = safe_div_u128(
        safe_mul_u128(position.T, field.T)?,
        Q64
    )? as i128;
    
    // V_l = position.L * field.L / Q64
    let v_l = safe_div_u128(
        safe_mul_u128(position.L, field.L)?,
        Q64
    )? as i128;
    
    // Total potential
    safe_add_i128(v_s, safe_add_i128(v_t, v_l)?)
}

/// Calculate gradient of potential function
pub fn calculate_gradient(
    start: &Position3D,
    end: &Position3D,
    field: &MarketField,
) -> CoreResult<Gradient3D> {
    // Finite differences
    let ds = if end.S > start.S {
        safe_sub_u128(end.S, start.S)? as i128
    } else {
        -(safe_sub_u128(start.S, end.S)? as i128)
    };
    
    let dt = if end.T > start.T {
        safe_sub_u128(end.T, start.T)? as i128
    } else {
        -(safe_sub_u128(start.T, end.T)? as i128)
    };
    
    let dl = if end.L > start.L {
        safe_sub_u128(end.L, start.L)? as i128
    } else {
        -(safe_sub_u128(start.L, end.L)? as i128)
    };
    
    // Apply field scalars
    let grad_s = safe_div_i128(
        safe_mul_i128(ds, field.S as i128)?,
        Q64 as i128
    )?;
    
    let grad_t = safe_div_i128(
        safe_mul_i128(dt, field.T as i128)?,
        Q64 as i128
    )?;
    
    let grad_l = safe_div_i128(
        safe_mul_i128(dl, field.L as i128)?,
        Q64 as i128
    )?;
    
    Ok(Gradient3D { grad_s, grad_t, grad_l })
}


#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use crate::math::fixed_point::ln_q64;
    
    /// Calculate potential using full logarithmic formula
    /// V = -ŵₛ ln(S) - ŵₜ ln(T) - ŵₗ ln(L)
    pub fn calculate_potential_logarithmic(
        position: &Position3D,
        field: &MarketField,
    ) -> CoreResult<i128> {
        // Get normalized weights
        let (w_hat_s, w_hat_t, w_hat_l) = field.weights.get_hat_weights();
        
        // Calculate logarithms
        let ln_s = ln_q64(position.S)?;
        let ln_t = ln_q64(position.T)?;
        let ln_l = ln_q64(position.L)?;
        
        // Apply weights: -w * ln(x)
        let v_s = -safe_div_i128(
            safe_mul_i128(w_hat_s as i128, ln_s)?,
            10000 // weights are in basis points
        )?;
        
        let v_t = -safe_div_i128(
            safe_mul_i128(w_hat_t as i128, ln_t)?,
            10000
        )?;
        
        let v_l = -safe_div_i128(
            safe_mul_i128(w_hat_l as i128, ln_l)?,
            10000
        )?;
        
        // Sum components
        safe_add_i128(v_s, safe_add_i128(v_t, v_l)?)
    }
    
    /// Calculate Hessian matrix (second derivatives)
    pub fn calculate_hessian(
        position: &Position3D,
        field: &MarketField,
    ) -> CoreResult<Hessian3D> {
        // For logarithmic potential, Hessian diagonal elements are:
        // ∂²V/∂x² = ŵ/x²
        let (w_hat_s, w_hat_t, w_hat_l) = field.weights.get_hat_weights();
        
        // Calculate diagonal elements
        let d2s = safe_div_u128(
            safe_mul_u128(w_hat_s as u128, Q64)?,
            safe_div_u128(
                safe_mul_u128(position.S, position.S)?,
                Q64
            )?
        )? as i128;
        
        let d2t = safe_div_u128(
            safe_mul_u128(w_hat_t as u128, Q64)?,
            safe_div_u128(
                safe_mul_u128(position.T, position.T)?,
                Q64
            )?
        )? as i128;
        
        let d2l = safe_div_u128(
            safe_mul_u128(w_hat_l as u128, Q64)?,
            safe_div_u128(
                safe_mul_u128(position.L, position.L)?,
                Q64
            )?
        )? as i128;
        
        Ok(Hessian3D {
            d2s,
            d2t,
            d2l,
            // Cross derivatives are 0 for separable potential
            dst: 0,
            dsl: 0,
            dtl: 0,
        })
    }
    
    /// Hessian matrix for second derivatives
    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
    pub struct Hessian3D {
        pub d2s: i128,
        pub d2t: i128,
        pub d2l: i128,
        pub dst: i128,
        pub dsl: i128,
        pub dtl: i128,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_linear_potential() {
        let position = Position3D::new(100 * Q64, 100 * Q64, 100 * Q64);
        let field = MarketField {
            s: Q64,
            t: Q64,
            l: Q64,
            weights: DomainWeights { w_s: 3333, w_t: 3333, w_l: 3334, w_tau: 0 },
            sigma_price: 100,
            sigma_rate: 50,
            sigma_leverage: 200,
        };
        
        let potential = calculate_potential_linear(&position, &field).unwrap();
        assert_eq!(potential, 300); // 100 + 100 + 100
    }
}