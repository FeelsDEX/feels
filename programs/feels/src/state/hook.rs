/// Manages external hook programs that can execute custom logic during pool operations.
/// Enables composability by allowing approved programs to run before/after swaps and
/// liquidity operations. Foundation for advanced features like MEV protection, dynamic
/// fees, and custom pool behaviors. Hooks are permissioned to ensure security.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use solana_program::{bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable};

// ============================================================================
// Constants
// ============================================================================

pub const MAX_HOOKS_PER_TYPE: usize = 4;

// ============================================================================
// Type Definitions
// ============================================================================

/// Hook types supported by the protocol
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum HookType {
    PreSwap,
    PostSwap,
    PreAddLiquidity,
    PostAddLiquidity,
    PreRemoveLiquidity,
    PostRemoveLiquidity,
}

/// Hook permission levels
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum HookPermission {
    /// Hook can only read state
    ReadOnly,
    /// Hook can modify non-critical state
    Modify,
    /// Hook can halt operations (circuit breaker)
    Halt,
}

/// Individual hook configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct HookConfig {
    /// Hook program address
    pub program_id: Pubkey,
    /// Permission level
    pub permission: HookPermission,
    /// Whether hook is currently active
    pub enabled: bool,
    /// Maximum compute units allowed
    pub max_compute_units: u32,
    /// Number of times this hook has been called
    pub call_count: u64,
    /// Last error timestamp (0 if no errors)
    pub last_error_timestamp: i64,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            program_id: Pubkey::default(),
            permission: HookPermission::ReadOnly,
            enabled: false,
            max_compute_units: 100_000,
            call_count: 0,
            last_error_timestamp: 0,
        }
    }
}

// ============================================================================
// Hook Registry Structure
// ============================================================================

/// Hook registry for a pool
#[account]
pub struct HookRegistry {
    /// Pool this registry belongs to
    pub pool: Pubkey,
    
    /// Authority who can modify hooks
    pub authority: Pubkey,
    
    /// Pre-swap hooks
    pub pre_swap_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Post-swap hooks
    pub post_swap_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Pre-add liquidity hooks
    pub pre_add_liquidity_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Post-add liquidity hooks
    pub post_add_liquidity_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Pre-remove liquidity hooks
    pub pre_remove_liquidity_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Post-remove liquidity hooks
    pub post_remove_liquidity_hooks: [HookConfig; MAX_HOOKS_PER_TYPE],
    
    /// Global hook enable/disable
    pub hooks_enabled: bool,
    
    /// Emergency pause authority (can disable all hooks)
    pub emergency_authority: Option<Pubkey>,
    
    /// Last update timestamp
    pub last_update_timestamp: i64,
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

impl HookRegistry {
    pub const SIZE: usize = 8 + // discriminator
        32 + // pool
        32 + // authority
        (32 + 1 + 1 + 4 + 8 + 8) * MAX_HOOKS_PER_TYPE * 6 + // all hook arrays
        1 + // hooks_enabled
        33 + // emergency_authority (Option<Pubkey>)
        8 + // last_update_timestamp
        128; // reserved
    
    /// Get hooks for a specific type
    pub fn get_hooks(&self, hook_type: HookType) -> &[HookConfig; MAX_HOOKS_PER_TYPE] {
        match hook_type {
            HookType::PreSwap => &self.pre_swap_hooks,
            HookType::PostSwap => &self.post_swap_hooks,
            HookType::PreAddLiquidity => &self.pre_add_liquidity_hooks,
            HookType::PostAddLiquidity => &self.post_add_liquidity_hooks,
            HookType::PreRemoveLiquidity => &self.pre_remove_liquidity_hooks,
            HookType::PostRemoveLiquidity => &self.post_remove_liquidity_hooks,
        }
    }
    
    /// Get mutable hooks for a specific type
    pub fn get_hooks_mut(&mut self, hook_type: HookType) -> &mut [HookConfig; MAX_HOOKS_PER_TYPE] {
        match hook_type {
            HookType::PreSwap => &mut self.pre_swap_hooks,
            HookType::PostSwap => &mut self.post_swap_hooks,
            HookType::PreAddLiquidity => &mut self.pre_add_liquidity_hooks,
            HookType::PostAddLiquidity => &mut self.post_add_liquidity_hooks,
            HookType::PreRemoveLiquidity => &mut self.pre_remove_liquidity_hooks,
            HookType::PostRemoveLiquidity => &mut self.post_remove_liquidity_hooks,
        }
    }
    
    /// Register a new hook
    pub fn register_hook(
        &mut self,
        hook_type: HookType,
        program_id: Pubkey,
        permission: HookPermission,
    ) -> Result<usize> {
        let hooks = self.get_hooks_mut(hook_type);
        
        // Find first available slot
        for (index, hook) in hooks.iter_mut().enumerate() {
            if hook.program_id == Pubkey::default() {
                *hook = HookConfig {
                    program_id,
                    permission,
                    enabled: true,
                    max_compute_units: 100_000,
                    call_count: 0,
                    last_error_timestamp: 0,
                };
                return Ok(index);
            }
        }
        
        Err(PoolError::TransientUpdatesFull.into())
    }
    
    /// Unregister a hook
    pub fn unregister_hook(
        &mut self,
        hook_type: HookType,
        program_id: Pubkey,
    ) -> Result<()> {
        let hooks = self.get_hooks_mut(hook_type);
        
        for hook in hooks.iter_mut() {
            if hook.program_id == program_id {
                *hook = HookConfig::default();
                return Ok(());
            }
        }
        
        Err(PoolError::InvalidOperation.into())
    }
    
    /// Get active hooks for execution
    pub fn get_active_hooks(&self, hook_type: HookType) -> Vec<&HookConfig> {
        if !self.hooks_enabled {
            return vec![];
        }
        
        self.get_hooks(hook_type)
            .iter()
            .filter(|h| h.enabled && h.program_id != Pubkey::default())
            .collect()
    }
}

/// Hook execution context passed to hooks
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct HookContext {
    /// Pool being operated on
    pub pool: Pubkey,
    /// User performing the operation
    pub user: Pubkey,
    /// Operation type
    pub operation: HookOperation,
    /// Timestamp
    pub timestamp: i64,
}

/// Operation details for hooks
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum HookOperation {
    Swap {
        amount_in: u64,
        amount_out: u64,
        token_in: Pubkey,
        token_out: Pubkey,
        price_before: u128,
        price_after: u128,
    },
    AddLiquidity {
        liquidity_amount: u128,
        amount_0: u64,
        amount_1: u64,
        tick_lower: i32,
        tick_upper: i32,
    },
    RemoveLiquidity {
        liquidity_amount: u128,
        amount_0: u64,
        amount_1: u64,
        tick_lower: i32,
        tick_upper: i32,
    },
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize hook registry for a pool
pub fn initialize_hook_registry(
    ctx: Context<InitializeHookRegistry>,
) -> Result<()> {
    let registry = &mut ctx.accounts.registry;
    let clock = Clock::get()?;
    
    registry.pool = ctx.accounts.pool.key();
    registry.authority = ctx.accounts.authority.key();
    registry.pre_swap_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.post_swap_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.pre_add_liquidity_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.post_add_liquidity_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.pre_remove_liquidity_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.post_remove_liquidity_hooks = core::array::from_fn(|_| HookConfig::default());
    registry.hooks_enabled = true;
    registry.emergency_authority = Some(ctx.accounts.authority.key());
    registry.last_update_timestamp = clock.unix_timestamp;
    registry._reserved = [0; 128];
    
    emit!(HookRegistryInitializedEvent {
        pool: ctx.accounts.pool.key(),
        registry: ctx.accounts.registry.key(),
        authority: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Register a new hook
pub fn register_hook(
    ctx: Context<RegisterHook>,
    hook_type: HookType,
    permission: HookPermission,
) -> Result<()> {
    let registry = &mut ctx.accounts.registry;
    let clock = Clock::get()?;
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == registry.authority,
        PoolError::InvalidAuthority
    );
    
    // Additional validation for critical permissions
    if permission == HookPermission::Halt {
        // Halt permission requires emergency authority or protocol-level authority
        let emergency_authorized = registry.emergency_authority
            .map(|emergency_auth| emergency_auth == ctx.accounts.authority.key())
            .unwrap_or(false);
        
        require!(
            emergency_authorized,
            PoolError::UnauthorizedGuardian
        );
    }
    
    // Register the hook
    let index = registry.register_hook(
        hook_type,
        ctx.accounts.hook_program.key(),
        permission,
    )?;
    
    registry.last_update_timestamp = clock.unix_timestamp;
    
    emit!(HookRegisteredEvent {
        pool: registry.pool,
        hook_program: ctx.accounts.hook_program.key(),
        hook_type,
        permission,
        index: index as u8,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Emergency pause all hooks
pub fn emergency_pause_hooks(
    ctx: Context<EmergencyPauseHooks>,
) -> Result<()> {
    let registry = &mut ctx.accounts.registry;
    
    // Validate emergency authority
    require!(
        Some(ctx.accounts.authority.key()) == registry.emergency_authority,
        PoolError::InvalidAuthority
    );
    
    registry.hooks_enabled = false;
    
    emit!(HooksEmergencyPausedEvent {
        pool: registry.pool,
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Account Structures
// ============================================================================

#[derive(Accounts)]
pub struct InitializeHookRegistry<'info> {
    #[account(
        init,
        payer = authority,
        space = HookRegistry::SIZE,
        seeds = [b"hook_registry", pool.key().as_ref()],
        bump
    )]
    pub registry: Account<'info, HookRegistry>,
    
    pub pool: AccountLoader<'info, crate::state::Pool>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterHook<'info> {
    #[account(
        mut,
        seeds = [b"hook_registry", registry.pool.as_ref()],
        bump
    )]
    pub registry: Account<'info, HookRegistry>,
    
    pub authority: Signer<'info>,
    
    /// Hook program to register - must be a valid program account
    #[account(
        constraint = hook_program.executable @ PoolError::InvalidHookProgram,
        constraint = hook_program.owner == &bpf_loader::id() || 
                     hook_program.owner == &bpf_loader_deprecated::id() ||
                     hook_program.owner == &bpf_loader_upgradeable::id() @ PoolError::InvalidHookProgram
    )]
    pub hook_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct EmergencyPauseHooks<'info> {
    #[account(
        mut,
        seeds = [b"hook_registry", registry.pool.as_ref()],
        bump
    )]
    pub registry: Account<'info, HookRegistry>,
    
    pub authority: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct HookRegistryInitializedEvent {
    pub pool: Pubkey,
    pub registry: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct HookRegisteredEvent {
    pub pool: Pubkey,
    pub hook_program: Pubkey,
    pub hook_type: HookType,
    pub permission: HookPermission,
    pub index: u8,
    pub timestamp: i64,
}

#[event]
pub struct HooksEmergencyPausedEvent {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}