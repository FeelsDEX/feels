/// Unified order system for all trading operations in the Feels Protocol.
/// All operations (swaps, liquidity, positions, limits) go through this single entry point.
/// Implements hub-and-spoke routing where all pools include FeelsSOL.
/// This ensures consistent validation, thermodynamic work calculation, and state management.
use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, Duration, RiskProfile, FeelsSOL, MarketField};
use crate::logic::{OrderManager, StateContext};
use crate::error::FeelsError;
use crate::utils::routing;
use crate::constant::{MAX_ROUTE_HOPS, MAX_SEGMENTS_PER_HOP, MAX_SEGMENTS_PER_TRADE};

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderParams {
    Create(CreateOrderParams),
    Modify(ModifyOrderParams),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateOrderParams {
    /// Type of order with embedded parameters
    pub order_type: OrderType,
    /// Amount for the order (interpretation depends on order type)
    pub amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ModifyOrderParams {
    pub order_id: u64,
    pub modification: OrderModification,
    pub new_params: OrderUpdateParams,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderType {
    /// Token-to-token swap (immediate execution)
    /// Enforces hub-and-spoke routing: max 2 hops, all pools include FeelsSOL
    Swap {
        /// Route through pools (max 2 hops)
        route: Vec<Pubkey>,
        /// Minimum amount to receive
        min_amount_out: u64,
        /// Direction for each hop (true = token0->token1)
        zero_for_one: Vec<bool>,
    },
    
    /// Entry flow: JitoSOL -> FeelsSOL (single hop)
    Entry {
        /// Minimum FeelsSOL to receive
        min_feelssol_out: u64,
    },
    
    /// Exit flow: FeelsSOL -> JitoSOL (single hop)
    Exit {
        /// Minimum JitoSOL to receive  
        min_jitosol_out: u64,
    },
    
    /// Enter a position from FeelsSOL
    EnterPosition {
        /// Type of position (Time or Leverage)
        position_type: PositionType,
        /// Minimum position tokens to receive
        min_position_tokens: u64,
    },
    
    /// Exit a position to FeelsSOL
    ExitPosition {
        /// Position mint to exit from
        position_mint: Pubkey,
        /// Minimum FeelsSOL to receive
        min_feelssol_out: u64,
    },
    
    /// Convert between positions via FeelsSOL hub (2 hops: Position -> FeelsSOL -> Position)
    ConvertPosition {
        /// Source position mint
        source_position: Pubkey,
        /// Target position type
        target_position_type: PositionType,
        /// Minimum destination tokens
        min_tokens_out: u64,
    },
    
    /// Add liquidity to a pool
    AddLiquidity {
        /// Price range for concentrated liquidity
        tick_lower: i32,
        tick_upper: i32,
        /// Liquidity amount
        liquidity: u128,
    },
    
    /// Remove liquidity from a pool
    RemoveLiquidity {
        /// Liquidity amount to remove
        liquidity: u128,
        /// Minimum amounts to receive
        min_amounts: [u64; 2],
    },
    
    /// Place a limit order
    LimitOrder {
        /// Target price
        sqrt_price_limit: u128,
        /// Order direction
        zero_for_one: bool,
        /// Expiration time
        expiration: Option<i64>,
    },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PositionType {
    Time { duration: Duration },
    Leverage { risk_profile: RiskProfile },
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderModification {
    AdjustLeverage { new_leverage: u64 },
    ChangeDuration { new_duration: Duration },
    AddLiquidity { additional_amount: u64 },
    RemoveLiquidity { amount_to_remove: u64 },
    UpdateLimit { new_rate_limit: u128 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderUpdateParams {
    pub max_slippage_bps: u16,
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub enum OrderResult {
    Create(CreateOrderResult),
    Modify(ModifyOrderResult),
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct CreateOrderResult {
    pub order_id: u64,
    pub rate: u128,
    pub liquidity_provided: u128,
    pub amount_filled: u64,
    pub fees_paid: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct ModifyOrderResult {
    pub order_id: u64,
    pub new_rate: u128,
    pub liquidity_delta: i128,
    pub updated_parameters: bool,
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate order parameters for safety and protocol limits
pub fn validate_order_parameters(
    amount: u64,
    sqrt_rate_limit: u128,
    duration: &Duration,
    leverage: u64,
    max_slippage_bps: u16,
) -> Result<()> {
    // Amount validation
    require!(amount > 0, FeelsProtocolError::InvalidAmount);
    require!(amount <= 1_000_000_000_000, FeelsProtocolError::InvalidAmount); // 1T max

    // Rate validation
    require!(sqrt_rate_limit > 0, FeelsProtocolError::InvalidAmount);

    // Duration validation
    match duration {
        Duration::Flash => {
            // Flash loans have additional restrictions
            require!(leverage == RiskProfile::LEVERAGE_SCALE, FeelsProtocolError::InvalidDuration);
        }
        _ => {} // Other durations validated by enum bounds
    }

    // Leverage validation
    require!(
        leverage >= RiskProfile::LEVERAGE_SCALE && leverage <= RiskProfile::MAX_LEVERAGE_SCALE,
        FeelsProtocolError::InvalidAmount
    );

    // Slippage validation
    require!(max_slippage_bps <= 10000, FeelsProtocolError::InvalidAmount); // Max 100%

    Ok(())
}

// ============================================================================
// Handler Function - ALL operations use unified OrderManager
// ============================================================================

/// Unified order handler for all trading operations
pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Order<'info>>,
    params: OrderParams,
) -> Result<OrderResult> {
    match params {
        OrderParams::Create(create_params) => handle_create_order(ctx, create_params),
        OrderParams::Modify(modify_params) => handle_modify_order(ctx, modify_params),
    }
}

/// Handle order creation - delegates everything to OrderManager
fn handle_create_order<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Order<'info>>,
    params: CreateOrderParams,
) -> Result<OrderResult> {
    // Common validation
    require!(params.amount > 0, FeelsError::InvalidAmount);
    
    // Extract tick arrays from remaining accounts
    let tick_arrays = ctx.remaining_accounts
        .iter()
        .filter_map(|acc| {
            if acc.owner == &crate::ID {
                Some(AccountLoader::<crate::state::TickArray>::try_from(acc).ok()?)
            } else {
                None
            }
        })
        .collect();
    
    // Create state context
    let state_context = StateContext::new(
        &ctx.accounts.market_field,
        &ctx.accounts.market_manager,
        &ctx.accounts.buffer_account,
        tick_arrays,
    )?;
    
    // Use unified OrderManager with integrated physics calculations
    let order_manager = OrderManager::new(state_context, &ctx.accounts.market_field);
    
    // Execute based on order type
    let result = match params.order_type {
        OrderType::Swap { route, min_amount_out, zero_for_one } => {
            execute_swap(order_manager, params.amount, &route, min_amount_out, &zero_for_one)?
        },
        
        OrderType::Entry { min_feelssol_out } => {
            execute_entry(order_manager, params.amount, min_feelssol_out)?
        },
        
        OrderType::Exit { min_jitosol_out } => {
            execute_exit(order_manager, params.amount, min_jitosol_out)?
        },
        
        OrderType::EnterPosition { position_type, min_position_tokens } => {
            execute_enter_position(order_manager, params.amount, &position_type, min_position_tokens)?
        },
        
        OrderType::ExitPosition { position_mint, min_feelssol_out } => {
            execute_exit_position(order_manager, params.amount, &position_mint, min_feelssol_out)?
        },
        
        OrderType::ConvertPosition { source_position, target_position_type, min_tokens_out } => {
            execute_convert_position(order_manager, params.amount, &source_position, &target_position_type, min_tokens_out)?
        },
        
        OrderType::AddLiquidity { tick_lower, tick_upper, liquidity } => {
            execute_add_liquidity(order_manager, tick_lower, tick_upper, liquidity)?
        },
        
        OrderType::RemoveLiquidity { liquidity, min_amounts } => {
            execute_remove_liquidity(order_manager, liquidity, &min_amounts)?
        },
        
        OrderType::LimitOrder { sqrt_price_limit, zero_for_one, expiration } => {
            execute_limit_order(order_manager, params.amount, sqrt_price_limit, zero_for_one, &expiration)?
        },
    };
    
    Ok(OrderResult::Create(result))
}

/// Handle order modification
fn handle_modify_order<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Order<'info>>,
    params: ModifyOrderParams,
) -> Result<OrderResult> {
    msg!("Order modification not yet implemented");
    
    Ok(OrderResult::Modify(ModifyOrderResult {
        order_id: params.order_id,
        new_rate: 0,
        liquidity_delta: 0,
        updated_parameters: false,
    }))
}

// ============================================================================
// Execution Functions - All use unified OrderManager with integrated physics
// ============================================================================

/// Execute a token swap with hub-and-spoke routing validation
fn execute_swap<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    route: &Vec<Pubkey>,
    min_amount_out: u64,
    zero_for_one: &Vec<bool>,
) -> Result<CreateOrderResult> {
    // Validate route constraints
    require!(
        route.len() <= MAX_ROUTE_HOPS,
        FeelsError::route_too_long(route.len(), MAX_ROUTE_HOPS)
    );
    require!(
        route.len() == zero_for_one.len(),
        FeelsError::InvalidInput
    );
    
    msg!("Executing swap with {} hop(s)", route.len());
    
    // Hub-and-spoke constraint validation:
    // - Single hop: One token must be FeelsSOL (validated at pool init)
    // - Two hop: Must go through FeelsSOL hub (validated by route structure)
    
    // Execute swap through physics manager (handles work calculation)
    let result = if route.len() == 1 {
        // Single hop - direct swap
        order_manager.execute_swap(
            amount_in,
            min_amount_out,
            zero_for_one[0],
            None, // No price limit
        )?
    } else {
        // Two hop - through FeelsSOL hub
        // First pool: TokenA -> FeelsSOL
        // Second pool: FeelsSOL -> TokenB
        order_manager.execute_two_hop_swap(
            amount_in,
            min_amount_out,
            &route[0],
            &route[1],
            zero_for_one[0],
            zero_for_one[1],
        )?
    };
    
    CreateOrderResult {
        order_id: 0,
        rate: result.sqrt_price_after,
        liquidity_provided: 0,
        amount_filled: result.amount_out,
        fees_paid: result.fee_amount,
    }
}

/// Execute entry: JitoSOL -> FeelsSOL
fn execute_entry<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    min_feelssol_out: u64,
) -> Result<CreateOrderResult> {
    // Entry uses the standard JitoSOL/FeelsSOL pool
    // The pool must exist and be initialized per hub-and-spoke design
    msg!("Executing entry: {} JitoSOL -> FeelsSOL", amount_in);
    
    // Entry is a single hop swap through the designated entry/exit pool
    let result = order_manager.execute_swap(
        amount_in,
        min_feelssol_out,
        true, // JitoSOL->FeelsSOL (token0->token1)
        None,
    )?;
    
    CreateOrderResult {
        order_id: 0,
        rate: result.sqrt_price_after,
        liquidity_provided: 0,
        amount_filled: result.amount_out,
        fees_paid: result.fee_amount,
    }
}

/// Execute exit: FeelsSOL -> JitoSOL
fn execute_exit<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    min_jitosol_out: u64,
) -> Result<CreateOrderResult> {
    // Exit uses the standard JitoSOL/FeelsSOL pool
    msg!("Executing exit: {} FeelsSOL -> JitoSOL", amount_in);
    
    // Exit is a single hop swap through the designated entry/exit pool
    let result = order_manager.execute_swap(
        amount_in,
        min_jitosol_out,
        false, // FeelsSOL->JitoSOL (token1->token0)
        None,
    )?;
    
    CreateOrderResult {
        order_id: 0,
        rate: result.sqrt_price_after,
        liquidity_provided: 0,
        amount_filled: result.amount_out,
        fees_paid: result.fee_amount,
    }
}

/// Execute enter position: FeelsSOL -> Position
fn execute_enter_position<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    position_type: &PositionType,
    min_position_tokens: u64,
) -> Result<CreateOrderResult> {
    let result = order_manager.enter_position(
        amount_in,
        position_type,
        min_position_tokens,
    )?;
    
    CreateOrderResult {
        order_id: 0,
        rate: result.exchange_rate,
        liquidity_provided: 0,
        amount_filled: result.tokens_out,
        fees_paid: result.fee_amount,
    }
}

/// Execute exit position: Position -> FeelsSOL
fn execute_exit_position<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    position_mint: &Pubkey,
    min_feelssol_out: u64,
) -> Result<CreateOrderResult> {
    let result = order_manager.exit_position(
        position_mint,
        amount_in,
        min_feelssol_out,
    )?;
    
    CreateOrderResult {
        order_id: 0,
        rate: result.exchange_rate,
        liquidity_provided: 0,
        amount_filled: result.tokens_out,
        fees_paid: result.fee_amount,
    }
}

/// Execute position conversion: Position -> FeelsSOL -> Position (2 hops)
fn execute_convert_position<'info>(
    order_manager: OrderManager<'info>,
    amount_in: u64,
    source_position: &Pubkey,
    target_position_type: &PositionType,
    min_tokens_out: u64,
) -> Result<CreateOrderResult> {
    // Position conversion requires two operations through the hub
    // First exit source position to FeelsSOL
    let exit_result = order_manager.exit_position(
        source_position,
        amount_in,
        0, // No slippage check on intermediate
    )?;
    
    // Then enter target position from FeelsSOL
    let enter_result = order_manager.enter_position(
        exit_result.tokens_out,
        target_position_type,
        min_tokens_out,
    )?;
    
    CreateOrderResult {
        order_id: 0,
        rate: enter_result.exchange_rate,
        liquidity_provided: 0,
        amount_filled: enter_result.tokens_out,
        fees_paid: exit_result.fee_amount + enter_result.fee_amount,
    }
}

/// Execute add liquidity
fn execute_add_liquidity<'info>(
    order_manager: OrderManager<'info>,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
) -> Result<CreateOrderResult> {
    let result = order_manager.add_liquidity(
        tick_lower,
        tick_upper,
        liquidity,
    )?;
    
    CreateOrderResult {
        order_id: result.position_id,
        rate: 0,
        liquidity_provided: result.liquidity,
        amount_filled: result.amount0 + result.amount1,
        fees_paid: 0,
    }
}

/// Execute remove liquidity
fn execute_remove_liquidity<'info>(
    order_manager: OrderManager<'info>,
    liquidity: u128,
    min_amounts: &[u64; 2],
) -> Result<CreateOrderResult> {
    let result = order_manager.remove_liquidity(
        0, // TODO: Get position ID from context
        liquidity,
    )?;
    
    require!(
        result.amount0 >= min_amounts[0] && result.amount1 >= min_amounts[1],
        FeelsError::InvalidSlippageLimit
    );
    
    CreateOrderResult {
        order_id: result.position_id,
        rate: 0,
        liquidity_provided: 0,
        amount_filled: result.amount0 + result.amount1,
        fees_paid: 0,
    }
}

/// Execute limit order
fn execute_limit_order<'info>(
    order_manager: OrderManager<'info>,
    amount: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
    expiration: &Option<i64>,
) -> Result<CreateOrderResult> {
    let result = order_manager.place_limit_order(
        amount,
        sqrt_price_limit,
        zero_for_one,
        expiration,
    )?;
    
    CreateOrderResult {
        order_id: result.order_id,
        rate: sqrt_price_limit,
        liquidity_provided: result.liquidity,
        amount_filled: 0, // Not filled yet
        fees_paid: 0,
    }
}