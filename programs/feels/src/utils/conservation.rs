/// Centralized conservation law primitives for the Feels Protocol.
/// This module ensures all conservation calculations use the same core primitives,
/// preventing duplication and ensuring consistency across the codebase.
///
/// # Conservation Law
/// 
/// The fundamental conservation law for rebasing operations is:
/// ```
/// Σ w_i · ln(g_i) = 0
/// ```
/// Where:
/// - w_i are the weights (must sum to 1)
/// - g_i are the rebase factors
/// - ln is the natural logarithm
///
/// This ensures no value is created or destroyed, only transferred between participants.

// Conservation verification and solving must be done off-chain using sdk_math.rs
// On-chain programs receive pre-computed rebase factors from keepers/oracles

/// Example usage for lending rebase conservation
/// ```ignore
/// use crate::utils::{FixedPoint, conservation};
/// 
/// // Define weights: lenders, borrowers, buffer
/// let weights = vec![
///     FixedPoint::from_u32(4000),  // w_A = 0.4
///     FixedPoint::from_u32(5000),  // w_D = 0.5  
///     FixedPoint::from_u32(1000),  // w_tau = 0.1
/// ];
/// 
/// // Define known factors (None for the one to solve)
/// let factors = vec![
///     Some(FixedPoint::from_u32(10200)), // g_A = 1.02 (2% yield)
///     Some(FixedPoint::from_u32(10300)), // g_D = 1.03 (3% rate)
///     None,                               // g_tau to solve
/// ];
/// 
/// // Solve for buffer factor
/// let result = conservation::solve_rebase(&weights, &factors)?;
/// let g_tau = result[2]; // Buffer scales down to conserve
/// 
/// // Verify conservation
/// let tolerance = FixedPoint::from_u32(1); // 0.0001 tolerance
/// assert!(conservation::verify(&weights, &result, tolerance)?);
/// ```

/// Example usage for leverage P&L conservation  
/// ```ignore
/// use crate::utils::{FixedPoint, conservation};
/// 
/// // Equal weights for longs and shorts
/// let weights = vec![
///     FixedPoint::from_u32(5000),  // w_long = 0.5
///     FixedPoint::from_u32(5000),  // w_short = 0.5
/// ];
/// 
/// // Price moved up 5%, so g = 1.05 for longs
/// let factors = vec![
///     Some(FixedPoint::from_u32(10500)), // g_long = 1.05
///     None,                               // g_short to solve
/// ];
/// 
/// // Solve for short factor
/// let result = conservation::solve_rebase(&weights, &factors)?;
/// let g_short = result[1]; // Should be 1/1.05 ≈ 0.9524
/// 
/// // Verify: w_long * ln(g_long) + w_short * ln(g_short) = 0
/// let tolerance = FixedPoint::from_u32(1);
/// assert!(conservation::verify(&weights, &result, tolerance)?);
/// ```

/// Conservation law for sub-domain rebasing with buffer participation
/// 
/// For lending domain with buffer:
/// ```
/// w_A · ln(g_A) + w_D · ln(g_D) + w_tau^(lend) · ln(g_tau) = 0
/// ```
/// 
/// Where local buffer weight is:
/// ```
/// w_tau^(lend) = ζ_lend · τ_value / (A + D + ζ_lend · τ_value)
/// ```
pub mod subdomain {
    // use super::*; // Unused import
    use anchor_lang::prelude::*;
    
    /// Calculate buffer participation weight for a subdomain
    /// Returns weight in basis points (out of 10,000)
    pub fn calculate_buffer_weight(
        domain_value: u128,
        buffer_value: u128, 
        participation_coefficient: u64, // basis points
    ) -> Result<u64> {
        const BPS: u64 = 10_000;
        
        // Calculate participated buffer value
        let participated_buffer = (buffer_value as u128 * participation_coefficient as u128) / BPS as u128;
        
        // Total value including buffer participation
        let total_value = domain_value
            .checked_add(participated_buffer)
            .ok_or(crate::error::FeelsProtocolError::MathOverflow)?;
            
        // Buffer weight = participated_buffer / total_value
        if total_value == 0 {
            return Ok(0);
        }
        
        let weight_bps = ((participated_buffer * BPS as u128) / total_value) as u64;
        Ok(weight_bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buffer_weight_calculation() {
        // Test with 50% participation coefficient
        let domain_value = 1_000_000u128;
        let buffer_value = 100_000u128;
        let participation_coef = 5000u64; // 50%
        
        let weight = subdomain::calculate_buffer_weight(
            domain_value,
            buffer_value,
            participation_coef
        ).unwrap();
        
        // Expected: 50k / (1M + 50k) = 50k / 1.05M ≈ 476 bps
        assert!(weight > 470 && weight < 480);
    }
    
    #[test]
    fn test_zero_buffer_weight() {
        let weight = subdomain::calculate_buffer_weight(
            1_000_000u128,
            0u128,
            5000u64
        ).unwrap();
        
        assert_eq!(weight, 0);
    }
}