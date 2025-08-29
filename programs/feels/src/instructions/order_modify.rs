/// Modify an existing 3D order by adjusting parameters across dimensions.
/// This replaces legacy leverage_add_liquidity and leverage_remove_liquidity with unified handling.
/// Modifications include adjusting leverage, changing duration commitment, adding/removing liquidity,
/// and updating rate limits for limit orders with proper margin requirements.
use anchor_lang::prelude::*;
use std::collections::BTreeMap;
use crate::{execute_hooks, execute_post_hooks};
use crate::logic::event::{OrderModifyEvent};
use crate::logic::hook::{HookContextBuilder, EVENT_ORDER_MODIFIED, EVENT_LIQUIDITY_CHANGED};
use crate::state::{FeelsProtocolError, RiskProfile, Tick3D};
use crate::state::duration::Duration;
use crate::state::reentrancy::{ReentrancyGuard, ReentrancyStatus};
use crate::logic::order::{SecureOrderManager, get_oracle_from_remaining};
use crate::utils::cpi_helpers::{transfer_from_user_to_pool, transfer_from_pool_to_user};

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
        ctx.accounts.order_owner.key() == ctx.accounts.user.key(),
        FeelsProtocolError::UnauthorizedOrderModification
    );
    
    // 1.2 Load order metadata
    let mut order_metadata = ctx.accounts.order_metadata.load_mut()?;
    require!(
        order_metadata.order_id == params.order_id,
        FeelsProtocolError::InvalidOrderId
    );
    require!(
        !order_metadata.is_closed,
        FeelsProtocolError::OrderAlreadyClosed
    );
    
    // 1.3 Acquire reentrancy lock
    let current_status = pool.get_reentrancy_status()?;
    require!(
        current_status == ReentrancyStatus::Unlocked,
        FeelsProtocolError::ReentrancyDetected
    );
    pool.set_reentrancy_status(ReentrancyStatus::Locked)?
    
    // 1.4 Get secure oracle if available
    let oracle = pool.oracle.and_then(|oracle_pubkey| {
        get_oracle_from_remaining(ctx.remaining_accounts, &oracle_pubkey)
    });
    
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
                &mut order_metadata,
                new_leverage,
                &params.new_params,
                oracle.as_ref(),
            )?
        },
        OrderModification::ChangeDuration { new_duration } => {
            change_duration(
                &mut pool,
                &mut order_metadata,
                new_duration,
                &params.new_params,
            )?
        },
        OrderModification::AddLiquidity { additional_amount } => {
            add_liquidity(
                &mut pool,
                &mut order_metadata,
                additional_amount,
                &params.new_params,
                ctx.accounts,
            )?
        },
        OrderModification::RemoveLiquidity { amount_to_remove } => {
            remove_liquidity(
                &mut pool,
                &mut order_metadata,
                amount_to_remove,
                &params.new_params,
                ctx.accounts,
            )?
        },
        OrderModification::UpdateLimit { new_rate_limit } => {
            update_limit_order(
                &mut pool,
                &mut order_metadata,
                new_rate_limit,
                &params.new_params,
            )?
        },
    };
    
    // ========================================================================
    // PHASE 3: FINALIZATION
    // ========================================================================
    
    // 3.1 Update order metadata
    order_metadata.last_modified = clock.unix_timestamp;
    order_metadata.modification_count += 1;
    
    // 3.2 Update pool state
    pool.last_update_slot = clock.slot;
    
    // 3.3 Build hook context
    let hook_context = build_modify_hook_context(
        &ctx,
        &params,
        &result,
        &order_metadata,
    );
    
    // 3.4 Release reentrancy lock for hooks
    if ctx.accounts.hook_registry.is_some() {
        pool.set_reentrancy_status(ReentrancyStatus::HookExecuting)?;
    }
    
    // 3.5 Save state before external calls
    drop(pool);
    drop(order_metadata);
    
    // 3.6 Execute hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_hooks!(
            Some(registry),
            None,
            EVENT_ORDER_MODIFIED,
            hook_context.clone(),
            ctx.remaining_accounts
        );
    }
    
    // 3.7 Execute any required transfers
    execute_modification_transfers(&ctx, &params, &result)?;
    
    // 3.8 Execute post-hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_post_hooks!(
            Some(registry),
            ctx.accounts.hook_message_queue.as_mut(),
            EVENT_ORDER_MODIFIED,
            hook_context,
            ctx.remaining_accounts
        );
    }
    
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
    order_metadata: &mut OrderMetadata,
    new_leverage: u64,
    params: &ModificationParams,
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
    
    let old_leverage = order_metadata.leverage;
    let old_risk_profile = RiskProfile::from_leverage(old_leverage, pool)?;
    let new_risk_profile = RiskProfile::from_leverage(new_leverage, pool)?;
    
    // Calculate margin requirements
    let position_value = order_metadata.locked_amount;
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
    
    // Update order metadata
    order_metadata.leverage = new_leverage;
    order_metadata.tick_3d = Tick3D {
        rate_tick: order_metadata.tick_3d.rate_tick,
        duration_tick: order_metadata.tick_3d.duration_tick,
        leverage_tick: new_risk_profile.to_tick(),
    };
    
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
    order_metadata: &mut OrderMetadata,
    new_duration: Duration,
    params: &ModificationParams,
) -> Result<ModifyOrderResult> {
    let old_duration = order_metadata.duration;
    
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
            let time_elapsed = Clock::get()?.unix_timestamp - order_metadata.created_at;
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
        order_metadata.locked_amount,
        &old_duration,
        &new_duration,
        pool,
    )?;
    
    // Update order metadata
    order_metadata.duration = new_duration;
    order_metadata.tick_3d = Tick3D {
        rate_tick: order_metadata.tick_3d.rate_tick,
        duration_tick: new_duration.to_tick(),
        leverage_tick: order_metadata.tick_3d.leverage_tick,
    };
    
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
fn add_liquidity(
    pool: &mut Pool,
    order_metadata: &mut OrderMetadata,
    additional_amount: u64,
    params: &ModificationParams,
    accounts: &crate::OrderModify,
) -> Result<ModifyOrderResult> {
    require!(additional_amount > 0, FeelsProtocolError::InvalidAmount);
    
    // For liquidity orders only
    require!(
        order_metadata.order_type == OrderType::Liquidity,
        FeelsProtocolError::NotLiquidityOrder
    );
    
    // Calculate additional liquidity with leverage
    let risk_profile = RiskProfile::from_leverage(order_metadata.leverage, pool)?;
    let additional_liquidity = (additional_amount as u128)
        .checked_mul(order_metadata.leverage as u128)
        .and_then(|l| l.checked_div(RiskProfile::LEVERAGE_SCALE as u128))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Update tick liquidity
    let tick_manager = crate::logic::tick::TickManager;
    tick_manager.update_tick(
        pool,
        order_metadata.tick_lower,
        additional_liquidity as i128,
        false,
        &[], // Tick arrays passed in remaining_accounts
    )?;
    tick_manager.update_tick(
        pool,
        order_metadata.tick_upper,
        -(additional_liquidity as i128),
        false,
        &[],
    )?;
    
    // Update pool liquidity if in range
    if pool.current_tick >= order_metadata.tick_lower && 
       pool.current_tick < order_metadata.tick_upper {
        pool.liquidity = pool.liquidity
            .checked_add(additional_liquidity)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    // Update order metadata
    let old_amount = order_metadata.locked_amount;
    order_metadata.locked_amount = old_amount
        .checked_add(additional_amount)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    order_metadata.liquidity = order_metadata.liquidity
        .checked_add(additional_liquidity)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    Ok(ModifyOrderResult {
        old_value: old_amount,
        new_value: order_metadata.locked_amount,
        margin_delta: Some(MarginDelta::Required(additional_amount)),
        liquidity_delta: Some(additional_liquidity as i128),
    })
}

/// Remove liquidity from position
fn remove_liquidity(
    pool: &mut Pool,
    order_metadata: &mut OrderMetadata,
    amount_to_remove: u64,
    params: &ModificationParams,
    accounts: &crate::OrderModify,
) -> Result<ModifyOrderResult> {
    require!(amount_to_remove > 0, FeelsProtocolError::InvalidAmount);
    require!(
        amount_to_remove <= order_metadata.locked_amount,
        FeelsProtocolError::InsufficientLiquidity
    );
    
    // For liquidity orders only
    require!(
        order_metadata.order_type == OrderType::Liquidity,
        FeelsProtocolError::NotLiquidityOrder
    );
    
    // Calculate liquidity to remove
    let liquidity_ratio = (amount_to_remove as u128)
        .checked_mul(u128::MAX)
        .and_then(|n| n.checked_div(order_metadata.locked_amount as u128))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    let liquidity_to_remove = (order_metadata.liquidity)
        .checked_mul(liquidity_ratio)
        .and_then(|l| l.checked_div(u128::MAX))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Update tick liquidity
    let tick_manager = crate::logic::tick::TickManager;
    tick_manager.update_tick(
        pool,
        order_metadata.tick_lower,
        -(liquidity_to_remove as i128),
        false,
        &[],
    )?;
    tick_manager.update_tick(
        pool,
        order_metadata.tick_upper,
        liquidity_to_remove as i128,
        false,
        &[],
    )?;
    
    // Update pool liquidity if in range
    if pool.current_tick >= order_metadata.tick_lower && 
       pool.current_tick < order_metadata.tick_upper {
        pool.liquidity = pool.liquidity
            .checked_sub(liquidity_to_remove)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
    }
    
    // Collect any accumulated fees
    let fees_collected = collect_position_fees(
        pool,
        order_metadata,
        liquidity_ratio,
    )?;
    
    // Update order metadata
    let old_amount = order_metadata.locked_amount;
    order_metadata.locked_amount = old_amount
        .checked_sub(amount_to_remove)
        .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
    order_metadata.liquidity = order_metadata.liquidity
        .checked_sub(liquidity_to_remove)
        .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
    
    Ok(ModifyOrderResult {
        old_value: old_amount,
        new_value: order_metadata.locked_amount,
        margin_delta: Some(MarginDelta::Releasable(amount_to_remove + fees_collected)),
        liquidity_delta: Some(-(liquidity_to_remove as i128)),
    })
}

/// Update limit order parameters
fn update_limit_order(
    pool: &mut Pool,
    order_metadata: &mut OrderMetadata,
    new_rate_limit: u128,
    params: &ModificationParams,
) -> Result<ModifyOrderResult> {
    // For limit orders only
    require!(
        order_metadata.order_type == OrderType::Limit,
        FeelsProtocolError::NotLimitOrder
    );
    
    // Validate new rate limit
    require!(
        new_rate_limit > 0,
        FeelsProtocolError::InvalidRateLimit
    );
    
    let old_rate_limit = order_metadata.rate_limit;
    let new_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(new_rate_limit)?;
    
    // Update order metadata
    order_metadata.rate_limit = new_rate_limit;
    order_metadata.tick_3d = Tick3D {
        rate_tick: new_tick,
        duration_tick: order_metadata.tick_3d.duration_tick,
        leverage_tick: order_metadata.tick_3d.leverage_tick,
    };
    
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
        .unwrap_or(position_value) as u64
}

/// Validate leverage adjustment against oracle
fn validate_leverage_adjustment(
    pool: &Pool,
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
    let old_fee = (base_fee as i64 * old_multiplier / 10000);
    let new_fee = (base_fee as i64 * new_multiplier / 10000);
    
    Ok(new_fee - old_fee)
}

/// Collect accumulated fees for a position
fn collect_position_fees(
    pool: &Pool,
    order_metadata: &OrderMetadata,
    liquidity_ratio: u128,
) -> Result<u64> {
    // Calculate fees based on position
    let estimated_fees = (order_metadata.locked_amount as u128)
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
    order_metadata: &OrderMetadata,
) -> crate::logic::hook::HookContext {
    let mut context = HookContextBuilder::base(
        ctx.accounts.pool.key(),
        ctx.accounts.user.key(),
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
    params: &OrderModifyParams,
    result: &ModifyOrderResult,
) -> Result<()> {
    if let Some(margin_delta) = &result.margin_delta {
        match margin_delta {
            MarginDelta::Required(amount) => {
                // Transfer additional margin from user to pool
                transfer_from_user_to_pool(
                    ctx.accounts.user_token_account.to_account_info(),
                    ctx.accounts.pool_token_account.to_account_info(),
                    ctx.accounts.user.to_account_info(),
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
                
                transfer_from_pool_to_user(
                    ctx.accounts.pool_token_account.to_account_info(),
                    ctx.accounts.user_token_account.to_account_info(),
                    ctx.accounts.pool.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    *amount,
                    &[
                        b"pool",
                        pool.token_a_mint.as_ref(),
                        pool.token_b_mint.as_ref(),
                        &pool.fee_rate.to_le_bytes(),
                        &[pool_bump],
                    ],
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

// Placeholder types - should be defined in state module
#[account]
#[derive(Default)]
pub struct OrderMetadata {
    pub order_id: Pubkey,
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub order_type: OrderType,
    pub tick_3d: Tick3D,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub locked_amount: u64,
    pub liquidity: u128,
    pub leverage: u64,
    pub duration: Duration,
    pub rate_limit: u128,
    pub created_at: i64,
    pub last_modified: i64,
    pub modification_count: u32,
    pub is_closed: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum OrderType {
    Liquidity,
    Limit,
}

// Re-export Pool type
use crate::state::Pool;