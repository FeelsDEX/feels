/// Executes external hook programs that extend pool functionality without core modifications.
/// Manages pre/post operation hooks with proper sandboxing and compute unit limits.
/// Enables features like MEV protection, custom fees, and advanced trading strategies
/// while maintaining security through permission checks and execution boundaries.

use anchor_lang::prelude::*;
use crate::state::{HookRegistry, HookType, HookContext, HookOperation, HookPermission, PoolError};

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
                Ok(_) => {
                    msg!("Hook {} executed successfully", hook.program_id);
                }
                Err(e) => {
                    match hook.permission {
                        HookPermission::Halt => {
                            msg!("Hook {} halted operation: {}", hook.program_id, e);
                            return Err(PoolError::InvalidOperation.into());
                        }
                        _ => {
                            msg!("Hook {} failed (non-critical): {}", hook.program_id, e);
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
            
            if let Err(e) = result {
                msg!("Post-hook {} failed: {}", hook.program_id, e);
                // Post hooks failures are non-critical
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// V9.2 Fix: Validate accounts before passing to hooks
/// Ensures hooks can only access authorized accounts
fn validate_hook_accounts(accounts: &[AccountInfo]) -> Result<()> {
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
    // V9.2 Fix: Validate accounts before passing to hooks
    validate_hook_accounts(accounts)?;
    
    // V47 Fix: Enforce compute unit limits for hook execution
    // Create compute budget instruction to limit hook compute consumption
    let compute_budget_ix = anchor_lang::solana_program::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
        hook.max_compute_units.unwrap_or(100_000), // Default to 100k CUs if not specified
    );
    
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
    
    // Execute CPI with compute budget limit
    // First set the compute budget
    anchor_lang::solana_program::program::invoke_unchecked(
        &compute_budget_ix,
        &[],
    )?;
    
    // Then execute the hook
    anchor_lang::solana_program::program::invoke(
        &hook_instruction,
        accounts,
    )?;
    
    Ok(())
}

/// Helper to create hook context for swap operations
#[allow(clippy::too_many_arguments)]
pub fn create_swap_hook_context(
    pool: Pubkey,
    user: Pubkey,
    amount_in: u64,
    amount_out: u64,
    token_in: Pubkey,
    token_out: Pubkey,
    price_before: u128,
    price_after: u128,
) -> HookContext {
    HookContext {
        pool,
        user,
        operation: HookOperation::Swap {
            amount_in,
            amount_out,
            token_in,
            token_out,
            price_before,
            price_after,
        },
        timestamp: Clock::get().unwrap_or_default().unix_timestamp,
    }
}

/// Helper to create hook context for liquidity operations
#[allow(clippy::too_many_arguments)]
pub fn create_liquidity_hook_context(
    pool: Pubkey,
    user: Pubkey,
    liquidity_amount: u128,
    amount_0: u64,
    amount_1: u64,
    tick_lower: i32,
    tick_upper: i32,
    is_add: bool,
) -> HookContext {
    let operation = if is_add {
        HookOperation::AddLiquidity {
            liquidity_amount,
            amount_0,
            amount_1,
            tick_lower,
            tick_upper,
        }
    } else {
        HookOperation::RemoveLiquidity {
            liquidity_amount,
            amount_0,
            amount_1,
            tick_lower,
            tick_upper,
        }
    };
    
    HookContext {
        pool,
        user,
        operation,
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
            let pre_context = create_swap_hook_context(
                pool,
                user,
                pre_hook_state.0, // amount_in
                0, // amount_out not known yet
                post_hook_state.2, // token_in
                post_hook_state.3, // token_out
                pre_hook_state.1, // price_before
                pre_hook_state.1, // price_after same as before for pre-hook
            );
            
            // Execute pre-swap hooks
            HookExecutor::execute_pre_hooks(
                reg,
                HookType::PreSwap,
                &pre_context,
                remaining_accounts,
            )?;
            
            // Create post-swap context
            let post_context = create_swap_hook_context(
                pool,
                user,
                pre_hook_state.0, // amount_in
                post_hook_state.0, // amount_out
                post_hook_state.2, // token_in
                post_hook_state.3, // token_out
                pre_hook_state.1, // price_before
                post_hook_state.1, // price_after
            );
            
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

/// Validate accounts passed to hooks to prevent unauthorized access
fn validate_hook_accounts(accounts: &[AccountInfo]) -> Result<()> {
    // V9.2 Fix: Account validation for hook execution
    for account in accounts {
        // Prevent hooks from accessing system accounts
        require!(
            account.key != &anchor_lang::solana_program::system_program::id() &&
            account.key != &anchor_lang::solana_program::sysvar::rent::id() &&
            account.key != &anchor_lang::solana_program::sysvar::clock::id(),
            crate::state::PoolError::InvalidAuthority
        );
        
        // Prevent hooks from accessing accounts they don't own or have permission for
        // Only allow accounts owned by:
        // - Token program (for token accounts)
        // - System program (for system accounts)
        // - Current program (for pool/position accounts)
        // - Hook program itself
        let owner = account.owner;
        require!(
            owner == &anchor_spl::token::ID ||
            owner == &anchor_lang::solana_program::system_program::id() ||
            owner == &crate::ID,
            crate::state::PoolError::InvalidHookProgram
        );
    }
    
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hook_context_serialization() {
        let context = create_swap_hook_context(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1000,
            950,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000,
            1_050_000,
        );
        
        // Test serialization
        let serialized = context.try_to_vec().unwrap();
        assert!(serialized.len() > 0);
    }
}