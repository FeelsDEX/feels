//! # Field Management
//! 
//! Market field validation and update logic.

use crate::errors::{CoreResult, FeelsCoreError};
use crate::constants::{MAX_SCALAR_CHANGE_BPS, MIN_UPDATE_INTERVAL, MAX_UPDATE_STALENESS};
use crate::types::field::FieldCommitmentData;

#[cfg(feature = "client")]
use serde::{Serialize, Deserialize};

/// Field update validation parameters
#[derive(Debug, Clone)]
#[cfg_attr(feature = "client", derive(Serialize, Deserialize))]
pub struct FieldUpdateParams {
    /// Maximum allowed change per update (basis points)
    pub max_scalar_change_bps: u32,
    /// Minimum interval between updates (seconds)
    pub min_update_interval: i64,
    /// Maximum staleness allowed (seconds)
    pub max_staleness: i64,
}

impl Default for FieldUpdateParams {
    fn default() -> Self {
        Self {
            max_scalar_change_bps: MAX_SCALAR_CHANGE_BPS,
            min_update_interval: MIN_UPDATE_INTERVAL,
            max_staleness: MAX_UPDATE_STALENESS,
        }
    }
}

/// Validate field update against policy constraints
pub fn validate_field_update(
    new_field: &FieldCommitmentData,
    current_field: &FieldCommitmentData,
    current_time: i64,
    params: &FieldUpdateParams,
) -> CoreResult<()> {
    // Check sequence number
    if new_field.sequence <= current_field.sequence {
        return Err(FeelsCoreError::InvalidSequence);
    }
    
    // Check staleness
    let age = current_time - new_field.snapshot_ts;
    if age > params.max_staleness {
        return Err(FeelsCoreError::StaleFieldData);
    }
    
    // Check update interval
    let interval = new_field.snapshot_ts - current_field.snapshot_ts;
    if interval < params.min_update_interval {
        return Err(FeelsCoreError::InvalidParameter);
    }
    
    // Check scalar changes
    validate_scalar_changes(new_field, current_field, params.max_scalar_change_bps)?;
    
    // Validate weights
    new_field.validate_weights()?;
    
    Ok(())
}

/// Validate scalar changes are within bounds
fn validate_scalar_changes(
    new_field: &FieldCommitmentData,
    current_field: &FieldCommitmentData,
    max_change_bps: u32,
) -> CoreResult<()> {
    // Check S change
    check_scalar_change(new_field.S, current_field.S, max_change_bps, "S")?;
    
    // Check T change
    check_scalar_change(new_field.T, current_field.T, max_change_bps, "T")?;
    
    // Check L change
    check_scalar_change(new_field.L, current_field.L, max_change_bps, "L")?;
    
    Ok(())
}

/// Check individual scalar change
fn check_scalar_change(
    new_value: u128,
    old_value: u128,
    max_change_bps: u32,
    _name: &str,
) -> CoreResult<()> {
    if old_value == 0 {
        return Ok(()); // Allow any change from 0
    }
    
    let change_bps = if new_value > old_value {
        // Increase: (new - old) / old * 10000
        ((new_value - old_value) * 10000) / old_value
    } else {
        // Decrease: (old - new) / old * 10000
        ((old_value - new_value) * 10000) / old_value
    };
    
    if change_bps > max_change_bps as u128 {
        return Err(FeelsCoreError::OutOfBounds);
    }
    
    Ok(())
}

#[cfg(feature = "advanced")]
pub mod advanced {
    use super::*;
    use crate::oracle::{TWAPOracle, VolatilityOracle};
    
    /// Compute field commitment from market state
    pub fn compute_field_commitment(
        twap_oracle: &TWAPOracle,
        volatility_oracle: &VolatilityOracle,
        current_time: i64,
        base_fee_bps: u64,
    ) -> CoreResult<FieldCommitmentData> {
        // Get TWAPs
        let twap_0 = twap_oracle.get_twap(3600, current_time)?; // 1-hour TWAP
        let twap_1 = twap_0; // Simplified: same price for both tokens
        
        // Get volatilities
        let (sigma_price, sigma_rate, sigma_leverage) = volatility_oracle.get_risk_scalers();
        
        // Compute domain scalars (simplified)
        // In production, these would come from complex market state aggregation
        let s = compute_spot_scalar(twap_0, twap_1)?;
        let t = compute_time_scalar()?;
        let l = compute_leverage_scalar()?;
        
        // Create field commitment
        FieldCommitmentData::new(
            s, t, l,
            3333, 3333, 3334, 0,    // Equal domain weights
            5000, 5000,              // Equal spot weights
            sigma_price, sigma_rate, sigma_leverage,
            twap_0, twap_1,
            base_fee_bps,
        )
    }
    
    fn compute_spot_scalar(twap_0: u128, twap_1: u128) -> CoreResult<u128> {
        // S = sqrt(twap_0 * twap_1)
        use crate::math::safe_math::sqrt_u128;
        let product = crate::math::safe_math::safe_mul_u128(twap_0, twap_1)?;
        Ok(sqrt_u128(product))
    }
    
    fn compute_time_scalar() -> CoreResult<u128> {
        // Placeholder: return neutral value
        Ok(crate::constants::Q64)
    }
    
    fn compute_leverage_scalar() -> CoreResult<u128> {
        // Placeholder: return neutral value
        Ok(crate::constants::Q64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_field_validation() {
        let current = FieldCommitmentData {
            S: 1000,
            T: 1000,
            L: 1000,
            w_s: 3333,
            w_t: 3333,
            w_l: 3334,
            w_tau: 0,
            omega_0: 5000,
            omega_1: 5000,
            sigma_price: 100,
            sigma_rate: 50,
            sigma_leverage: 200,
            twap_0: 1000,
            twap_1: 1000,
            snapshot_ts: 100,
            max_staleness: 300,
            sequence: 1,
            base_fee_bps: 30,
            local_coefficients: None,
            commitment_hash: None,
            lipschitz_L: None,
            gap_bps: None,
        };
        
        let mut new = current.clone();
        new.S = 1020; // 2% increase
        new.sequence = 2;
        new.snapshot_ts = 200;
        
        let params = FieldUpdateParams::default();
        assert!(validate_field_update(&new, &current, 250, &params).is_ok());
        
        // Test excessive change
        new.S = 1030; // 3% increase (exceeds 2% limit)
        assert!(validate_field_update(&new, &current, 250, &params).is_err());
    }
}