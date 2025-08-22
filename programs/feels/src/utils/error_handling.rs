/// Provides comprehensive validation and error handling utilities across the protocol.
/// Implements defensive programming practices with extensive input validation,
/// bounds checking, and edge case handling. Ensures protocol safety by catching
/// potential issues early and providing clear error messages for debugging.

use anchor_lang::prelude::*;
use crate::state::PoolError;

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
        require!(amount > 0, PoolError::InvalidAmount);
        require!(amount >= min_amount, PoolError::SwapAmountTooSmall);
        require!(amount <= u64::MAX / 2, PoolError::ArithmeticOverflow); // Leave room for calculations
        Ok(())
    }
    
    /// Validate price bounds with overflow protection
    pub fn validate_sqrt_price(sqrt_price: u128) -> Result<()> {
        require!(
            sqrt_price >= crate::utils::MIN_SQRT_PRICE_X64,
            PoolError::PriceOutOfBounds
        );
        require!(
            sqrt_price <= crate::utils::MAX_SQRT_PRICE_X64,
            PoolError::PriceOutOfBounds
        );
        Ok(())
    }
    
    /// Validate tick range with comprehensive checks
    pub fn validate_tick_range(tick_lower: i32, tick_upper: i32, tick_spacing: i16) -> Result<()> {
        // Basic range validation
        require!(tick_lower < tick_upper, PoolError::InvalidTickRange);
        require!(
            tick_lower >= crate::utils::math_ticks::MIN_TICK,
            PoolError::TickOutOfBounds
        );
        require!(
            tick_upper <= crate::utils::math_ticks::MAX_TICK,
            PoolError::TickOutOfBounds
        );
        
        // Spacing alignment validation
        require!(tick_lower % tick_spacing as i32 == 0, PoolError::TickNotAligned);
        require!(tick_upper % tick_spacing as i32 == 0, PoolError::TickNotAligned);
        
        // Reasonable range size (prevent DoS)
        let tick_distance = tick_upper.saturating_sub(tick_lower);
        require!(tick_distance <= 1_000_000, PoolError::InvalidTickRange); // Max 1M tick spread
        
        Ok(())
    }
    
    /// Validate liquidity operations with overflow protection
    pub fn validate_liquidity_delta(current: u128, delta: i128) -> Result<u128> {
        if delta >= 0 {
            // Adding liquidity
            let new_liquidity = current.checked_add(delta as u128)
                .ok_or(PoolError::LiquidityOverflow)?;
            require!(new_liquidity <= u128::MAX / 2, PoolError::LiquidityOverflow);
            Ok(new_liquidity)
        } else {
            // Removing liquidity
            let abs_delta = (-delta) as u128;
            require!(current >= abs_delta, PoolError::LiquidityUnderflow);
            Ok(current - abs_delta)
        }
    }
    
    /// Validate fee calculations with precision handling
    pub fn calculate_fee_safe(amount: u64, fee_rate: u16) -> Result<u64> {
        require!(fee_rate <= 10000, PoolError::InvalidFeeRate); // Max 100%
        
        let amount_u128 = amount as u128;
        let fee_rate_u128 = fee_rate as u128;
        
        let fee = amount_u128
            .checked_mul(fee_rate_u128)
            .ok_or(PoolError::MathOverflow)?
            .checked_div(10000)
            .ok_or(PoolError::DivisionByZero)?;
        
        require!(fee <= u64::MAX as u128, PoolError::MathOverflow);
        Ok(fee as u64)
    }
    
    /// Validate slippage protection
    pub fn validate_slippage(
        actual_amount: u64,
        expected_amount: u64,
        max_slippage_bps: u16,
    ) -> Result<()> {
        require!(max_slippage_bps <= 10000, PoolError::InvalidAmount); // Max 100%
        
        let min_acceptable = expected_amount
            .saturating_mul(10000u64.saturating_sub(max_slippage_bps as u64))
            .saturating_div(10000);
        
        require!(actual_amount >= min_acceptable, PoolError::SlippageExceeded);
        Ok(())
    }
    
    /// Validate account ownership with detailed checks
    pub fn validate_account_owner(
        account: &AccountInfo,
        expected_owner: &Pubkey,
        error: PoolError,
    ) -> Result<()> {
        require!(!account.data_is_empty(), error);
        require!(account.owner == expected_owner, error);
        Ok(())
    }
    
    /// Validate PDA derivation matches expected
    pub fn validate_pda(
        account: &AccountInfo,
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<()> {
        let (expected_pda, _bump) = Pubkey::find_program_address(seeds, program_id);
        require!(
            account.key() == expected_pda,
            PoolError::InvalidPool
        );
        Ok(())
    }
    
    // calculate_percentage moved to math_general.rs
    
    /// Validate timestamp for time-based operations
    pub fn validate_timestamp(
        current_timestamp: i64,
        target_timestamp: i64,
        max_future_seconds: i64,
    ) -> Result<()> {
        // Prevent operations too far in the past
        require!(
            target_timestamp >= current_timestamp - 3600, // Max 1 hour in past
            PoolError::InvalidOperation
        );
        
        // Prevent operations too far in the future
        require!(
            target_timestamp <= current_timestamp + max_future_seconds,
            PoolError::InvalidOperation
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
PoolError::InsufficientLiquidity
        );
        
        let remaining = source_balance.saturating_sub(transfer_amount);
        require!(
            remaining >= min_remaining,
PoolError::InsufficientLiquidity
        );
        
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper function to create error with context
pub fn create_error_with_context(message: &str) -> anchor_lang::error::Error {
    msg!("Error: {}", message);
    anchor_lang::error::Error::from(PoolError::InvalidOperation)
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