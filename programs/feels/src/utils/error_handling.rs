/// Provides validation and error handling utilities across the protocol.
/// Implements defensive programming practices with input validation,
/// bounds checking, and edge case handling. Ensures protocol safety by catching
/// potential issues early and providing clear error messages for debugging.

use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Type Definitions
// ============================================================================

/// This module provides additional safety checks and validation functions
/// to ensure all edge cases are handled properly throughout the codebase
pub struct ErrorHandling;

// ============================================================================
// Core Implementation
// ============================================================================

impl ErrorHandling {
    /// Validate token amounts with comprehensive checks
    pub fn validate_token_amount(amount: u64, min_amount: u64) -> Result<()> {
        require!(amount > 0, FeelsProtocolError::InvalidAmount);
        require!(amount >= min_amount, FeelsProtocolError::SwapAmountTooSmall);
        require!(amount <= u64::MAX / 2, FeelsProtocolError::ArithmeticOverflow); // Leave room for calculations
        Ok(())
    }

    /// Validate price bounds with overflow protection  
    pub fn validate_sqrt_price(sqrt_price: u128) -> Result<()> {
        require!(
            sqrt_price > 0 && sqrt_price < u128::MAX / 1000, // Basic bounds check without Q96 dependency
            FeelsProtocolError::InvalidParameter
        );
        Ok(())
    }

    /// Validate tick range with comprehensive checks
    pub fn validate_tick_range(tick_lower: i32, tick_upper: i32, tick_spacing: i16) -> Result<()> {
        // Basic range validation
        require!(tick_lower < tick_upper, FeelsProtocolError::InvalidTickRange);
        require!(
            tick_lower >= crate::utils::MIN_TICK,
            FeelsProtocolError::TickOutOfBounds
        );
        require!(
            tick_upper <= crate::utils::MAX_TICK,
            FeelsProtocolError::TickOutOfBounds
        );

        // Spacing alignment validation
        require!(
            tick_lower % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );
        require!(
            tick_upper % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );

        // Reasonable range size (prevent DoS)
        let tick_distance = tick_upper.saturating_sub(tick_lower);
        require!(tick_distance <= 1_000_000, FeelsProtocolError::InvalidTickRange); // Max 1M tick spread

        Ok(())
    }

    /// Validate liquidity operations with overflow protection
    pub fn validate_liquidity_delta(current: u128, delta: i128) -> Result<u128> {
        if delta >= 0 {
            // Adding liquidity
            let new_liquidity = current
                .checked_add(delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            require!(new_liquidity <= u128::MAX / 2, FeelsProtocolError::MathOverflow);
            Ok(new_liquidity)
        } else {
            // Removing liquidity
            // Convert negative i128 to positive u128 safely
            let abs_delta = delta.unsigned_abs();
            require!(current >= abs_delta, FeelsProtocolError::ArithmeticUnderflow);
            Ok(current - abs_delta)
        }
    }


    /// Validate fee calculations with precision handling (simplified version without PreciseNumber)
    pub fn calculate_fee_safe(amount: u64, fee_rate: u16) -> Result<u64> {
        require!(fee_rate <= 10000, FeelsProtocolError::InvalidParameter); // Max 100%

        // Use u128 to prevent overflow during multiplication
        let amount_u128 = amount as u128;
        let fee_rate_u128 = fee_rate as u128;

        let fee_u128 = amount_u128
            .checked_mul(fee_rate_u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(10000u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;

        // Convert back to u64
        fee_u128
            .try_into()
            .map_err(|_| FeelsProtocolError::MathOverflow.into())
    }

    /// Validate slippage protection
    pub fn validate_slippage(
        actual_amount: u64,
        expected_amount: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        require!(max_slippage_bps <= 10000, FeelsProtocolError::InvalidAmount); // Max 100%

        let min_acceptable = expected_amount
            .saturating_mul(10000u64.saturating_sub(max_slippage_bps as u64))
            .saturating_div(10000);

        require!(actual_amount >= min_acceptable, FeelsProtocolError::ExcessiveChange);
        Ok(())
    }

    /// Validate account ownership with detailed checks
    pub fn validate_account_owner(
        account: &AccountInfo,
        expected_owner: &Pubkey,
        error: FeelsProtocolError,
    ) -> Result<()> {
        if account.data_is_empty() {
            return Err(error.into());
        }
        if account.owner != expected_owner {
            return Err(error.into());
        }
        Ok(())
    }

    /// Validate PDA derivation matches expected
    pub fn validate_pda(account: &AccountInfo, seeds: &[&[u8]], program_id: &Pubkey) -> Result<()> {
        let (expected_pda, _bump) = Pubkey::find_program_address(seeds, program_id);
        require!(account.key() == expected_pda, FeelsProtocolError::InvalidPool);
        Ok(())
    }

    /// Validate timestamp for time-based operations
    pub fn validate_timestamp(
        current_timestamp: i64,
        target_timestamp: i64,
        max_future_seconds: i64,
    ) -> Result<()> {
        // Prevent operations too far in the past
        require!(
            target_timestamp >= current_timestamp - 3600, // Max 1 hour in past
            FeelsProtocolError::InvalidOperation
        );

        // Prevent operations too far in the future
        require!(
            target_timestamp <= current_timestamp + max_future_seconds,
            FeelsProtocolError::InvalidOperation
        );

        Ok(())
    }

    /// Validate tick range for 3D encoding
    pub fn validate_tick_component(tick: i32, max_bits: u32) -> Result<()> {
        require!(
            tick.abs() < (1 << max_bits),
            FeelsProtocolError::InvalidTickRange
        );
        Ok(())
    }

    /// Validate leverage within acceptable range
    pub fn validate_leverage(leverage: u64, min: u64, max: u64) -> Result<()> {
        require!(
            leverage >= min && leverage <= max,
            FeelsProtocolError::InvalidPercentage
        );
        Ok(())
    }

    /// Validate chronological timestamp ordering
    pub fn validate_timestamp_ordering(new_timestamp: i64, last_timestamp: i64) -> Result<()> {
        require!(
            new_timestamp > last_timestamp,
            FeelsProtocolError::InvalidParameter
        );
        Ok(())
    }

    /// Safe token transfer with balance validation
    pub fn validate_transfer_amount(
        source_balance: u64,
        transfer_amount: u64,
        min_remaining: u64,
    ) -> Result<()> {
        require!(
            source_balance >= transfer_amount,
            FeelsProtocolError::InsufficientLiquidity
        );

        let remaining = source_balance.saturating_sub(transfer_amount);
        require!(remaining >= min_remaining, FeelsProtocolError::InsufficientLiquidity);

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper function to create error with context
pub fn create_error_with_context(_message: &str) -> anchor_lang::error::Error {
    anchor_lang::error::Error::from(FeelsProtocolError::InvalidOperation)
}

/// Helper function to handle anchor errors
pub fn handle_anchor_error(error: anchor_lang::error::Error) -> Result<()> {
    Err(error)
}

// ============================================================================
// Macros
// ============================================================================

/// Macro for comprehensive error handling in operations
#[macro_export]
macro_rules! safe_operation {
    ($operation:expr, $error:expr) => {
        $operation.ok_or($error)?
    };
}

/// Macro for validating numeric bounds
#[macro_export]
macro_rules! validate_bounds {
    ($value:expr, $min:expr, $max:expr, $error:expr) => {
        require!($value >= $min && $value <= $max, $error);
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidity_validation() {
        // Test adding liquidity
        let result = ErrorHandling::validate_liquidity_delta(1000, 500);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1500);

        // Test removing liquidity
        let result = ErrorHandling::validate_liquidity_delta(1000, -500);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 500);

        // Test underflow
        let result = ErrorHandling::validate_liquidity_delta(100, -500);
        assert!(result.is_err());
    }

    #[test]
    fn test_fee_calculation() {
        // 0.3% fee on 10000
        let fee = ErrorHandling::calculate_fee_safe(10000, 30).unwrap();
        assert_eq!(fee, 30);

        // 1% fee on 10000
        let fee = ErrorHandling::calculate_fee_safe(10000, 100).unwrap();
        assert_eq!(fee, 100);
    }
}
