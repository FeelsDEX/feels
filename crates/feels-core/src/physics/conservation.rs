//! # Conservation Laws
//! 
//! Implements the fundamental conservation identity:
//! Σ wᵢ ln(gᵢ) = 0

use crate::errors::{CoreResult, FeelsCoreError};
use crate::constants::BPS_DENOMINATOR;
// safe_add_i64 is defined locally in this module

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Conservation verification data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "anchor", derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize))]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct ConservationData {
    /// Domain weights at snapshot
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    /// Logarithms of growth factors (scaled by 1e6)
    pub ln_g_s: i64,
    pub ln_g_t: i64,
    pub ln_g_l: i64,
    /// Buffer adjustment (computed)
    pub ln_g_tau: i64,
}

impl ConservationData {
    /// Verify conservation identity holds
    pub fn verify_conservation(&self) -> CoreResult<bool> {
        // Check weights sum to 100%
        let weight_sum = self.w_s + self.w_t + self.w_l + self.w_tau;
        if weight_sum != BPS_DENOMINATOR as u32 {
            return Err(FeelsCoreError::InvalidWeightSum);
        }
        
        // Calculate weighted sum: Σ wᵢ ln(gᵢ)
        let sum = calculate_weighted_log_sum(
            &[self.w_s, self.w_t, self.w_l, self.w_tau],
            &[self.ln_g_s, self.ln_g_t, self.ln_g_l, self.ln_g_tau],
        )?;
        
        // Allow small numerical error (1e-6 in scaled units)
        Ok(sum.abs() < 1)
    }
    
    /// Calculate required buffer adjustment
    pub fn calculate_buffer_adjustment(&self) -> CoreResult<i64> {
        if self.w_tau == 0 {
            return Err(FeelsCoreError::DivisionByZero);
        }
        
        // ln(g_tau) = -(w_s*ln(g_s) + w_t*ln(g_t) + w_l*ln(g_l)) / w_tau
        let domain_sum = calculate_weighted_log_sum(
            &[self.w_s, self.w_t, self.w_l],
            &[self.ln_g_s, self.ln_g_t, self.ln_g_l],
        )?;
        
        Ok(-domain_sum * BPS_DENOMINATOR as i64 / self.w_tau as i64)
    }
}

/// Calculate weighted logarithm sum
fn calculate_weighted_log_sum(weights: &[u32], ln_values: &[i64]) -> CoreResult<i64> {
    let mut sum = 0i64;
    
    for (w, ln_g) in weights.iter().zip(ln_values.iter()) {
        let weighted = (*w as i64) * (*ln_g);
        sum = safe_add_i64(sum, weighted)?;
    }
    
    Ok(sum)
}

/// Growth factors for rebasing
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct GrowthFactors {
    pub g_s: f64,
    pub g_t: f64,
    pub g_l: f64,
    pub g_tau: f64,
}

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    
    /// Calculate exact growth factors from rates
    pub fn calculate_growth_factors(
        r_s: f64,  // Spot rate
        r_t: f64,  // Time rate
        r_l: f64,  // Leverage rate
        dt: f64,   // Time delta in years
        weights: &ConservationWeights,
    ) -> CoreResult<GrowthFactors> {
        // Calculate domain growth factors
        let g_s = (r_s * dt).exp();
        let g_t = (r_t * dt).exp();
        let g_l = (r_l * dt).exp();
        
        // Calculate buffer factor to satisfy conservation
        let ln_g_tau = if weights.w_tau > 0.0 {
            -(weights.w_s * g_s.ln() + 
              weights.w_t * g_t.ln() + 
              weights.w_l * g_l.ln()) / weights.w_tau
        } else {
            0.0
        };
        
        let g_tau = ln_g_tau.exp();
        
        Ok(GrowthFactors { g_s, g_t, g_l, g_tau })
    }
    
    /// Conservation weights (normalized to 1.0)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConservationWeights {
        pub w_s: f64,
        pub w_t: f64,
        pub w_l: f64,
        pub w_tau: f64,
    }
    
    impl ConservationWeights {
        pub fn from_bps(w_s: u32, w_t: u32, w_l: u32, w_tau: u32) -> CoreResult<Self> {
            let sum = w_s + w_t + w_l + w_tau;
            if sum != BPS_DENOMINATOR as u32 {
                return Err(FeelsCoreError::InvalidWeightSum);
            }
            
            Ok(Self {
                w_s: w_s as f64 / BPS_DENOMINATOR as f64,
                w_t: w_t as f64 / BPS_DENOMINATOR as f64,
                w_l: w_l as f64 / BPS_DENOMINATOR as f64,
                w_tau: w_tau as f64 / BPS_DENOMINATOR as f64,
            })
        }
        
        pub fn verify(&self) -> bool {
            (self.w_s + self.w_t + self.w_l + self.w_tau - 1.0).abs() < 1e-9
        }
    }
}

// Safe i64 arithmetic helpers
fn safe_add_i64(a: i64, b: i64) -> CoreResult<i64> {
    a.checked_add(b).ok_or(FeelsCoreError::MathOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_conservation_verification() {
        let data = ConservationData {
            w_s: 2500,
            w_t: 2500,
            w_l: 2500,
            w_tau: 2500,
            ln_g_s: 1000,   // 0.001 in natural units
            ln_g_t: -1000,  // -0.001
            ln_g_l: 500,    // 0.0005
            ln_g_tau: -500, // -0.0005
        };
        
        // This should satisfy conservation
        assert!(data.verify_conservation().unwrap());
    }
    
    #[test]
    fn test_buffer_adjustment() {
        let data = ConservationData {
            w_s: 3000,
            w_t: 3000,
            w_l: 3000,
            w_tau: 1000,
            ln_g_s: 1000,
            ln_g_t: 1000,
            ln_g_l: 1000,
            ln_g_tau: 0,
        };
        
        let adjustment = data.calculate_buffer_adjustment().unwrap();
        // Buffer should adjust to -3 * weighted sum
        assert_eq!(adjustment, -9000);
    }
}