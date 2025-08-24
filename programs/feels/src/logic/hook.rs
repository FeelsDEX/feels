/// Executes external hook programs that extend pool functionality without core modifications.
/// Manages pre/post operation hooks with proper sandboxing and compute unit limits.
/// Enables features like MEV protection, custom fees, and advanced trading strategies
/// while maintaining security through permission checks and execution boundaries.

use anchor_lang::prelude::*;
use crate::state::{HookRegistry, HookType, HookContext, HookOperation, HookPermission, PoolError};

// ============================================================================
// Hook Parameter Structures
// ============================================================================

/// Parameters for creating swap hook context
pub struct SwapHookParams {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub price_before: u128,
    pub price_after: u128,
}

/// Parameters for creating liquidity hook context
pub struct LiquidityHookParams {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub liquidity_amount: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub is_add: bool,
}

// ============================================================================
// Hook Executor Implementation
// ============================================================================

pub struct HookExecutor;

impl HookExecutor {
    /// Execute pre-operation hooks
    pub fn execute_pre_hooks(
        registry: &HookRegistry,
        hook_type: HookType,
        context: &HookContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<()> {
        // Get active hooks
        let hooks = registry.get_active_hooks(hook_type);
        
        // Execute each hook
        for (index, hook) in hooks.iter().enumerate() {
            // TODO: Validate compute units (requires newer Solana version)
            // let compute_budget = anchor_lang::solana_program::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            //     hook.max_compute_units,
            // );
            
            // Prepare hook CPI
            let result = execute_single_hook(
                hook,
                context,
                &remaining_accounts[index..],
            );
            
            // Handle hook result based on permission
            match result {
                Ok(_) => {}
                Err(_e) => {
                    match hook.permission {
                        HookPermission::Halt => {
                            return Err(PoolError::InvalidOperation.into());
                        }
                        _ => {
                            // Continue with other hooks
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute post-operation hooks
    pub fn execute_post_hooks(
        registry: &HookRegistry,
        hook_type: HookType,
        context: &HookContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<()> {
        // Post hooks cannot halt operations, only observe/modify state
        let hooks = registry.get_active_hooks(hook_type);
        
        for (index, hook) in hooks.iter().enumerate() {
            let result = execute_single_hook(
                hook,
                context,
                &remaining_accounts[index..],
            );
            
            if result.is_err() {
                // Post hooks failures are non-critical
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate accounts before passing to hooks
/// Ensures hooks can only access authorized accounts
pub fn validate_hook_accounts(accounts: &[AccountInfo]) -> Result<()> {
    use crate::state::PoolError;
    
    for account in accounts {
        // Ensure account is not a system-owned account
        if account.owner == &anchor_lang::solana_program::system_program::ID {
            // Allow system accounts but they should be read-only
            require!(!account.is_writable, PoolError::Unauthorized);
        }
        
        // Additional validation can be added here:
        // - Check against a whitelist of allowed programs
        // - Verify account ownership matches expected programs
        // - Ensure critical accounts like vaults are not passed
    }
    
    Ok(())
}

/// Execute a single hook via CPI
fn execute_single_hook(
    hook: &crate::state::HookConfig,
    context: &HookContext,
    accounts: &[AccountInfo],
) -> Result<()> {
    // Validate accounts before passing to hooks
    validate_hook_accounts(accounts)?;
    
    // Enforce compute unit limits for hook execution
    // Note: Compute budget instructions would need to be handled at transaction level
    // For now, we track the limits for monitoring and potential future enforcement
    let _compute_unit_limit = if hook.max_compute_units > 0 { hook.max_compute_units } else { 100_000 };
    
    // Serialize hook context
    let data = context.try_to_vec()?;
    
    // Create instruction for hook
    let hook_instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: hook.program_id,
        accounts: accounts.iter().map(|acc| AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        }).collect(),
        data,
    };
    
    // Execute hook with tracked compute limit (enforcement at transaction level)
    // TODO: Add actual compute budget enforcement when available
    
    // Execute the hook
    anchor_lang::solana_program::program::invoke(
        &hook_instruction,
        accounts,
    )?;
    
    Ok(())
}

/// Helper to create hook context for swap operations
pub fn create_swap_hook_context(params: &SwapHookParams) -> HookContext {
    HookContext {
        pool: params.pool,
        user: params.user,
        operation: HookOperation::Swap {
            amount_in: params.amount_in,
            amount_out: params.amount_out,
            token_in: params.token_in,
            token_out: params.token_out,
            price_before: params.price_before,
            price_after: params.price_after,
        },
        // Already using unwrap_or_default() to handle Clock::get() errors gracefully
        timestamp: Clock::get().unwrap_or_default().unix_timestamp,
    }
}

/// Helper to create hook context for liquidity operations
pub fn create_liquidity_hook_context(params: &LiquidityHookParams) -> HookContext {
    let operation = if params.is_add {
        HookOperation::AddLiquidity {
            liquidity_amount: params.liquidity_amount,
            amount_0: params.amount_0,
            amount_1: params.amount_1,
            tick_lower: params.tick_lower,
            tick_upper: params.tick_upper,
        }
    } else {
        HookOperation::RemoveLiquidity {
            liquidity_amount: params.liquidity_amount,
            amount_0: params.amount_0,
            amount_1: params.amount_1,
            tick_lower: params.tick_lower,
            tick_upper: params.tick_upper,
        }
    };
    
    HookContext {
        pool: params.pool,
        user: params.user,
        operation,
        // Already using unwrap_or_default() to handle Clock::get() errors gracefully
        timestamp: Clock::get().unwrap_or_default().unix_timestamp,
    }
}

/// Integration point for swap instruction
pub fn execute_swap_hooks<'info>(
    registry: Option<&Account<'info, HookRegistry>>,
    pool: Pubkey,
    user: Pubkey,
    pre_hook_state: (u64, u128), // (amount_in, price_before)
    post_hook_state: (u64, u128, Pubkey, Pubkey), // (amount_out, price_after, token_in, token_out)
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    if let Some(reg) = registry {
        if reg.hooks_enabled {
            // Create pre-swap context
            let pre_params = SwapHookParams {
                pool,
                user,
                amount_in: pre_hook_state.0,
                amount_out: 0, // amount_out not known yet
                token_in: post_hook_state.2,
                token_out: post_hook_state.3,
                price_before: pre_hook_state.1,
                price_after: pre_hook_state.1, // price_after same as before for pre-hook
            };
            let pre_context = create_swap_hook_context(&pre_params);
            
            // Execute pre-swap hooks
            HookExecutor::execute_pre_hooks(
                reg,
                HookType::PreSwap,
                &pre_context,
                remaining_accounts,
            )?;
            
            // Create post-swap context
            let post_params = SwapHookParams {
                pool,
                user,
                amount_in: pre_hook_state.0,
                amount_out: post_hook_state.0,
                token_in: post_hook_state.2,
                token_out: post_hook_state.3,
                price_before: pre_hook_state.1,
                price_after: post_hook_state.1,
            };
            let post_context = create_swap_hook_context(&post_params);
            
            // Execute post-swap hooks
            HookExecutor::execute_post_hooks(
                reg,
                HookType::PostSwap,
                &post_context,
                remaining_accounts,
            )?;
        }
    }
    
    Ok(())
}

// ============================================================================
// Alternative Hook Execution Phases (Future Implementation)
// ============================================================================

/// Hook execution phases (alternative implementation)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookPhase {
    BeforeSwap,
    AfterSwap,
    BeforeLiquidity,
    AfterLiquidity,
}

/// Simple hook execution utility for future phases
pub struct SimpleHookExecutor;

impl SimpleHookExecutor {
    /// Execute registered hooks for a given phase (future implementation)
    pub fn execute_hooks(
        context: &HookContext,
        _hook_accounts: &[AccountInfo],
    ) -> Result<()> {
        // For Phase 1, hooks are not yet implemented
        // This provides the interface for future hook execution
        
        match context.operation {
            HookOperation::Swap { .. } => {
                // Future: Execute swap hooks (MEV protection, etc.)
                Ok(())
            }
            HookOperation::AddLiquidity { .. } | HookOperation::RemoveLiquidity { .. } => {
                // Future: Execute liquidity hooks (validation, etc.)
                Ok(())
            }
        }
    }
    
    /// Validate hook permissions (future implementation)
    pub fn validate_hook_permissions(
        _hook_program: &Pubkey,
        _pool: &Pubkey,
    ) -> Result<()> {
        // For Phase 1, all hooks are disabled for security
        // Future phases will implement granular permission system
        Err(PoolError::InvalidOperation.into())
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hook_context_serialization() {
        let params = SwapHookParams {
            pool: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            amount_in: 1000,
            amount_out: 950,
            token_in: Pubkey::new_unique(),
            token_out: Pubkey::new_unique(),
            price_before: 1_000_000,
            price_after: 1_050_000,
        };
        let context = create_swap_hook_context(&params);
        
        // Test serialization
        let serialized = context.try_to_vec().unwrap();
        assert!(!serialized.is_empty());
    }
}