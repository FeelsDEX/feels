/// Conservation law verification re-exported from logic module
/// 
/// All conservation verification is handled by the logic module's comprehensive
/// implementation. This utils module provides convenience re-exports and the
/// minimal buffer weight calculation helper.
/// 
/// The fundamental conservation law:
/// ```
/// Σ wᵢ ln(gᵢ) = 0
/// ```
/// 
/// Must be verified for all rebase operations to ensure no value creation/destruction.

// Re-export conservation types from logic module
pub use crate::logic::conservation_check::{
    ConservationProof,
    ConservationCheckResult,
    verify_conservation,
    verify_rebase_conservation,
    BufferConservationContext,
    RebaseOperationType,
    BufferConservationProof,
    verify_conservation_with_buffer,
    calculate_buffer_fee_share,
    DomainActivity,
    build_buffer_conservation_proof,
    builders,
};

/// Subdomain-specific buffer weight calculations
pub mod subdomain {
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