/// Protocol configuration instructions for administrative operations including leverage management,
/// hook registration, and other advanced pool parameter updates. These operations typically require
/// pool authority permissions and modify Phase 2 extension parameters.
use anchor_lang::prelude::*;
use crate::logic::event::{HookRegisteredEvent, HookUnregisteredEvent};
use crate::state::{FeelsProtocolError, LeverageParameters, ProtectionCurve, HookRegistry, HookPermission};

// ============================================================================
// Leverage Configuration
// ============================================================================

/// Enable continuous leverage for a pool with configurable protection curves
/// Leverage allows traders to amplify their positions while the protection curve
/// ensures gradual loss mitigation during stress scenarios
pub fn enable_leverage(
    ctx: Context<crate::EnableLeverage>,
    params: crate::EnableLeverageParams,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool.load_mut()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Validate parameters
    require!(
        params.max_leverage >= 1_000_000 && params.max_leverage <= 100_000_000, // 1x to 100x
        FeelsProtocolError::InvalidLeverage
    );
    require!(
        params.initial_ceiling >= 1_000_000 && params.initial_ceiling <= params.max_leverage,
        FeelsProtocolError::InvalidLeverage
    );
    require!(
        params.liquidation_threshold > 0 && params.liquidation_threshold < params.max_leverage,
        FeelsProtocolError::InvalidLeverage
    );

    // Configure leverage parameters
    pool.leverage_params = LeverageParameters {
        max_leverage: params.max_leverage,
        current_ceiling: params.initial_ceiling,
        liquidation_threshold: params.liquidation_threshold,
        protection_curve: params.protection_curve,
        enabled: true,
    };

    pool.last_updated_at = Clock::get()?.unix_timestamp;

    msg!("Leverage enabled for pool");
    msg!("Max leverage: {}x", params.max_leverage / 1_000_000);
    msg!("Initial ceiling: {}x", params.initial_ceiling / 1_000_000);
    msg!("Liquidation threshold: {}x", params.liquidation_threshold / 1_000_000);
    msg!("Protection curve: {:?}", params.protection_curve);

    Ok(())
}

/// Update the leverage ceiling for a pool based on market conditions and risk parameters
/// This allows the pool authority to adjust maximum leverage limits in response to volatility,
/// liquidity depth changes, or systemic risk factors
pub fn update_leverage_ceiling(
    ctx: Context<crate::UpdateLeverageCeiling>,
    new_ceiling: u64,
    update_protection_curve: bool,
    protection_curve: Option<ProtectionCurve>,
) -> Result<()> {
    let mut pool = ctx.accounts.pool.load_mut()?;
    let clock = Clock::get()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Validate leverage is enabled
    require!(
        pool.leverage_params.enabled,
        FeelsProtocolError::LeverageNotEnabled
    );

    // Validate new ceiling is within bounds
    require!(
        new_ceiling >= 1_000_000 && new_ceiling <= pool.leverage_params.max_leverage,
        FeelsProtocolError::InvalidLeverage
    );

    let old_ceiling = pool.leverage_params.current_ceiling;

    // Update leverage ceiling
    pool.leverage_params.current_ceiling = new_ceiling;

    // Update protection curve if requested
    if update_protection_curve {
        if let Some(curve) = protection_curve {
            pool.leverage_params.protection_curve = curve;
            msg!("Protection curve updated: {:?}", curve);
        }
    }

    pool.last_updated_at = clock.unix_timestamp;

    msg!("Leverage ceiling updated");
    msg!("Old ceiling: {}x", old_ceiling / 1_000_000);
    msg!("New ceiling: {}x", new_ceiling / 1_000_000);

    // If ceiling was reduced, emit warning about existing positions
    if new_ceiling < old_ceiling {
        msg!("WARNING: Leverage ceiling reduced - existing positions above new ceiling may need adjustment");
    }

    Ok(())
}

// ============================================================================
// Hook Registration
// ============================================================================

/// Register a new hook for a pool
/// Hooks allow external programs to observe and react to pool events
pub fn register_hook(
    ctx: Context<crate::RegisterHook>,
    params: crate::RegisterHookParams,
) -> Result<()> {
    let registry = &mut ctx.accounts.hook_registry;
    let clock = Clock::get()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == registry.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Validate hook program is not zero
    require!(
        params.hook_program != Pubkey::default(),
        FeelsProtocolError::InvalidAccount
    );

    // Validate event and stage masks are not zero
    require!(
        params.event_mask > 0 && params.stage_mask > 0,
        FeelsProtocolError::InvalidParameter
    );

    // Register the hook
    let hook_index = registry.register_hook(
        params.hook_program,
        params.event_mask,
        params.stage_mask,
        params.permission,
    )?;

    registry.last_update_timestamp = clock.unix_timestamp;

    // Emit hook registration event
    emit!(HookRegisteredEvent {
        pool: registry.pool,
        hook_program: params.hook_program,
        event_mask: params.event_mask,
        stage_mask: params.stage_mask,
        permission: params.permission as u8,
        index: hook_index as u8,
        timestamp: clock.unix_timestamp,
    });

    msg!("Hook registered successfully");
    msg!("Hook program: {}", params.hook_program);
    msg!("Event mask: 0b{:08b}", params.event_mask);
    msg!("Stage mask: 0b{:04b}", params.stage_mask);
    msg!("Permission: {:?}", params.permission);
    msg!("Registry index: {}", hook_index);

    Ok(())
}

/// Unregister a hook from a pool
pub fn unregister_hook(
    ctx: Context<crate::UnregisterHook>,
    hook_index: u8,
) -> Result<()> {
    let registry = &mut ctx.accounts.hook_registry;
    let clock = Clock::get()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == registry.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Validate hook index
    require!(
        hook_index < registry.hook_count,
        FeelsProtocolError::InvalidParameter
    );

    // Get hook info before removal for event
    let hook = registry.hooks[hook_index as usize];

    // Remove hook by swapping with last and decrementing count
    let last_index = registry.hook_count - 1;
    if hook_index < last_index {
        registry.hooks[hook_index as usize] = registry.hooks[last_index as usize];
    }
    
    // Clear the last slot and decrement count
    registry.hooks[last_index as usize] = Default::default();
    registry.hook_count -= 1;
    registry.last_update_timestamp = clock.unix_timestamp;

    // Emit hook unregistration event
    emit!(HookUnregisteredEvent {
        pool: registry.pool,
        hook_program: hook.program_id,
        index: hook_index,
        timestamp: clock.unix_timestamp,
    });

    msg!("Hook unregistered successfully");
    msg!("Hook program: {}", hook.program_id);
    msg!("Index: {}", hook_index);
    msg!("Remaining hooks: {}", registry.hook_count);

    Ok(())
}

/// Initialize hook registry for a pool
pub fn initialize_hook_registry(ctx: Context<crate::InitializeHookRegistry>) -> Result<()> {
    let registry = &mut ctx.accounts.hook_registry;
    let clock = Clock::get()?;
    
    // Initialize registry
    registry.pool = ctx.accounts.pool.key();
    registry.authority = ctx.accounts.authority.key();
    registry.hook_count = 0;
    registry.hooks_enabled = true;
    registry.message_queue_enabled = false;
    registry.emergency_authority = Some(ctx.accounts.authority.key());
    registry.last_update_timestamp = clock.unix_timestamp;

    // Initialize all hook slots as empty
    for i in 0..registry.hooks.len() {
        registry.hooks[i] = Default::default();
    }

    msg!("Hook registry initialized");
    msg!("Pool: {}", registry.pool);
    msg!("Authority: {}", registry.authority);
    
    Ok(())
}

/// Toggle hooks enabled/disabled for a pool
pub fn toggle_hooks(
    ctx: Context<crate::ToggleHooks>,
    enabled: bool,
) -> Result<()> {
    let registry = &mut ctx.accounts.hook_registry;
    let clock = Clock::get()?;

    // Validate authority (allow emergency authority for disabling)
    let authorized = ctx.accounts.authority.key() == registry.authority ||
        (registry.emergency_authority.is_some() && 
         ctx.accounts.authority.key() == registry.emergency_authority.unwrap());
    
    require!(authorized, FeelsProtocolError::InvalidAuthority);

    registry.hooks_enabled = enabled;
    registry.last_update_timestamp = clock.unix_timestamp;

    msg!("Hooks {}", if enabled { "enabled" } else { "disabled" });

    Ok(())
}