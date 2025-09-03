/// Standardized patterns for instruction handlers
/// 
/// This module provides macros and utilities to ensure consistent structure
/// across all instruction handlers in the protocol.
use anchor_lang::prelude::*;
use std::cell::Ref;
use crate::error::FeelsError;

// ============================================================================
// Instruction Handler Pattern Traits
// ============================================================================

/// Standard phases for instruction execution
pub trait InstructionHandler<'info, P, R> {
    /// Phase 1: Validate inputs and setup
    fn validate(&self, params: &P) -> Result<()>;
    
    /// Phase 2: Load and prepare state
    fn prepare_state(&mut self) -> Result<()>;
    
    /// Phase 3: Execute core logic
    fn execute(&mut self, params: P) -> Result<R>;
    
    /// Phase 4: Emit events
    fn emit_events(&self, result: &R) -> Result<()>;
    
    /// Phase 5: Cleanup and finalize
    fn finalize(&mut self) -> Result<()>;
}

/// Common validation utilities
pub trait ValidationUtils {
    /// Validate non-zero amount
    fn validate_amount(&self, amount: u64) -> Result<()> {
        require!(
            amount > 0,
            FeelsError::zero_amount("input")
        );
        Ok(())
    }
    
    /// Validate basis points value
    fn validate_bps(&self, bps: u16) -> Result<()> {
        require!(
            bps <= 10_000,
            FeelsError::ValidationError
        );
        Ok(())
    }
    
    /// Validate authority
    fn validate_authority(&self, authority: &Pubkey, expected: &Pubkey) -> Result<()> {
        require!(
            authority == expected,
            FeelsError::Unauthorized
        );
        Ok(())
    }
}

// ============================================================================
// Instruction Handler Macros
// ============================================================================

/// Macro to generate standard instruction handler structure
#[macro_export]
macro_rules! instruction_handler {
    (
        $handler_name:ident,
        $context_type:ty,
        $params_type:ty,
        $result_type:ty,
        {
            validate: $validate_block:block,
            prepare: $prepare_block:block,
            execute: $execute_block:block,
            events: $events_block:block,
            finalize: $finalize_block:block
        }
    ) => {
        pub fn $handler_name<'info>(
            ctx: Context<'_, '_, 'info, 'info, $context_type>,
            params: $params_type,
        ) -> Result<$result_type> {
            // Create a closure that captures ctx and params for validation
            let validate_fn = |ctx: &Context<'_, '_, 'info, 'info, $context_type>, params: &$params_type| -> Result<()> {
                $validate_block
                Ok(())
            };
            
            // Phase 1: Validation
            msg!("Phase 1: Validating inputs");
            validate_fn(&ctx, &params)?;
            
            // Create a closure for preparation
            let prepare_fn = |ctx: &Context<'_, '_, 'info, 'info, $context_type>, params: &$params_type| -> Result<()> {
                $prepare_block
                Ok(())
            };
            
            // Phase 2: State preparation
            msg!("Phase 2: Preparing state");
            prepare_fn(&ctx, &params)?;
            
            // Create a closure for execution
            let execute_fn = |ctx: &Context<'_, '_, 'info, 'info, $context_type>, params: $params_type| -> Result<$result_type> {
                $execute_block
            };
            
            // Phase 3: Core execution
            msg!("Phase 3: Executing logic");
            let result = execute_fn(&ctx, params)?;
            
            // Create a closure for events
            let events_fn = |ctx: &Context<'_, '_, 'info, 'info, $context_type>, result: &$result_type| -> Result<()> {
                $events_block
                Ok(())
            };
            
            // Phase 4: Event emission
            msg!("Phase 4: Emitting events");
            events_fn(&ctx, &result)?;
            
            // Create a closure for finalization
            let finalize_fn = |result: &$result_type, params: &$params_type| -> Result<()> {
                $finalize_block
                Ok(())
            };
            
            // Phase 5: Finalization
            msg!("Phase 5: Finalizing");
            finalize_fn(&result, &params)?;
            
            Ok(result)
        }
    };
}

/// Macro for common validation patterns
#[macro_export]
macro_rules! validate {
    (amount: $amount:expr) => {
        require!($amount > 0, FeelsError::zero_amount("input"));
    };
    
    (authority: $auth:expr, $expected:expr) => {
        require!(
            $auth == $expected,
            FeelsError::Unauthorized
        );
    };
    
    (range: $value:expr, $min:expr, $max:expr, $name:expr) => {
        require!(
            $value >= $min && $value <= $max,
            FeelsError::InvalidRange
        );
    };
}

/// Macro for reentrancy protection
#[macro_export]
macro_rules! with_reentrancy_lock {
    ($pool:expr, $body:block) => {{
        use crate::state::reentrancy::ReentrancyStatus;
        
        // Acquire lock
        let current_status = $pool.get_reentrancy_status()?;
        require!(
            current_status == ReentrancyStatus::Unlocked,
            FeelsError::ReentrancyDetected
        );
        $pool.set_reentrancy_status(ReentrancyStatus::Locked)?;
        
        // Execute body
        let result = $body;
        
        // Release lock
        $pool.set_reentrancy_status(ReentrancyStatus::Unlocked)?;
        
        result
    }};
}

// ============================================================================
// Event Builder Pattern
// ============================================================================

// EventBuilder removed - use Anchor's emit! directly for better consistency

// ============================================================================
// Common Instruction Patterns
// ============================================================================

/// Pattern for swap-like instructions
pub struct SwapPattern;

impl SwapPattern {
    /// Validate swap parameters
    pub fn validate_params(
        amount_in: u64,
        min_amount_out: u64,
        sqrt_price_limit: u128,
    ) -> Result<()> {
        validate!(amount: amount_in);
        require!(
            min_amount_out > 0,
            FeelsError::zero_amount("min_output")
        );
        require!(
            sqrt_price_limit > 0,
            FeelsError::InvalidAmount
        );
        Ok(())
    }
}

/// Pattern for liquidity-like instructions
pub struct LiquidityPattern;

impl LiquidityPattern {
    /// Validate liquidity parameters
    pub fn validate_params(
        liquidity_amount: u128,
        tick_lower: i32,
        tick_upper: i32,
    ) -> Result<()> {
        require!(
            liquidity_amount > 0,
            FeelsError::zero_amount("liquidity")
        );
        require!(
            tick_lower < tick_upper,
            FeelsError::InvalidRange
        );
        Ok(())
    }
}

/// Pattern for admin-like instructions
pub struct AdminPattern;

impl AdminPattern {
    /// Validate admin operation
    pub fn validate_authority(
        signer: &Pubkey,
        expected_authority: &Pubkey,
    ) -> Result<()> {
        require!(
            signer == expected_authority,
            crate::error::FeelsProtocolError::Unauthorized
        );
        Ok(())
    }
    
    /// Validate parameter update
    pub fn validate_update<T: PartialOrd + std::fmt::Display>(
        new_value: T,
        min_allowed: T,
        max_allowed: T,
        _param_name: &str,
    ) -> Result<()> {
        require!(
            new_value >= min_allowed && new_value <= max_allowed,
            FeelsError::ValidationError
        );
        Ok(())
    }
}

// ============================================================================
// State Loading Helpers
// ============================================================================

/// Helper to load multiple accounts with error context
pub struct StateLoader;

impl StateLoader {
    /// Load account with descriptive error
    pub fn load_account<'info, T: anchor_lang::AccountDeserialize + anchor_lang::Owner>(
        account: &'info AccountInfo<'info>,
        _name: &str,
    ) -> Result<T> {
        T::try_deserialize(&mut &account.data.borrow()[..])
            .map_err(|_| FeelsError::NotInitialized.into())
    }
    
    /// Load zero-copy account with descriptive error
    pub fn load_zero_copy<'info, T: anchor_lang::ZeroCopy + anchor_lang::Owner>(
        loader: &'info AccountLoader<'info, T>,
        name: &str,
    ) -> Result<Ref<'info, T>> {
        loader.load().map_err(|_| {
            msg!("Failed to load zero-copy account: {} at {}", name, loader.key());
            FeelsError::NotInitialized.into()
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validation_utils() {
        struct TestValidator;
        impl ValidationUtils for TestValidator {}
        
        let validator = TestValidator;
        
        // Test amount validation
        assert!(validator.validate_amount(100).is_ok());
        assert!(validator.validate_amount(0).is_err());
        
        // Test bps validation
        assert!(validator.validate_bps(5000).is_ok());
        assert!(validator.validate_bps(10001).is_err());
    }
    
    #[test]
    fn test_swap_pattern() {
        // Valid swap params
        assert!(SwapPattern::validate_params(1000, 900, 1_u128 << 96).is_ok());
        
        // Invalid swap params
        assert!(SwapPattern::validate_params(0, 900, 1_u128 << 96).is_err());
        assert!(SwapPattern::validate_params(1000, 0, 1_u128 << 96).is_err());
        assert!(SwapPattern::validate_params(1000, 900, 0).is_err());
    }
}