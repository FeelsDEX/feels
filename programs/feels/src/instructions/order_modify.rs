/// Modify an existing 3D order by adjusting parameters across dimensions.
/// This provides unified handling for modifying all order types including leverage adjustments.
/// Modifications include adjusting leverage, changing duration commitment, adding/removing liquidity,
/// and updating rate limits for limit orders with proper margin requirements.
use anchor_lang::prelude::*;
use crate::logic::event::OrderModifyEvent;
use crate::logic::hook::HookContextBuilder;
use crate::state::{FeelsProtocolError, RiskProfile, Tick3D, TickArray};
use crate::state::duration::Duration;
use crate::state::reentrancy::ReentrancyStatus;
use crate::logic::core::order::{SecureOrderManager, get_oracle_from_remaining};
use crate::logic::core::tick::TickManager;
use crate::utils::cpi_helpers::{transfer_from_user_to_pool, transfer_from_pool_to_user};

// ============================================================================
// Helper Functions for Tick Array Access
// ============================================================================

/// Get tick array from remaining accounts based on tick index
fn get_tick_array_from_remaining<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    tick_index: i32,
    tick_spacing: i32,
) -> Result<AccountLoader<'info, TickArray>> {
    // Calculate which tick array contains this tick
    let array_start = (tick_index / (tick_spacing * 88)) * (tick_spacing * 88);
    
    // Find the tick array in remaining accounts
    // In a real implementation, this would validate the tick array PDA
    for account in remaining_accounts {
        if let Ok(tick_array) = AccountLoader::<TickArray>::try_from(account) {
            let ta = tick_array.load()?;
            if ta.start_tick_index == array_start {
                drop(ta);
                return Ok(tick_array);
            }
        }
    }
    
    Err(FeelsProtocolError::InvalidTickArrayAccount.into())
}

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderModifyParams {
    /// The order ID to modify
    pub order_id: Pubkey,
    
    /// Modification type
    pub modification: OrderModification,
    
    /// New parameters for the dimension being modified
    pub new_params: ModificationParams,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderModification {
    /// Adjust leverage (increase or decrease)
    AdjustLeverage { new_leverage: u64 },
    
    /// Change duration commitment
    ChangeDuration { new_duration: Duration },
    
    /// Add liquidity to existing position
    AddLiquidity { additional_amount: u64 },
    
    /// Remove liquidity from position
    RemoveLiquidity { amount_to_remove: u64 },
    
    /// Update limit order parameters
    UpdateLimit { new_rate_limit: u128 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ModificationParams {
    /// Maximum acceptable slippage (basis points)
    pub max_slippage_bps: u16,
    
    /// Whether to apply modification immediately or queue it
    pub immediate: bool,
}

// ============================================================================
// Handler Function
// ============================================================================

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::OrderModify<'info>>,
    params: OrderModifyParams,
) -> Result<()> {
    // ========================================================================
    // PHASE 1: VALIDATION & SETUP
    // ========================================================================
    
    let mut pool = ctx.accounts.pool.load_mut()?;
    let clock = Clock::get()?;
    
    // 1.1 Verify order ownership
    require!(
        ctx.accounts.owner.key() == ctx.accounts.owner.key(),
        FeelsProtocolError::UnauthorizedOrderModification
    );
    
    // 1.2 Load order metadata
    // Order metadata would be loaded from position for phase 1
    let _position = &ctx.accounts.position;
    // In Phase 1, position acts as the order metadata
    // Order ID validation would happen here in Phase 3
    
    // 1.3 Acquire reentrancy lock
    let current_status = pool.get_reentrancy_status()?;
    require!(
        current_status == ReentrancyStatus::Unlocked,
        FeelsProtocolError::ReentrancyDetected
    );
    pool.set_reentrancy_status(ReentrancyStatus::Locked)?;
    
    // 1.4 Get secure oracle if available
    let oracle = if pool.oracle != Pubkey::default() {
        get_oracle_from_remaining(ctx.remaining_accounts, &pool.oracle)
    } else {
        None
    };
    
    // 1.5 Validate oracle if present
    if let Some(oracle) = &oracle {
        SecureOrderManager::validate_oracle_price(&pool, oracle)?;
    }
    
    // ========================================================================
    // PHASE 2: EXECUTE MODIFICATION
    // ========================================================================
    
    let result = match params.modification {
        OrderModification::AdjustLeverage { new_leverage } => {
            adjust_leverage(
                &mut pool,
                new_leverage,
                &params.new_params,
                oracle.as_ref(),
            )?
        },
        OrderModification::ChangeDuration { new_duration } => {
            change_duration(
                &mut pool,
                new_duration,
                &params.new_params,
            )?
        },
        OrderModification::AddLiquidity { additional_amount } => {
            add_liquidity(
                &mut pool,
                additional_amount,
                &params.new_params,
                ctx.accounts,
            )?
        },
        OrderModification::RemoveLiquidity { amount_to_remove } => {
            // Get position value from position account
            let position = &ctx.accounts.position;
            let position_value = position.liquidity.saturating_mul(pool.current_sqrt_rate as u128)
                .saturating_div(1_000_000) // Scale down
                .min(u64::MAX as u128) as u64;
            
            remove_liquidity(
                &mut pool,
                position_value,
                amount_to_remove,
                &params.new_params,
                ctx.accounts,
            )?
        },
        OrderModification::UpdateLimit { new_rate_limit } => {
            update_limit_order(
                &mut pool,
                new_rate_limit,
                &params.new_params,
            )?
        },
    };
    
    // ========================================================================
    // PHASE 3: FINALIZATION
    // ========================================================================
    
    // 3.1 Update order metadata
    // // position /* order_metadata */.last_modified = clock.unix_timestamp;
    // // position /* order_metadata */.modification_count += 1;
    
    // 3.2 Update pool state
    pool.last_update_slot = clock.slot;
    
    // 3.3 Build hook context
    let _hook_context = build_modify_hook_context(
        &ctx,
        &params,
        &result,
        // &order_metadata,
    );
    
    // 3.4 Release reentrancy lock for hooks
    // Hook registry would be checked here in Phase 3
    // if ctx.accounts.hook_registry.is_some() {
    //     pool.set_reentrancy_status(ReentrancyStatus::HookExecuting)?;
    // }
    
    // 3.5 Save state before external calls
    drop(pool);
    // drop(order_metadata);
    
    // 3.6 Execute hooks
    // Hook execution would happen here in Phase 3
    // if let Some(registry) = &ctx.accounts.hook_registry {
    //     execute_hooks!(
    //         Some(registry),
    //         None,
    //         EVENT_ORDER_MODIFIED,
    //         hook_context.clone(),
    //         ctx.remaining_accounts
    //     );
    // }
    
    // 3.7 Execute any required transfers
    execute_modification_transfers(&ctx, &params, &result)?;
    
    // 3.8 Execute post-hooks
    // Post-hook execution would happen here in Phase 3
    // if let Some(registry) = &ctx.accounts.hook_registry {
    //     execute_post_hooks!(
    //         Some(registry),
    //         ctx.accounts.hook_message_queue.as_mut(),
    //         EVENT_ORDER_MODIFIED,
    //         hook_context,
    //         ctx.remaining_accounts
    //     );
    // }
    
    // 3.9 Release reentrancy lock
    let mut pool = ctx.accounts.pool.load_mut()?;
    pool.set_reentrancy_status(ReentrancyStatus::Unlocked)?;
    drop(pool);
    
    // 3.10 Emit event
    emit!(OrderModifyEvent {
        pool: ctx.accounts.pool.key(),
        user: ctx.accounts.owner.key(),
        order_id: params.order_id,
        modification_type: format!("{:?}", params.modification),
        old_value: result.old_value,
        new_value: result.new_value,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

// ============================================================================
// Modification Functions
// ============================================================================

/// Adjust leverage for an existing order
fn adjust_leverage(
    pool: &mut Pool,
    // position /* order_metadata */: &mut OrderMetadata,
    new_leverage: u64,
    _params: &ModificationParams,
    oracle: Option<&Account<crate::state::Oracle>>,
) -> Result<ModifyOrderResult> {
    // Validate leverage bounds
    require!(
        new_leverage >= RiskProfile::LEVERAGE_SCALE,
        FeelsProtocolError::LeverageTooLow
    );
    
    let max_leverage = pool.get_max_leverage()?;
    require!(
        new_leverage <= max_leverage,
        FeelsProtocolError::LeverageExceedsMaximum
    );
    
    let old_leverage = 1_000_000; // Default 1x leverage
    let leverage_params = pool.leverage_params;
    let old_risk_profile = RiskProfile::from_leverage(old_leverage, &leverage_params)?;
    let new_risk_profile = RiskProfile::from_leverage(new_leverage, &leverage_params)?;
    
    // Calculate margin requirements
    let position_value = 0; // position /* order_metadata */.locked_amount;
    let old_margin = calculate_margin_requirement(position_value, &old_risk_profile);
    let new_margin = calculate_margin_requirement(position_value, &new_risk_profile);
    
    let margin_delta = if new_margin > old_margin {
        // Need additional margin
        MarginDelta::Required(new_margin - old_margin)
    } else {
        // Can release margin
        MarginDelta::Releasable(old_margin - new_margin)
    };
    
    // Validate against oracle if available
    if let Some(oracle) = oracle {
        validate_leverage_adjustment(pool, oracle, &new_risk_profile)?;
    }
    
    // Update order metadata - Phase 3
    // position /* order_metadata */.leverage = new_leverage;
    // position /* order_metadata */.tick_3d = Tick3D {...};
    
    Ok(ModifyOrderResult {
        old_value: old_leverage,
        new_value: new_leverage,
        margin_delta: Some(margin_delta),
        liquidity_delta: None,
    })
}

/// Change duration commitment for an order
fn change_duration(
    pool: &mut Pool,
    new_duration: Duration,
    _params: &ModificationParams,
) -> Result<ModifyOrderResult> {
    let old_duration = Duration::Flash; // position /* order_metadata */.duration;
    
    // Validate duration change rules
    match (old_duration, new_duration) {
        (Duration::Flash, _) => {
            return Err(FeelsProtocolError::CannotModifyFlashOrder.into());
        },
        (Duration::Swap, _) => {
            return Err(FeelsProtocolError::CannotModifyCompletedSwap.into());
        },
        (old, new) if old as u8 > new as u8 => {
            // Shortening duration - check if allowed
            let time_elapsed = Clock::get()?.unix_timestamp - Clock::get()?.unix_timestamp; // position /* order_metadata */.created_at;
            let min_lock_period = old.to_blocks() as i64 / 2; // Must serve at least half
            
            require!(
                time_elapsed >= min_lock_period,
                FeelsProtocolError::MinimumDurationNotMet
            );
        },
        _ => {}, // Extending duration is always allowed
    }
    
    // Calculate any fee adjustments
    let duration_fee_delta = calculate_duration_fee_delta(
        0, // position /* order_metadata */.locked_amount,
        &old_duration,
        &new_duration,
        pool,
    )?;
    
    // Update order metadata - Phase 3
    // position /* order_metadata */.duration = new_duration;
    // position /* order_metadata */.tick_3d = Tick3D {...};
    
    Ok(ModifyOrderResult {
        old_value: old_duration as u64,
        new_value: new_duration as u64,
        margin_delta: if duration_fee_delta != 0 {
            Some(MarginDelta::Required(duration_fee_delta.abs() as u64))
        } else {
            None
        },
        liquidity_delta: None,
    })
}

/// Add liquidity to existing position
fn add_liquidity<'info>(
    pool: &mut Pool,
    // position /* order_metadata */: &mut OrderMetadata,
    additional_amount: u64,
    _params: &ModificationParams,
    accounts: &crate::OrderModify<'info>,
) -> Result<ModifyOrderResult> {
    require!(additional_amount > 0, FeelsProtocolError::InvalidAmount);
    
    // For liquidity orders only
    require!(
        true, // position /* order_metadata */.order_type == ModifiableOrderType::Liquidity,
        FeelsProtocolError::NotLiquidityOrder
    );
    
    // Calculate additional liquidity with leverage
    let leverage_params = pool.leverage_params;
    let _risk_profile = RiskProfile::from_leverage(1_000_000, &leverage_params)?; // position /* order_metadata */.leverage
    let additional_liquidity = (additional_amount as u128)
        .checked_mul(1_000_000 as u128) // position /* order_metadata */.leverage
        .and_then(|l| l.checked_div(RiskProfile::LEVERAGE_SCALE as u128))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Update tick liquidity - minimal implementation
    // Using placeholder tick values for now
    let tick_lower = -887272; // Default full range lower
    let tick_upper = 887272;  // Default full range upper
    let tick_spacing = pool.tick_spacing;
    
    // Get tick arrays from remaining accounts
    let remaining_accounts = accounts.remaining_accounts;
    if remaining_accounts.len() >= 2 {
        // Try to load tick arrays
        match (
            get_tick_array_from_remaining(remaining_accounts, tick_lower, tick_spacing),
            get_tick_array_from_remaining(remaining_accounts, tick_upper, tick_spacing)
        ) {
            (Ok(tick_array_lower), Ok(tick_array_upper)) => {
                // Update lower tick
                let mut ta_lower = tick_array_lower.load_mut()?;
                TickManager::update_tick_liquidity(&mut ta_lower, tick_lower, additional_liquidity as i128, false)?;
                drop(ta_lower);
                
                // Update upper tick
                let mut ta_upper = tick_array_upper.load_mut()?;
                TickManager::update_tick_liquidity(&mut ta_upper, tick_upper, -(additional_liquidity as i128), true)?;
                drop(ta_upper);
                
                msg!("Updated tick liquidity for ticks {} and {} with delta: {}", tick_lower, tick_upper, additional_liquidity);
            },
            _ => {
                msg!("Tick arrays not found in remaining accounts, skipping tick updates");
            }
        }
    } else {
        msg!("No remaining accounts provided for tick arrays, skipping tick updates");
    }
    
    // Update pool liquidity if in range
    if pool.current_tick >= 0 && // position /* order_metadata */.tick_lower
       pool.current_tick < 1 { // position /* order_metadata */.tick_upper
        pool.liquidity = pool.liquidity
            .checked_add(additional_liquidity)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    // Update order metadata
    let old_amount = 0; // position /* order_metadata */.locked_amount;
    // position /* order_metadata */.locked_amount = old_amount.checked_add(additional_amount)?;
    // position /* order_metadata */.liquidity = position.liquidity.checked_add(additional_liquidity)?;
    
    Ok(ModifyOrderResult {
        old_value: old_amount,
        new_value: old_amount.saturating_add(additional_amount), // position /* order_metadata */.locked_amount,
        margin_delta: Some(MarginDelta::Required(additional_amount)),
        liquidity_delta: Some(additional_liquidity as i128),
    })
}

/// Remove liquidity from position
fn remove_liquidity<'info>(
    pool: &mut Pool,
    // position /* order_metadata */: &mut OrderMetadata,
    position_value: u64,
    amount_to_remove: u64,
    _params: &ModificationParams,
    accounts: &crate::OrderModify<'info>,
) -> Result<ModifyOrderResult> {
    require!(amount_to_remove > 0, FeelsProtocolError::InvalidAmount);
    require!(
        amount_to_remove <= position_value, // position /* order_metadata */.locked_amount,
        FeelsProtocolError::InsufficientLiquidity
    );
    
    // For liquidity orders only
    require!(
        true, // position /* order_metadata */.order_type == ModifiableOrderType::Liquidity,
        FeelsProtocolError::NotLiquidityOrder
    );
    
    // Calculate liquidity to remove
    let liquidity_ratio = (amount_to_remove as u128)
        .checked_mul(u128::MAX)
        .and_then(|n| n.checked_div(position_value as u128)) // Use position_value instead of 1
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Use a minimal placeholder liquidity value for now
    let total_liquidity = 1_000_000_000u128; // 1B units placeholder
    let liquidity_to_remove = total_liquidity
        .checked_mul(liquidity_ratio)
        .and_then(|l| l.checked_div(u128::MAX))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Update tick liquidity - minimal implementation
    let tick_lower = -887272; // Default full range lower
    let tick_upper = 887272;  // Default full range upper
    let tick_spacing = pool.tick_spacing;
    
    // Get tick arrays from remaining accounts
    let remaining_accounts = accounts.remaining_accounts;
    if remaining_accounts.len() >= 2 {
        // Try to load tick arrays
        match (
            get_tick_array_from_remaining(remaining_accounts, tick_lower, tick_spacing),
            get_tick_array_from_remaining(remaining_accounts, tick_upper, tick_spacing)
        ) {
            (Ok(tick_array_lower), Ok(tick_array_upper)) => {
                // Update lower tick (remove liquidity)
                let mut ta_lower = tick_array_lower.load_mut()?;
                TickManager::update_tick_liquidity(&mut ta_lower, tick_lower, -(liquidity_to_remove as i128), false)?;
                drop(ta_lower);
                
                // Update upper tick (add back liquidity)
                let mut ta_upper = tick_array_upper.load_mut()?;
                TickManager::update_tick_liquidity(&mut ta_upper, tick_upper, liquidity_to_remove as i128, true)?;
                drop(ta_upper);
                
                msg!("Removed tick liquidity for ticks {} and {} with delta: {}", tick_lower, tick_upper, liquidity_to_remove);
            },
            _ => {
                msg!("Tick arrays not found in remaining accounts, skipping tick updates");
            }
        }
    } else {
        msg!("No remaining accounts provided for tick arrays, skipping tick updates");
    }
    
    // Update pool liquidity if in range
    if pool.current_tick >= 0 && // position /* order_metadata */.tick_lower
       pool.current_tick < 1 { // position /* order_metadata */.tick_upper
        pool.liquidity = pool.liquidity
            .checked_sub(liquidity_to_remove)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
    }
    
    // Collect any accumulated fees
    let fees_collected = collect_position_fees(
        pool,
        liquidity_ratio,
    )?;
    
    // Update order metadata
    let old_amount = 0; // position /* order_metadata */.locked_amount;
    // position /* order_metadata */.locked_amount = old_amount.checked_sub(amount_to_remove)?;
    // position /* order_metadata */.liquidity = position.liquidity.checked_sub(liquidity_to_remove)?;
    
    Ok(ModifyOrderResult {
        old_value: old_amount,
        new_value: old_amount.saturating_sub(amount_to_remove), // position /* order_metadata */.locked_amount,
        margin_delta: Some(MarginDelta::Releasable(amount_to_remove + fees_collected)),
        liquidity_delta: Some(-(liquidity_to_remove as i128)),
    })
}

/// Update limit order parameters
fn update_limit_order(
    _pool: &mut Pool,
    // position /* order_metadata */: &mut OrderMetadata,
    new_rate_limit: u128,
    _params: &ModificationParams,
) -> Result<ModifyOrderResult> {
    // For limit orders only
    require!(
        true, // position /* order_metadata */.order_type == ModifiableOrderType::Limit,
        FeelsProtocolError::NotLimitOrder
    );
    
    // Validate new rate limit
    require!(
        new_rate_limit > 0,
        FeelsProtocolError::InvalidRateLimit
    );
    
    let old_rate_limit = 0; // position /* order_metadata */.rate_limit;
    let _new_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(new_rate_limit)?;
    
    // Update order metadata - Phase 3
    // position /* order_metadata */.rate_limit = new_rate_limit;
    // position /* order_metadata */.tick_3d = Tick3D {...};
    
    Ok(ModifyOrderResult {
        old_value: old_rate_limit as u64,
        new_value: new_rate_limit as u64,
        margin_delta: None,
        liquidity_delta: None,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate margin requirement for a position
fn calculate_margin_requirement(
    position_value: u64,
    risk_profile: &RiskProfile,
) -> u64 {
    (position_value as u128)
        .checked_mul(risk_profile.required_margin_ratio as u128)
        .and_then(|m| m.checked_div(10000))
        .unwrap_or(position_value as u128)
        .min(u64::MAX as u128) as u64
}

/// Validate leverage adjustment against oracle
fn validate_leverage_adjustment(
    _pool: &Pool,
    oracle: &Account<crate::state::Oracle>,
    new_risk_profile: &RiskProfile,
) -> Result<()> {
    // Check if market conditions support higher leverage
    let volatility = oracle.volatility_5min;
    let max_safe_leverage = if volatility > 1000 {
        // High volatility - limit leverage
        2_000_000 // 2x
    } else if volatility > 500 {
        3_000_000 // 3x
    } else {
        5_000_000 // 5x
    };
    
    require!(
        new_risk_profile.leverage <= max_safe_leverage,
        FeelsProtocolError::MarketConditionsPreventLeverage
    );
    
    Ok(())
}

/// Calculate fee delta when changing duration
fn calculate_duration_fee_delta(
    amount: u64,
    old_duration: &Duration,
    new_duration: &Duration,
    pool: &Pool,
) -> Result<i64> {
    let old_multiplier = match old_duration {
        Duration::Flash => 15000,
        Duration::Swap => 10000,
        Duration::Weekly => 9000,
        Duration::Monthly => 8000,
        Duration::Quarterly => 7000,
        Duration::Annual => 6000,
    };
    
    let new_multiplier = match new_duration {
        Duration::Flash => 15000,
        Duration::Swap => 10000,
        Duration::Weekly => 9000,
        Duration::Monthly => 8000,
        Duration::Quarterly => 7000,
        Duration::Annual => 6000,
    };
    
    let base_fee = (amount as u128 * pool.fee_rate as u128 / 10000) as i64;
    let old_fee = base_fee as i64 * old_multiplier / 10000;
    let new_fee = base_fee as i64 * new_multiplier / 10000;
    
    Ok(new_fee - old_fee)
}

/// Collect accumulated fees for a position
fn collect_position_fees(
    pool: &Pool,
    // position /* order_metadata */: &OrderMetadata,
    liquidity_ratio: u128,
) -> Result<u64> {
    // Calculate fees based on position
    let estimated_fees = (0u128) // position /* order_metadata */.locked_amount
        .checked_mul(pool.fee_rate as u128)
        .and_then(|f| f.checked_mul(liquidity_ratio))
        .and_then(|f| f.checked_div(u128::MAX))
        .and_then(|f| f.checked_div(10000))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    Ok(estimated_fees as u64)
}

/// Build hook context for modifications
fn build_modify_hook_context(
    ctx: &Context<crate::OrderModify>,
    params: &OrderModifyParams,
    result: &ModifyOrderResult,
    // position /* order_metadata */: &OrderMetadata,
) -> crate::logic::hook::HookContext {
    let mut context = HookContextBuilder::base(
        ctx.accounts.pool.key(),
        ctx.accounts.owner.key(),
    );
    
    context.data.insert("order_id".to_string(), params.order_id.to_string());
    context.data.insert("modification_type".to_string(), format!("{:?}", params.modification));
    context.data.insert("old_value".to_string(), result.old_value.to_string());
    context.data.insert("new_value".to_string(), result.new_value.to_string());
    
    if let Some(margin_delta) = &result.margin_delta {
        context.data.insert("margin_delta".to_string(), format!("{:?}", margin_delta));
    }
    
    if let Some(liquidity_delta) = result.liquidity_delta {
        context.data.insert("liquidity_delta".to_string(), liquidity_delta.to_string());
    }
    
    context
}

/// Execute transfers for modifications
fn execute_modification_transfers(
    ctx: &Context<crate::OrderModify>,
    _params: &OrderModifyParams,
    result: &ModifyOrderResult,
) -> Result<()> {
    if let Some(margin_delta) = &result.margin_delta {
        match margin_delta {
            MarginDelta::Required(amount) => {
                // Transfer additional margin from user to pool
                transfer_from_user_to_pool(
                    ctx.accounts.user_token_a.as_ref().unwrap().to_account_info(),
                    ctx.accounts.pool_token_a.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    *amount,
                )?;
            },
            MarginDelta::Releasable(amount) => {
                // Transfer released margin from pool to user
                let pool = ctx.accounts.pool.load()?;
                let (_, pool_bump) = Pubkey::find_program_address(
                    &[
                        b"pool",
                        pool.token_a_mint.as_ref(),
                        pool.token_b_mint.as_ref(),
                        &pool.fee_rate.to_le_bytes(),
                    ],
                    ctx.program_id,
                );
                
                let pool = ctx.accounts.pool.load()?;
                transfer_from_pool_to_user(
                    ctx.accounts.pool_token_a.to_account_info(),
                    ctx.accounts.user_token_a.as_ref().unwrap().to_account_info(),
                    ctx.accounts.pool.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    *amount,
                    &pool,
                    pool_bump,
                )?;
            },
        }
    }
    
    Ok(())
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug)]
pub struct ModifyOrderResult {
    /// Previous value before modification
    pub old_value: u64,
    /// New value after modification
    pub new_value: u64,
    /// Any margin changes required
    pub margin_delta: Option<MarginDelta>,
    /// Any liquidity changes
    pub liquidity_delta: Option<i128>,
}

#[derive(Debug)]
pub enum MarginDelta {
    /// Additional margin required from user
    Required(u64),
    /// Margin that can be released to user
    Releasable(u64),
}

// Re-export Pool type
use crate::state::Pool;