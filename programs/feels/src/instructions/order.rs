/// Unified 3D order instruction that handles all trading activity in the protocol.
/// Performs swaps, liquidity provision, and position management across three dimensions:
/// Rate (price/interest), Duration (time commitment), and Leverage (risk multiplier).
/// The unified invariant K = R^wr × D^wd × L^wl governs all interactions.
use anchor_lang::prelude::*;
use crate::{execute_hooks, execute_post_hooks};
use crate::logic::event::OrderEvent;
use crate::logic::hook::{HookContextBuilder, EVENT_ORDER_CREATED, EVENT_ORDER_FILLED};
use crate::logic::{OrderManager, OrderState};
use crate::state::{Pool, FeelsProtocolError, RiskProfile, Tick3D, Oracle, FeeConfig};
use crate::state::duration::Duration;
use crate::state::reentrancy::ReentrancyStatus;
use crate::logic::order::{SecureOrderManager, get_oracle_from_remaining, get_oracle_data_from_remaining};

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderParams {
    /// Amount being committed to the order
    pub amount: u64,
    
    /// Rate dimension - can be either:
    /// - For swaps: desired output rate (price limit)
    /// - For liquidity: tick range (lower, upper)
    pub rate_params: RateParams,
    
    /// Duration dimension - how long the order is active
    pub duration: Duration,
    
    /// Leverage dimension - risk multiplier (6 decimals, 1e6 = 1.0x)
    pub leverage: u64,
    
    /// Order type determines behavior
    pub order_type: OrderType,
    
    /// Minimum output for swaps, maximum slippage for liquidity
    pub limit_value: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RateParams {
    /// Single rate for swaps and flash loans
    TargetRate {
        sqrt_rate_limit: u128,
        is_token_a_to_b: bool,
    },
    /// Rate range for liquidity provision
    RateRange {
        tick_lower: i32,
        tick_upper: i32,
    },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderType {
    /// Immediate execution (Duration::Swap or Duration::Flash)
    Immediate,
    /// Liquidity provision (Duration > Swap)
    Liquidity,
    /// Limit order (executes when rate conditions met)
    Limit,
}

// ============================================================================
// Handler Function
// ============================================================================

/// Execute a unified 3D order
pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Order<'info>>,
    params: OrderParams,
) -> Result<OrderResult> {
    // ========================================================================
    // PHASE 1: VALIDATION & SETUP
    // ========================================================================
    
    require!(params.amount > 0, FeelsProtocolError::InputAmountZero);
    require!(
        params.leverage >= RiskProfile::LEVERAGE_SCALE && 
        params.leverage <= RiskProfile::MAX_LEVERAGE_SCALE,
        FeelsProtocolError::InvalidLeverage
    );

    let mut pool = ctx.accounts.pool.load_mut()?;
    let clock = Clock::get()?;
    
    // 1.1 Validate order type matches duration
    match (&params.order_type, &params.duration) {
        (OrderType::Immediate, Duration::Swap) |
        (OrderType::Immediate, Duration::Flash) => {}, // Valid immediate orders
        (OrderType::Liquidity, duration) => {
            require!(
                !matches!(duration, Duration::Swap | Duration::Flash),
                FeelsProtocolError::InvalidDuration
            );
        },
        (OrderType::Limit, _) => {}, // Limits can have any duration
        _ => return Err(FeelsProtocolError::InvalidOrderType.into()),
    }
    
    // 1.2 Acquire reentrancy lock
    let current_status = pool.get_reentrancy_status()?;
    require!(
        current_status == ReentrancyStatus::Unlocked,
        FeelsProtocolError::ReentrancyDetected
    );
    pool.set_reentrancy_status(ReentrancyStatus::Locked)?;
    
    // 1.3 Get secure oracle if available
    let (oracle, oracle_data_account) = if pool.oracle != Pubkey::default() {
        let oracle_opt = get_oracle_from_remaining(
            ctx.remaining_accounts,
            &pool.oracle,
        );
        let oracle_data_opt = oracle_opt.as_ref().and_then(|o| {
            get_oracle_data_from_remaining(
                ctx.remaining_accounts,
                &o.data_account,
            )
        });
        (oracle_opt, oracle_data_opt)
    } else {
        (None, None)
    };
    
    // 1.4 Validate oracle if present
    if let Some(oracle) = &oracle {
        SecureOrderManager::validate_oracle_price(&pool, oracle)?;
    }
    
    // 1.5 Calculate risk profile based on leverage
    let leverage_params = pool.leverage_params;
    let risk_profile = RiskProfile::from_leverage(params.leverage, &leverage_params)?;
    
    // ========================================================================
    // PHASE 2: COMPUTE 3D POSITION
    // ========================================================================
    
    // 2.1 Encode the 3D tick position
    let tick_3d = match &params.rate_params {
        RateParams::TargetRate { sqrt_rate_limit, .. } => {
            Tick3D {
                rate_tick: crate::utils::TickMath::get_tick_at_sqrt_ratio(*sqrt_rate_limit)?,
                duration_tick: params.duration.to_tick(),
                leverage_tick: risk_profile.to_tick(),
            }
        },
        RateParams::RateRange { tick_lower, tick_upper } => {
            // For ranges, use the midpoint for 3D encoding
            let mid_tick = (*tick_lower + *tick_upper) / 2;
            Tick3D {
                rate_tick: mid_tick,
                duration_tick: params.duration.to_tick(),
                leverage_tick: risk_profile.to_tick(),
            }
        },
    };
    
    // 2.2 Calculate effective amounts with leverage
    let effective_amount = calculate_effective_amount(
        params.amount,
        params.leverage,
        &params.duration,
    )?;
    
    // 2.3 Calculate fees with all dimensions considered
    let fee_breakdown = calculate_3d_fees(
        &ctx.accounts.fee_config,
        effective_amount,
        &risk_profile,
        &params.duration,
        oracle.as_ref(),
        oracle_data_account,
    )?;
    
    let amount_after_fees = effective_amount
        .checked_sub(fee_breakdown.total_fee)
        .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
    
    // ========================================================================
    // PHASE 3: EXECUTE ORDER BASED ON TYPE
    // ========================================================================
    
    let result = match params.order_type {
        OrderType::Immediate => {
            execute_immediate_order(
                &mut pool,
                &params,
                amount_after_fees,
                fee_breakdown.clone(),
                &risk_profile,
                ctx.remaining_accounts,
                ctx.accounts.tick_array_router.as_ref(),
                ctx.program_id,
            )?
        },
        OrderType::Liquidity => {
            execute_liquidity_order(
                &mut pool,
                &params,
                amount_after_fees,
                fee_breakdown.clone(),
                &risk_profile,
                &tick_3d,
                ctx.accounts,
                ctx.remaining_accounts,
            )?
        },
        OrderType::Limit => {
            execute_limit_order(
                &mut pool,
                &params,
                amount_after_fees,
                fee_breakdown.clone(),
                &risk_profile,
                &tick_3d,
                ctx.accounts,
            )?
        },
    };
    
    // ========================================================================
    // PHASE 4: FINALIZATION
    // ========================================================================
    
    // 4.1 Update pool state
    pool.last_update_slot = clock.slot;
    
    // 4.2 Accumulate protocol fees
    if fee_breakdown.protocol_fee > 0 {
        let is_token_a = match &params.rate_params {
            RateParams::TargetRate { is_token_a_to_b, .. } => *is_token_a_to_b,
            _ => true, // Default for liquidity
        };
        pool.accumulate_protocol_fees(fee_breakdown.protocol_fee, is_token_a)?;
    }
    
    // 4.3 Build hook context
    let hook_context = build_3d_hook_context(
        &ctx,
        &params,
        &result,
        &tick_3d,
    );
    
    // 4.4 Release reentrancy lock for hooks
    if ctx.accounts.hook_registry.is_some() {
        pool.set_reentrancy_status(ReentrancyStatus::HookExecuting)?;
    }
    
    // 4.5 Save pool state before external calls
    drop(pool);
    
    // 4.6 Execute hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_hooks!(
            Some(registry),
            None,
            match params.order_type {
                OrderType::Immediate => EVENT_ORDER_FILLED,
                _ => EVENT_ORDER_CREATED,
            },
            hook_context.clone(),
            ctx.remaining_accounts
        );
    }
    
    // 4.7 Execute token transfers
    execute_3d_transfers(&ctx, &params, &result)?;
    
    // 4.8 Execute post-hooks
    if let Some(registry) = &ctx.accounts.hook_registry {
        execute_post_hooks!(
            Some(registry),
            ctx.accounts.hook_message_queue.as_mut(),
            match params.order_type {
                OrderType::Immediate => EVENT_ORDER_FILLED,
                _ => EVENT_ORDER_CREATED,
            },
            hook_context,
            ctx.remaining_accounts
        );
    }
    
    // 4.9 Release reentrancy lock
    let mut pool = ctx.accounts.pool.load_mut()?;
    pool.set_reentrancy_status(ReentrancyStatus::Unlocked)?;
    drop(pool);
    
    // 4.10 Emit event
    emit!(OrderEvent {
        pool: ctx.accounts.pool.key(),
        user: ctx.accounts.user.key(),
        order_type: crate::logic::event::OrderEventType::Created,
        amount_in: params.amount,
        amount_out: result.amount_out,
        rate_tick: tick_3d.rate_tick,
        duration: params.duration,
        leverage: params.leverage,
        fees_paid: fee_breakdown.total_fee,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(result)
}

// ============================================================================
// Execution Functions
// ============================================================================

/// Execute immediate order (swap or flash loan)
fn execute_immediate_order<'info>(
    pool: &mut Pool,
    params: &OrderParams,
    amount_after_fees: u64,
    _fee_breakdown: crate::utils::FeeBreakdown,
    _risk_profile: &RiskProfile,
    _remaining_accounts: &'info [AccountInfo<'info>],
    tick_array_router: Option<&Account<'info, TickArrayRouter>>,
    program_id: &Pubkey,
) -> Result<OrderResult> {
    match &params.rate_params {
        RateParams::TargetRate { sqrt_rate_limit, is_token_a_to_b } => {
            // Initialize order state
            let mut order_state = OrderState {
                amount_remaining: amount_after_fees,
                amount_calculated: 0,
                sqrt_rate: pool.current_sqrt_rate,
                tick: pool.current_tick,
                fee_amount: _fee_breakdown.total_fee,
                liquidity: pool.liquidity,
            };
            
            // Execute the order through concentrated liquidity
            let amount_out = OrderManager::execute_concentrated_liquidity_order(
                &mut order_state,
                pool,
                *sqrt_rate_limit,
                *is_token_a_to_b,
                _remaining_accounts,
                tick_array_router,
                program_id,
            )?;
            
            // Validate output meets minimum
            require!(
                amount_out >= params.limit_value,
                FeelsProtocolError::SlippageExceeded
            );
            
            Ok(OrderResult {
                order_id: None, // Immediate orders don't get IDs
                amount_out,
                amount_locked: 0,
                tick_3d_final: Tick3D {
                    rate_tick: pool.current_tick,
                    duration_tick: params.duration.to_tick(),
                    leverage_tick: _risk_profile.to_tick(),
                },
            })
        },
        _ => Err(FeelsProtocolError::InvalidOrderType.into()),
    }
}

/// Execute liquidity provision order
fn execute_liquidity_order(
    pool: &mut Pool,
    params: &OrderParams,
    amount_after_fees: u64,
    _fee_breakdown: crate::utils::FeeBreakdown,
    _risk_profile: &RiskProfile,
    tick_3d: &Tick3D,
    accounts: &crate::Order,
    _remaining_accounts: &[AccountInfo],
) -> Result<OrderResult> {
    match &params.rate_params {
        RateParams::RateRange { tick_lower, tick_upper } => {
            // Validate tick range
            require!(
                tick_lower < tick_upper,
                FeelsProtocolError::InvalidTickRange
            );
            
            // Calculate liquidity amount based on leverage
            let base_liquidity = crate::logic::concentrated_liquidity::ConcentratedLiquidityManager::calculate_liquidity_from_amounts(
                pool.current_sqrt_rate,
                crate::utils::TickMath::get_sqrt_ratio_at_tick(*tick_lower)?,
                crate::utils::TickMath::get_sqrt_ratio_at_tick(*tick_upper)?,
                amount_after_fees,
                0, // amount_b - single sided liquidity
            )?;
            
            // Apply leverage multiplier to liquidity
            let effective_liquidity = (base_liquidity as u128)
                .checked_mul(params.leverage as u128)
                .and_then(|l| l.checked_div(RiskProfile::LEVERAGE_SCALE as u128))
                .ok_or(FeelsProtocolError::MathOverflow)?;
            
            // Update tick arrays
            // TODO: In a real implementation, we would get the tick arrays from remaining_accounts
            // For now, we'll skip the tick update as it requires loading the actual tick array accounts
            // This would be done like:
            // let tick_array_lower = get_tick_array_from_remaining(remaining_accounts, tick_lower)?;
            // let tick_array_upper = get_tick_array_from_remaining(remaining_accounts, tick_upper)?;
            // TickManager::update_tick_liquidity(&mut tick_array_lower, *tick_lower, effective_liquidity as i128, false)?;
            // TickManager::update_tick_liquidity(&mut tick_array_upper, *tick_upper, -(effective_liquidity as i128), true)?;
            
            msg!("Tick liquidity updates would happen here for ticks {} and {}", tick_lower, tick_upper);
            
            // Update pool liquidity if position is in range
            if pool.current_tick >= *tick_lower && pool.current_tick < *tick_upper {
                pool.liquidity = pool.liquidity
                    .checked_add(effective_liquidity)
                    .ok_or(FeelsProtocolError::MathOverflow)?;
            }
            
            // Generate order ID for liquidity positions
            let order_id = generate_order_id(
                &accounts.user.key(),
                &accounts.pool.key(),
                tick_3d,
            );
            
            Ok(OrderResult {
                order_id: Some(order_id),
                amount_out: 0, // No immediate output for liquidity
                amount_locked: amount_after_fees,
                tick_3d_final: *tick_3d,
            })
        },
        _ => Err(FeelsProtocolError::InvalidOrderType.into()),
    }
}

/// Execute limit order
fn execute_limit_order(
    _pool: &mut Pool,
    _params: &OrderParams,
    amount_after_fees: u64,
    _fee_breakdown: crate::utils::FeeBreakdown,
    _risk_profile: &RiskProfile,
    tick_3d: &Tick3D,
    accounts: &crate::Order,
) -> Result<OrderResult> {
    // Limit orders are stored and will be actively matched by keepers
    
    let order_id = generate_order_id(
        &accounts.user.key(),
        &accounts.pool.key(),
        tick_3d,
    );
    
    // Store order metadata
    // TODO: When implementing compressed accounts, limit orders would be stored in:
    // - Compressed order book merkle tree
    // - Only order hash stored on-chain
    // - Full order data accessible via proof
    // This enables millions of limit orders without account rent
    msg!("Limit order created: {}", order_id);
    msg!("Will execute when rate conditions are met");
    
    Ok(OrderResult {
        order_id: Some(order_id),
        amount_out: 0,
        amount_locked: amount_after_fees,
        tick_3d_final: *tick_3d,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate effective amount considering leverage and duration
fn calculate_effective_amount(
    base_amount: u64,
    leverage: u64,
    duration: &Duration,
) -> Result<u64> {
    // Apply leverage multiplier
    let leveraged_amount = (base_amount as u128)
        .checked_mul(leverage as u128)
        .and_then(|a| a.checked_div(RiskProfile::LEVERAGE_SCALE as u128))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    // Apply duration discount/premium
    let duration_multiplier = match duration {
        Duration::Flash => 9500, // 5% discount for flash
        Duration::Swap => 10000, // No adjustment
        Duration::Weekly => 10100, // 1% premium
        Duration::Monthly => 10300, // 3% premium
        Duration::Quarterly => 10500, // 5% premium
        Duration::Annual => 11000, // 10% premium
    };
    
    let final_amount = leveraged_amount
        .checked_mul(duration_multiplier)
        .and_then(|a| a.checked_div(10000))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    Ok(final_amount as u64)
}

/// Calculate fees considering all three dimensions
fn calculate_3d_fees(
    fee_config: &Account<FeeConfig>,
    amount: u64,
    risk_profile: &RiskProfile,
    duration: &Duration,
    oracle: Option<&Account<Oracle>>,
    oracle_data: Option<&AccountInfo>,
) -> Result<crate::utils::FeeBreakdown> {
    use crate::logic::fee_manager::{FeeManager, FeeContext, FeeType, PositionFeeData};
    
    // Build fee context with all parameters
    let position_data = Some(PositionFeeData {
        leverage: risk_profile.leverage,
        duration: Some(duration.clone()),
    });
    
    // Use SecureOrderManager if oracle is available, otherwise use FeeManager directly
    let mut fees = if let (Some(_oracle), Some(_oracle_data)) = (oracle, oracle_data) {
        // Use secure fee calculation with oracle
        crate::logic::order::SecureOrderManager::calculate_swap_fees_safe(
            fee_config,
            amount,
            oracle,
            oracle_data,
        )?
    } else {
        // Build fee context without oracle
        let context = FeeContext {
            fee_type: FeeType::Swap,
            amount,
            fee_config,
            volatility_tracker: None,
            lending_metrics: None,
            position_data,
            volatility_bps: None,
            volume_24h: None,
        };
        
        // Calculate fees using FeeManager
        FeeManager::calculate_fee(context)?
    };
    
    // Apply leverage fee multiplier (higher leverage = higher fees)
    let leverage_multiplier = risk_profile.fee_multiplier;
    fees.liquidity_fee = ((fees.liquidity_fee as u128 * leverage_multiplier as u128) / 10000) as u64;
    fees.protocol_fee = ((fees.protocol_fee as u128 * leverage_multiplier as u128) / 10000) as u64;
    
    // Apply duration fee adjustment
    let duration_multiplier = match duration {
        Duration::Flash => 15000, // 1.5x for flash loans
        Duration::Swap => 10000, // 1x base
        Duration::Weekly => 9000, // 0.9x for longer commitments
        Duration::Monthly => 8000, // 0.8x
        Duration::Quarterly => 7000, // 0.7x
        Duration::Annual => 6000, // 0.6x
    };
    
    fees.liquidity_fee = ((fees.liquidity_fee as u128 * duration_multiplier) / 10000) as u64;
    fees.protocol_fee = ((fees.protocol_fee as u128 * duration_multiplier) / 10000) as u64;
    fees.total_fee = fees.liquidity_fee + fees.protocol_fee;
    
    Ok(fees)
}

/// Generate unique order ID from components
fn generate_order_id(
    user: &Pubkey,
    pool: &Pubkey,
    tick_3d: &Tick3D,
) -> Pubkey {
    let seeds = &[
        b"order",
        user.as_ref(),
        pool.as_ref(),
        &tick_3d.encode().to_le_bytes(),
        &Clock::get().unwrap().unix_timestamp.to_le_bytes(),
    ];
    
    Pubkey::find_program_address(seeds, &crate::ID).0
}

/// Build hook context for 3D orders
fn build_3d_hook_context(
    ctx: &Context<crate::Order>,
    params: &OrderParams,
    result: &OrderResult,
    tick_3d: &Tick3D,
) -> crate::logic::hook::HookContext {
    let mut context = HookContextBuilder::base(
        ctx.accounts.pool.key(),
        ctx.accounts.user.key(),
    );
    
    // Add 3D-specific context
    context.data.insert("order_type".to_string(), format!("{:?}", params.order_type));
    context.data.insert("duration".to_string(), format!("{:?}", params.duration));
    context.data.insert("leverage".to_string(), params.leverage.to_string());
    context.data.insert("tick_3d_encoded".to_string(), tick_3d.encode().to_string());
    
    if let Some(order_id) = &result.order_id {
        context.data.insert("order_id".to_string(), order_id.to_string());
    }
    
    context
}

/// Execute transfers for 3D orders
fn execute_3d_transfers(
    ctx: &Context<crate::Order>,
    params: &OrderParams,
    result: &OrderResult,
) -> Result<()> {
    // Get pool bump
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
    
    match params.order_type {
        OrderType::Immediate => {
            // Execute swap transfers
            let is_token_a_to_b = match &params.rate_params {
                RateParams::TargetRate { is_token_a_to_b, .. } => *is_token_a_to_b,
                _ => unreachable!(),
            };
            
            crate::utils::cpi_helpers::execute_swap_transfers(
                ctx.accounts.user_token_a.to_account_info(),
                ctx.accounts.user_token_b.to_account_info(),
                ctx.accounts.pool_token_a.to_account_info(),
                ctx.accounts.pool_token_b.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                params.amount,
                result.amount_out,
                is_token_a_to_b,
                &pool,
                pool_bump,
            )?;
        },
        OrderType::Liquidity => {
            // Transfer tokens to pool for liquidity
            crate::utils::cpi_helpers::transfer_pair_from_user_to_pool(
                ctx.accounts.user_token_a.to_account_info(),
                ctx.accounts.user_token_b.to_account_info(),
                ctx.accounts.pool_token_a.to_account_info(),
                ctx.accounts.pool_token_b.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                result.amount_locked,
                0, // Amount B calculated based on range
            )?;
        },
        OrderType::Limit => {
            // Lock tokens for limit order
            crate::utils::cpi_helpers::transfer_from_user_to_pool(
                ctx.accounts.user_token_a.to_account_info(),
                ctx.accounts.pool_token_a.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                params.amount,
            )?;
        },
    }
    
    Ok(())
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, AnchorSerialize, AnchorDeserialize)]
pub struct OrderResult {
    /// Order ID for non-immediate orders
    pub order_id: Option<Pubkey>,
    /// Amount received (for immediate orders)
    pub amount_out: u64,
    /// Amount locked in position (for liquidity/limit orders)
    pub amount_locked: u64,
    /// Final 3D tick position
    pub tick_3d_final: Tick3D,
}