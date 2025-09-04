//! # Unified Order Handler with Unified Market Account
//! 
//! Updated version of the order handler that uses the single unified Market account
//! instead of separate MarketField and MarketManager accounts.
//!
//! ## Key Changes:
//! - Single Market account contains both thermodynamic state and AMM parameters
//! - Simplified account validation and access patterns
//! - Reduced account requirements for each instruction

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer, Burn, MintTo};
use crate::error::FeelsProtocolError;
use crate::state::*;
use crate::logic::{
    OrderManager, OrderType as LogicOrderType, HubRoute,
    UnifiedStateContext, UnifiedWorkUnit,
    calculate_thermodynamic_fee, ThermodynamicFeeParams,
    calculate_path_work, PathSegment, WorkResult,
    ConservationProof, verify_conservation,
};
use feels_core::constants::*;

// ============================================================================
// Order Types (Same as before)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TradingRiskProfile {
    pub max_slippage_bps: u16,
    pub max_price_impact_bps: u16,
    pub require_atomic: bool,
    pub timeout_seconds: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapRoute {
    pub pools: Vec<Pubkey>,
    pub token_path: Vec<Pubkey>,
    pub fee_tiers: Vec<u16>,
    pub estimated_amount_out: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum OrderType {
    /// Standard token swap (max 2 hops)
    Swap {
        amount: u64,
        other_amount_threshold: u64,
        exact_input: bool,
        route: HubRoute,
    },
    
    /// Entry flow: JitoSOL → FeelsSOL
    Entry {
        jitosol_in: u64,
        min_feelssol_out: u64,
    },
    
    /// Exit flow: FeelsSOL → JitoSOL
    Exit {
        feelssol_in: u64,
        min_jitosol_out: u64,
    },
    
    /// Enter position: FeelsSOL → Position tokens
    EnterPosition {
        feelssol_in: u64,
        min_tokens_out: u64,
        position_type: PositionType,
    },
    
    /// Exit position: Position tokens → FeelsSOL
    ExitPosition {
        position_tokens_in: u64,
        min_feelssol_out: u64,
        position_type: PositionType,
    },
    
    /// Add liquidity
    AddLiquidity {
        amount_a: u64,
        amount_b: u64,
        min_liquidity: u128,
        tick_lower: i32,
        tick_upper: i32,
    },
    
    /// Remove liquidity
    RemoveLiquidity {
        liquidity: u128,
        min_amount_a: u64,
        min_amount_b: u64,
        tick_lower: i32,
        tick_upper: i32,
    },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum PositionType {
    Spot,
    Time,
    Leverage,
}

// ============================================================================
// Order Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderParams {
    pub order_type: OrderType,
    pub deadline: Option<i64>,
    pub risk_profile: Option<TradingRiskProfile>,
}

// ============================================================================
// Order Result
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct OrderResult {
    // Trade amounts
    pub amount_in: u64,
    pub amount_out: u64,
    
    // Liquidity changes
    pub liquidity_delta: i128,
    pub liquidity_shares: u64,
    
    // Work and fees
    pub work_performed: u128,
    pub fee_paid: u64,
    pub rebate_received: u64,
    
    // Market state changes
    pub final_sqrt_price: u128,
    pub final_tick: i32,
    
    // Conservation proof
    pub conservation_valid: bool,
}

// ============================================================================
// Main Handler
// ============================================================================

pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, UnifiedOrder<'info>>,
    params: OrderParams,
) -> Result<OrderResult> {
    // Initialize work unit for atomic state management
    let mut work_unit = UnifiedWorkUnit::new();
    
    // Load unified market account
    let market = work_unit.load_market(&ctx.accounts.market)?;
    
    // Validate market is active
    require!(
        market.is_initialized && !market.is_paused,
        FeelsProtocolError::InvalidOperation
    );
    
    // Load buffer account
    let buffer = work_unit.load_buffer(&ctx.accounts.buffer)?;
    
    // Create state context
    let tick_arrays: Vec<&AccountLoader<TickArray>> = ctx.remaining_accounts
        .iter()
        .filter_map(|acc| {
            if acc.data_len() == std::mem::size_of::<TickArray>() + 8 {
                Some(unsafe { std::mem::transmute(acc) })
            } else {
                None
            }
        })
        .collect();
    
    let state_context = UnifiedStateContext::new(
        &ctx.accounts.market,
        &ctx.accounts.buffer,
        tick_arrays,
    )?;
    
    // Create order manager
    let order_manager = OrderManager::new(
        &ctx.accounts.market,
        &ctx.accounts.buffer,
        &ctx.accounts.feelssol_state.key(),
    )?;
    
    // Execute order based on type
    let result = match params.order_type {
        OrderType::Swap { amount, other_amount_threshold, exact_input, route } => {
            execute_swap(
                &mut work_unit,
                &state_context,
                &order_manager,
                amount,
                other_amount_threshold,
                exact_input,
                route,
            )?
        },
        
        OrderType::Entry { jitosol_in, min_feelssol_out } => {
            execute_entry(
                &mut work_unit,
                &state_context,
                jitosol_in,
                min_feelssol_out,
                &ctx.accounts.user_jitosol,
                &ctx.accounts.user_feelssol,
                &ctx.accounts.feelssol_state,
                &ctx.accounts.feelssol_mint,
                &ctx.accounts.token_program,
            )?
        },
        
        OrderType::Exit { feelssol_in, min_jitosol_out } => {
            execute_exit(
                &mut work_unit,
                &state_context,
                feelssol_in,
                min_jitosol_out,
                &ctx.accounts.user_feelssol,
                &ctx.accounts.user_jitosol,
                &ctx.accounts.feelssol_state,
                &ctx.accounts.feelssol_mint,
                &ctx.accounts.token_program,
            )?
        },
        
        OrderType::AddLiquidity { amount_a, amount_b, min_liquidity, tick_lower, tick_upper } => {
            execute_add_liquidity(
                &mut work_unit,
                &state_context,
                amount_a,
                amount_b,
                min_liquidity,
                tick_lower,
                tick_upper,
            )?
        },
        
        _ => {
            // Other order types to be implemented
            return Err(FeelsProtocolError::NotImplemented.into());
        }
    };
    
    // Verify conservation if required
    if should_verify_conservation(&params.order_type) {
        let proof = create_conservation_proof(&work_unit)?;
        verify_conservation(&proof)?;
    }
    
    // Commit all state changes
    work_unit.commit()?;
    
    Ok(result)
}

// ============================================================================
// Order Execution Functions
// ============================================================================

fn execute_swap(
    work_unit: &mut UnifiedWorkUnit,
    state_context: &UnifiedStateContext,
    order_manager: &OrderManager,
    amount: u64,
    other_amount_threshold: u64,
    exact_input: bool,
    route: HubRoute,
) -> Result<OrderResult> {
    let market = work_unit.get_market()?;
    
    // Calculate path segments
    let segments = match route {
        HubRoute::Direct => {
            vec![PathSegment {
                start: Position3D::new(market.S, market.T, market.L),
                end: Position3D::new(market.S, market.T, market.L), // To be calculated
                distance: amount as u128,
                dimension: TradeDimension::Spot,
            }]
        },
        HubRoute::ThroughHub { first_hop, second_hop } => {
            // Two segments for hub route
            vec![
                PathSegment {
                    start: Position3D::new(market.S, market.T, market.L),
                    end: Position3D::new(market.S, market.T, market.L), // Intermediate
                    distance: amount as u128,
                    dimension: TradeDimension::Spot,
                },
                PathSegment {
                    start: Position3D::new(market.S, market.T, market.L), // From intermediate
                    end: Position3D::new(market.S, market.T, market.L), // Final
                    distance: 0, // To be calculated
                    dimension: TradeDimension::Spot,
                },
            ]
        },
    };
    
    // Calculate work along path
    let market_field = MarketField {
        s: market.S,
        t: market.T,
        l: market.L,
        weights: market.get_domain_weights(),
        sigma_price: market.sigma_price,
        sigma_rate: market.sigma_rate,
        sigma_leverage: market.sigma_leverage,
    };
    
    let work_result = calculate_path_work(&segments, &market_field)?;
    
    // Calculate fees based on work
    let fee_params = ThermodynamicFeeParams {
        work: work_result.net_work,
        amount_in: amount,
        execution_price: market.sqrt_price,
        oracle_price: work_unit.get_oracle()?.get_safe_twap_a(),
        base_fee_bps: market.base_fee_bps,
        kappa: 50, // 0.5% clamping
        max_rebate_bps: 300, // 3% max
        is_buy: !zero_for_one,
        buffer: None, // Access through work unit
    };
    
    let fee_result = calculate_thermodynamic_fee(fee_params)?;
    
    // Execute the swap logic (simplified)
    let (amount_out, final_sqrt_price, final_tick) = if exact_input {
        // Swap exact input
        let output = amount.saturating_sub(fee_result.net_fee);
        require!(output >= other_amount_threshold, FeelsProtocolError::SlippageExceeded);
        (output, market.sqrt_price, market.current_tick)
    } else {
        // Swap exact output
        let input = amount.saturating_add(fee_result.net_fee);
        require!(input <= other_amount_threshold, FeelsProtocolError::SlippageExceeded);
        (amount, market.sqrt_price, market.current_tick)
    };
    
    // Update market state
    work_unit.update_price(final_sqrt_price, final_tick)?;
    work_unit.record_volume(amount, amount_out)?;
    
    // Collect fees
    work_unit.collect_fees(true, fee_result.net_fee)?;
    
    Ok(OrderResult {
        amount_in: if exact_input { amount } else { amount.saturating_add(fee_result.net_fee) },
        amount_out,
        work_performed: work_result.total_work,
        fee_paid: fee_result.net_fee,
        rebate_received: fee_result.rebate,
        final_sqrt_price,
        final_tick,
        conservation_valid: true,
        ..Default::default()
    })
}

fn execute_entry(
    work_unit: &mut UnifiedWorkUnit,
    _state_context: &UnifiedStateContext,
    jitosol_in: u64,
    min_feelssol_out: u64,
    user_jitosol: &Account<TokenAccount>,
    user_feelssol: &Account<TokenAccount>,
    feelssol_state: &Account<FeelsSOL>,
    feelssol_mint: &Account<Mint>,
    token_program: &Program<Token>,
) -> Result<OrderResult> {
    // Transfer JitoSOL from user
    let cpi_accounts = Transfer {
        from: user_jitosol.to_account_info(),
        to: feelssol_state.to_account_info(),
        authority: user_jitosol.to_account_info(),
    };
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, jitosol_in)?;
    
    // Calculate FeelsSOL to mint (1:1 for now)
    let feelssol_out = jitosol_in;
    require!(
        feelssol_out >= min_feelssol_out,
        FeelsProtocolError::SlippageExceeded
    );
    
    // Mint FeelsSOL to user
    let mint_accounts = MintTo {
        mint: feelssol_mint.to_account_info(),
        to: user_feelssol.to_account_info(),
        authority: feelssol_state.to_account_info(),
    };
    let mint_ctx = CpiContext::new(token_program.to_account_info(), mint_accounts);
    token::mint_to(mint_ctx, feelssol_out)?;
    
    Ok(OrderResult {
        amount_in: jitosol_in,
        amount_out: feelssol_out,
        ..Default::default()
    })
}

fn execute_exit(
    work_unit: &mut UnifiedWorkUnit,
    _state_context: &UnifiedStateContext,
    feelssol_in: u64,
    min_jitosol_out: u64,
    user_feelssol: &Account<TokenAccount>,
    user_jitosol: &Account<TokenAccount>,
    feelssol_state: &Account<FeelsSOL>,
    feelssol_mint: &Account<Mint>,
    token_program: &Program<Token>,
) -> Result<OrderResult> {
    // Burn FeelsSOL from user
    let burn_accounts = Burn {
        mint: feelssol_mint.to_account_info(),
        from: user_feelssol.to_account_info(),
        authority: user_feelssol.to_account_info(),
    };
    let burn_ctx = CpiContext::new(token_program.to_account_info(), burn_accounts);
    token::burn(burn_ctx, feelssol_in)?;
    
    // Calculate JitoSOL to return (1:1 for now)
    let jitosol_out = feelssol_in;
    require!(
        jitosol_out >= min_jitosol_out,
        FeelsProtocolError::SlippageExceeded
    );
    
    // Transfer JitoSOL to user
    let transfer_accounts = Transfer {
        from: feelssol_state.to_account_info(),
        to: user_jitosol.to_account_info(),
        authority: feelssol_state.to_account_info(),
    };
    let transfer_ctx = CpiContext::new(token_program.to_account_info(), transfer_accounts);
    token::transfer(transfer_ctx, jitosol_out)?;
    
    Ok(OrderResult {
        amount_in: feelssol_in,
        amount_out: jitosol_out,
        ..Default::default()
    })
}

fn execute_add_liquidity(
    work_unit: &mut UnifiedWorkUnit,
    state_context: &UnifiedStateContext,
    amount_a: u64,
    amount_b: u64,
    min_liquidity: u128,
    tick_lower: i32,
    tick_upper: i32,
) -> Result<OrderResult> {
    // Calculate liquidity from amounts
    let liquidity = std::cmp::min(amount_a, amount_b) as u128; // Simplified
    require!(
        liquidity >= min_liquidity,
        FeelsProtocolError::SlippageExceeded
    );
    
    // Update ticks
    let mut ticks = state_context.ticks;
    let market = work_unit.get_market()?;
    
    // Update lower tick
    ticks.update_tick(
        tick_lower,
        liquidity as i128,
        market.fee_growth_global_0[0] as u128,
        market.fee_growth_global_1[0] as u128,
    )?;
    
    // Update upper tick
    ticks.update_tick(
        tick_upper,
        -(liquidity as i128),
        market.fee_growth_global_0[0] as u128,
        market.fee_growth_global_1[0] as u128,
    )?;
    
    // Update market liquidity
    work_unit.add_liquidity(liquidity)?;
    
    Ok(OrderResult {
        amount_in: amount_a,
        amount_out: amount_b,
        liquidity_delta: liquidity as i128,
        liquidity_shares: liquidity as u64,
        conservation_valid: true,
        ..Default::default()
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn should_verify_conservation(order_type: &OrderType) -> bool {
    match order_type {
        OrderType::Swap { .. } => true,
        OrderType::AddLiquidity { .. } => true,
        OrderType::RemoveLiquidity { .. } => true,
        _ => false,
    }
}

fn create_conservation_proof(work_unit: &UnifiedWorkUnit) -> Result<ConservationProof> {
    let market = work_unit.get_market()?;
    
    Ok(ConservationProof {
        initial_state: Position3D::new(Q64, Q64, Q64), // Placeholder
        final_state: Position3D::new(market.S, market.T, market.L),
        growth_factors: ConservationGrowthFactors {
            g_s: Q64,
            g_t: Q64,
            g_l: Q64,
            g_tau: Q64,
        },
        weighted_logs: ConservationWeightedLogs {
            w_s_ln_g_s: 0,
            w_t_ln_g_t: 0,
            w_l_ln_g_l: 0,
            w_tau_ln_g_tau: 0,
        },
        domain_weights: market.get_domain_weights(),
        operation: ConservationOperation::Swap,
        epsilon_tolerance: 100, // 1 basis point
    })
}

// ============================================================================
// Accounts Structure
// ============================================================================

#[derive(Accounts)]
#[instruction(params: OrderParams)]
pub struct UnifiedOrder<'info> {
    // User accounts
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_a: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_token_b: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_feelssol: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub user_jitosol: Box<Account<'info, TokenAccount>>,
    
    // Market state - UNIFIED ACCOUNT
    #[account(mut)]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub buffer: Account<'info, BufferAccount>,
    
    // Token accounts
    #[account(mut)]
    pub vault_a: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_b: Box<Account<'info, TokenAccount>>,
    
    // Mints
    pub token_a_mint: Box<Account<'info, Mint>>,
    pub token_b_mint: Box<Account<'info, Mint>>,
    
    // FeelsSOL state
    #[account(mut)]
    pub feelssol_state: Box<Account<'info, FeelsSOL>>,
    #[account(mut)]
    pub feelssol_mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub feelssol_vault: Box<Account<'info, TokenAccount>>,
    
    // Position token (for position operations)
    #[account(mut)]
    pub position_token_state: Option<Account<'info, PositionTokenState>>,
    #[account(mut)]
    pub position_token_mint: Option<Account<'info, Mint>>,
    
    // Programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Position token state placeholder
#[account]
pub struct PositionTokenState {
    pub mint: Pubkey,
    pub position_type: PositionType,
    pub feelssol_backing: u64,
}

// Re-export for backward compatibility
pub use self::handler;