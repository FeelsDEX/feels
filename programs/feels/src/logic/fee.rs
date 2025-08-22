/// Handles all fee calculations including trading fees, protocol fees, and LP rewards.
/// Implements tiered fee structure (0.01%, 0.05%, 0.3%, 1%) matching Uniswap V3 standards.
/// Ensures precise fee accounting with basis point precision and proper rounding to
/// protect protocol and LPs from rounding errors while maintaining fair pricing.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use crate::utils::constant::{BASIS_POINTS_DENOMINATOR, MAX_PROTOCOL_FEE_RATE, VALID_FEE_TIERS};

/// This module provides the single source of truth for all fee calculations,
/// ensuring consistency across the protocol. All fee rates are expressed in
/// basis points (1 basis point = 0.01%).
/// 
/// Standard fee tiers (matching Uniswap V3):
/// - 1 basis point (0.01%) for stablecoin pairs
/// - 5 basis points (0.05%) for standard pairs  
/// - 30 basis points (0.30%) for exotic pairs
/// - 100 basis points (1.00%) for very exotic pairs

// ============================================================================
// Type Definitions
// ============================================================================

/// Fee calculation result
#[derive(Debug, Clone, Copy)]
pub struct FeeBreakdown {
    pub total_fee: u64,        // Total fee charged
    pub protocol_fee: u64,     // Amount going to protocol
    pub lp_fee: u64,          // Amount going to LPs
}

// ============================================================================
// Fee Mathematics Implementation
// ============================================================================

/// Unified fee mathematics for the entire protocol
pub struct FeeMath;

impl FeeMath {
    /// Calculate total fee amount from input amount
    /// 
    /// Args:
    /// - amount_in: Input amount for the swap
    /// - fee_rate: Fee tier in basis points (1, 5, 30, 100)
    /// 
    /// Returns: Total fee amount
    pub fn calculate_total_fee(amount_in: u64, fee_rate: u16) -> Result<u64> {
        Self::validate_fee_rate(fee_rate)?;
        
        let fee_amount = (amount_in as u128)
            .checked_mul(fee_rate as u128)
            .ok_or(PoolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(PoolError::DivisionByZero)?;
        
        Ok(fee_amount.min(u64::MAX as u128) as u64)
    }

    /// Calculate fee breakdown between protocol and LPs
    /// 
    /// Args:
    /// - total_fee: Total fee amount
    /// - protocol_fee_rate: Protocol's share in basis points of the total fee
    /// 
    /// Returns: FeeBreakdown with protocol and LP portions
    pub fn calculate_fee_breakdown(total_fee: u64, protocol_fee_rate: u16) -> Result<FeeBreakdown> {
        Self::validate_protocol_fee_rate(protocol_fee_rate)?;
        
        let protocol_fee = (total_fee as u128)
            .checked_mul(protocol_fee_rate as u128)
            .ok_or(PoolError::MathOverflow)?
            .checked_div(BASIS_POINTS_DENOMINATOR as u128)
            .ok_or(PoolError::DivisionByZero)? as u64;
        
        let lp_fee = total_fee.checked_sub(protocol_fee)
            .ok_or(PoolError::ArithmeticUnderflow)?;
        
        Ok(FeeBreakdown {
            total_fee,
            protocol_fee,
            lp_fee,
        })
    }

    /// Single function to calculate complete fee structure from input amount
    /// This is the primary fee calculation function used throughout the protocol
    /// 
    /// Args:
    /// - amount_in: Input amount for the swap
    /// - fee_rate: Fee tier in basis points
    /// - protocol_fee_rate: Protocol's share of total fee in basis points
    /// 
    /// Returns: Complete fee breakdown
    pub fn calculate_swap_fees(
        amount_in: u64,
        fee_rate: u16,
        protocol_fee_rate: u16,
    ) -> Result<FeeBreakdown> {
        let total_fee = Self::calculate_total_fee(amount_in, fee_rate)?;
        Self::calculate_fee_breakdown(total_fee, protocol_fee_rate)
    }

    /// Validate fee rate is within acceptable bounds and matches standard tiers
    pub fn validate_fee_rate(fee_rate: u16) -> Result<()> {
        require!(
            VALID_FEE_TIERS.contains(&fee_rate),
            PoolError::InvalidFeeRate
        );
        Ok(())
    }

    /// Validate protocol fee rate is within acceptable bounds
    pub fn validate_protocol_fee_rate(protocol_fee_rate: u16) -> Result<()> {
        require!(
            protocol_fee_rate <= MAX_PROTOCOL_FEE_RATE,
            PoolError::InvalidFeeRate
        );
        Ok(())
    }

    /// Calculate the tick spacing for a given fee rate (Uniswap V3 standard)
    pub fn fee_rate_to_tick_spacing(fee_rate: u16) -> Result<i16> {
        let tick_spacing = match fee_rate {
            1 => 1,      // 0.01% fee -> 1 tick spacing
            5 => 10,     // 0.05% fee -> 10 tick spacing  
            30 => 60,    // 0.30% fee -> 60 tick spacing
            100 => 200,  // 1.00% fee -> 200 tick spacing
            _ => return Err(PoolError::InvalidFeeRate.into()),
        };
        
        Ok(tick_spacing)
    }

    /// Get the standard protocol fee rate for a given fee tier
    /// This provides sensible defaults for protocol fee configuration
    pub fn get_standard_protocol_fee_rate(fee_rate: u16) -> Result<u16> {
        let protocol_rate = match fee_rate {
            1 => 100,    // 1% of 0.01% = 0.0001% protocol fee
            5 => 100,    // 1% of 0.05% = 0.0005% protocol fee
            30 => 100,   // 1% of 0.30% = 0.003% protocol fee
            100 => 100,  // 1% of 1.00% = 0.01% protocol fee
            _ => return Err(PoolError::InvalidFeeRate.into()),
        };
        
        Ok(protocol_rate)
    }

    /// Calculate effective fee rate after considering all factors
    /// This is where dynamic fee adjustments would be implemented in Phase 2
    pub fn calculate_effective_fee_rate(
        base_fee_rate: u16,
        _volume_24h: u128,
        _volatility_factor: u16,
    ) -> Result<u16> {
        // Phase 1: Just return base fee rate
        // Phase 2+: Implement dynamic fee adjustments based on volume/volatility
        Self::validate_fee_rate(base_fee_rate)?;
        Ok(base_fee_rate)
    }
}

// ============================================================================
// Fee Configuration Utilities
// ============================================================================

/// Fee configuration utilities for pool management
pub struct FeeConfig;

impl FeeConfig {
    /// Create fee configuration for a new pool
    pub fn create_for_pool(fee_rate: u16) -> Result<(u16, u16, i16)> {
        FeeMath::validate_fee_rate(fee_rate)?;
        
        let protocol_fee_rate = FeeMath::get_standard_protocol_fee_rate(fee_rate)?;
        let tick_spacing = FeeMath::fee_rate_to_tick_spacing(fee_rate)?;
        
        Ok((fee_rate, protocol_fee_rate, tick_spacing))
    }

    /// Validate fee configuration consistency
    pub fn validate_pool_fees(
        fee_rate: u16,
        protocol_fee_rate: u16,
        tick_spacing: i16,
    ) -> Result<()> {
        FeeMath::validate_fee_rate(fee_rate)?;
        FeeMath::validate_protocol_fee_rate(protocol_fee_rate)?;
        
        let expected_tick_spacing = FeeMath::fee_rate_to_tick_spacing(fee_rate)?;
        require!(
            tick_spacing == expected_tick_spacing,
            PoolError::InvalidTickSpacing
        );
        
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculations() {
        // Test standard fee calculation
        let amount_in = 1_000_000u64; // 1M tokens
        let fee_rate = 30u16; // 0.30%
        
        let total_fee = FeeMath::calculate_total_fee(amount_in, fee_rate).unwrap();
        assert_eq!(total_fee, 3_000); // 0.30% of 1M = 3K
        
        // Test fee breakdown
        let protocol_fee_rate = 100u16; // 1% of total fee
        let breakdown = FeeMath::calculate_fee_breakdown(total_fee, protocol_fee_rate).unwrap();
        assert_eq!(breakdown.protocol_fee, 30); // 1% of 3K = 30
        assert_eq!(breakdown.lp_fee, 2_970); // 99% of 3K = 2,970
        assert_eq!(breakdown.total_fee, 3_000);
    }

    #[test]
    fn test_complete_swap_fees() {
        let amount_in = 1_000_000u64;
        let fee_rate = 30u16;
        let protocol_fee_rate = 100u16;
        
        let breakdown = FeeMath::calculate_swap_fees(amount_in, fee_rate, protocol_fee_rate).unwrap();
        assert_eq!(breakdown.total_fee, 3_000);
        assert_eq!(breakdown.protocol_fee, 30);
        assert_eq!(breakdown.lp_fee, 2_970);
    }

    #[test]
    fn test_fee_rate_validation() {
        // Valid fee rates
        assert!(FeeMath::validate_fee_rate(1).is_ok());
        assert!(FeeMath::validate_fee_rate(5).is_ok());
        assert!(FeeMath::validate_fee_rate(30).is_ok());
        assert!(FeeMath::validate_fee_rate(100).is_ok());
        
        // Invalid fee rates
        assert!(FeeMath::validate_fee_rate(2).is_err());
        assert!(FeeMath::validate_fee_rate(1000).is_err());
    }

    #[test]
    fn test_tick_spacing_mapping() {
        assert_eq!(FeeMath::fee_rate_to_tick_spacing(1).unwrap(), 1);
        assert_eq!(FeeMath::fee_rate_to_tick_spacing(5).unwrap(), 10);
        assert_eq!(FeeMath::fee_rate_to_tick_spacing(30).unwrap(), 60);
        assert_eq!(FeeMath::fee_rate_to_tick_spacing(100).unwrap(), 200);
    }

    #[test]
    fn test_overflow_protection() {
        let max_amount = u64::MAX;
        let max_fee_rate = 100u16;
        
        // Should handle large amounts without overflow
        let result = FeeMath::calculate_total_fee(max_amount, max_fee_rate);
        assert!(result.is_ok());
    }
}