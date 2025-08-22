/// Executes token swaps within a concentrated liquidity pool using the constant product formula.
/// Handles price impact calculation, fee collection, and tick crossing as the swap moves through
/// different liquidity ranges. Updates oracle observations for TWAP calculations and emits events
/// for indexing. This is the core trading mechanism of the Feels Protocol AMM.

use crate::state::{Pool, ObservationState, Observation, PoolError, TickArray};
use crate::utils::{SafeMath, TickMath};
use crate::logic::swap::SwapRoute;
use crate::utils::math_liquidity;
use crate::utils::FeeGrowthMath;
use crate::logic::LiquidityMath;
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};

// ============================================================================
// Main Handler Functions
// ============================================================================

/// Execute a swap in the concentrated liquidity pool
pub fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::Swap<'info>>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit: u128,
    is_token_0_to_1: bool,
) -> Result<u64> {
    require!(amount_in > 0, PoolError::InputAmountZero);
    require!(sqrt_price_limit > 0, PoolError::PriceLimitOutsideValidRange);
    
    let mut pool = ctx.accounts.pool.load_mut()?;
    let clock = Clock::get()?;
    
    // Derive the pool bump once for all transfers
    let (_, pool_bump) = Pubkey::find_program_address(
        &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &pool.fee_rate.to_le_bytes(),
        ],
        ctx.program_id,
    );
    
    // Initialize swap state with current pool state
    // This ensures synchronization from the start - both swap_state and pool
    // will be kept consistent throughout the swap process
    let mut swap_state = SwapState {
        amount_remaining: amount_in,
        amount_calculated: 0,
        sqrt_price: pool.current_sqrt_price,
        tick: pool.current_tick,
        fee_amount: 0,
        liquidity: pool.liquidity,
    };
    
    // Calculate fees using unified fee system
    let fee_breakdown = pool.calculate_swap_fees(amount_in)?;
    let amount_in_after_fee = amount_in.safe_sub(fee_breakdown.total_fee)?;
    
    swap_state.amount_remaining = amount_in_after_fee;
    swap_state.fee_amount = fee_breakdown.total_fee;
    
    // Execute concentrated liquidity swap with proper tick iteration
    let amount_out = execute_concentrated_liquidity_swap(
        &mut swap_state,
        &mut pool,
        sqrt_price_limit,
        is_token_0_to_1,
        ctx.remaining_accounts,
    )?;
    
    // Validate slippage protection with granular errors
    require!(amount_out >= amount_out_minimum, PoolError::SlippageExceeded);
    require!(amount_out > 0, PoolError::SwapResultsInZeroOutput);
    
    // Validate price limit with granular errors
    if is_token_0_to_1 {
        require!(swap_state.sqrt_price >= sqrt_price_limit, PoolError::SlippageProtectionTriggered);
    } else {
        require!(swap_state.sqrt_price <= sqrt_price_limit, PoolError::SlippageProtectionTriggered);
    }
    
    // Final verification that pool state is synchronized with swap results
    // Note: Pool state should already be synchronized due to updates during the swap loop,
    // but we ensure consistency here as a safety measure
    pool.current_sqrt_price = swap_state.sqrt_price;
    pool.current_tick = swap_state.tick;
    pool.liquidity = swap_state.liquidity;
    
    // Accumulate protocol fees using unified fee system
    if fee_breakdown.protocol_fee > 0 {
        pool.accumulate_protocol_fees(&fee_breakdown, is_token_0_to_1)?;
    }
    
    // Update volume statistics using safe arithmetic
    if is_token_0_to_1 {
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_in as u128)?;
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_out as u128)?;
    } else {
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_in as u128)?;
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_out as u128)?;
    }
    
    pool.last_update_slot = clock.slot;
    
    // Execute token transfers
    if is_token_0_to_1 {
        // Transfer token A from user to pool
        #[allow(deprecated)]
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_a.to_account_info(),
                    to: ctx.accounts.pool_token_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_in,
        )?;
        
        // Transfer token B from pool to user
        let seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &pool.fee_rate.to_le_bytes(),
            &[pool_bump],
        ];
        
        #[allow(deprecated)]
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_b.to_account_info(),
                    to: ctx.accounts.user_token_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_out,
        )?;
    } else {
        // Transfer token B from user to pool
        #[allow(deprecated)]
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_b.to_account_info(),
                    to: ctx.accounts.pool_token_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_in,
        )?;
        
        // Transfer token A from pool to user
        let seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &pool.fee_rate.to_le_bytes(),
            &[pool_bump],
        ];
        
        #[allow(deprecated)]
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_a.to_account_info(),
                    to: ctx.accounts.user_token_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_out,
        )?;
    }
    
    // Update oracle observation
    update_oracle_observation(
        &mut ctx.accounts.oracle_state,
        pool.current_sqrt_price,
        pool.current_tick,
        clock.unix_timestamp,
    )?;
    
    emit!(SwapEvent {
        pool: ctx.accounts.pool.key(),
        user: ctx.accounts.user.key(),
        amount_in,
        amount_out,
        sqrt_price_after: pool.current_sqrt_price,
        tick_after: pool.current_tick,
        fee: fee_breakdown.total_fee,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(amount_out)
}

#[event]
pub struct SwapEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_price_after: u128,
    pub tick_after: i32,
    pub fee: u64,
    pub timestamp: i64,
}

impl crate::logic::EventBase for SwapEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Swap state for tracking computation
#[derive(Debug)]
struct SwapState {
    amount_remaining: u64,
    amount_calculated: u64,
    sqrt_price: u128,
    tick: i32,
    fee_amount: u64,
    liquidity: u128,
}

/// Execute concentrated liquidity swap with proper tick iteration
/// 
/// This implements the core Uniswap V3-style concentrated liquidity algorithm.
/// The swap proceeds by iterating through initialized tick ranges, computing
/// the swap within each range, and crossing ticks as needed.
/// 
/// # Algorithm Overview:
/// 1. Start at current pool price with active liquidity
/// 2. Compute maximum swap possible within current tick range
/// 3. If price hits tick boundary, cross it and update liquidity
/// 4. Continue until input exhausted or price limit reached
/// 
/// # Parameters:
/// - `swap_state`: Mutable state tracking swap progress
/// - `pool`: The pool being swapped in
/// - `sqrt_price_limit`: Maximum price movement allowed (slippage protection)
/// - `zero_for_one`: Direction - true for token0->token1, false for token1->token0
/// - `remaining_accounts`: Tick array accounts needed for the swap
/// 
/// # Returns:
/// The calculated output amount after fees
fn execute_concentrated_liquidity_swap<'info>(
    swap_state: &mut SwapState,
    pool: &mut Pool,
    sqrt_price_limit: u128,
    zero_for_one: bool,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<u64> {
    // Ensure price limit is within protocol bounds to prevent overflow
    // For sells (zero_for_one): price decreases, so limit must be above minimum
    // For buys (!zero_for_one): price increases, so limit must be below maximum
    let _sqrt_price_start = swap_state.sqrt_price;
    let sqrt_price_limit_adjusted = if zero_for_one {
        sqrt_price_limit.max(crate::utils::MIN_SQRT_PRICE_X64)
    } else {
        sqrt_price_limit
    };

    // Main swap loop - iterate through price space until:
    // 1. All input tokens are consumed (amount_remaining = 0), OR
    // 2. Price limit is reached (current price = limit price)
    while swap_state.amount_remaining > 0 && swap_state.sqrt_price != sqrt_price_limit_adjusted {
        let step = compute_swap_step(
            swap_state.sqrt_price,
            sqrt_price_limit_adjusted,
            swap_state.liquidity,
            swap_state.amount_remaining,
            pool.fee_rate,
            zero_for_one,
        )?;

        // Update swap state with the results of this step
        // Each step consumes some input and produces some output
        swap_state.sqrt_price = step.sqrt_price_next;
        swap_state.amount_remaining = swap_state.amount_remaining.safe_sub(step.amount_in)?;
        swap_state.amount_calculated = swap_state.amount_calculated.safe_add(step.amount_out)?;
        swap_state.fee_amount = swap_state.fee_amount.safe_add(step.fee_amount)?;

        // Update global fee growth to track fees earned by liquidity providers
        // Fee growth is stored as a Q128.128 fixed-point number for precision
        // Formula: fee_growth += (fees_collected * 2^128) / active_liquidity
        if swap_state.liquidity > 0 {
            let fee_growth_delta = FeeGrowthMath::fee_to_fee_growth(
                step.fee_amount,
                swap_state.liquidity
            )?;
            
            if zero_for_one {
                pool.fee_growth_global_0 = FeeGrowthMath::add_fee_growth(
                    pool.fee_growth_global_0,
                    fee_growth_delta,
                )?;
            } else {
                pool.fee_growth_global_1 = FeeGrowthMath::add_fee_growth(
                    pool.fee_growth_global_1,
                    fee_growth_delta,
                )?;
            }
        }

        // Check if this step moved the price to a tick boundary
        // If so, we need to cross the tick and update all state consistently
        if step.sqrt_price_next == step.sqrt_price_target {
            // We've hit a tick boundary - cross it and update all related state
            // This updates liquidity, tick, and price in both swap_state and pool
            cross_tick(
                pool,
                swap_state,
                step.tick_next,
                zero_for_one,
                remaining_accounts,
            )?;
        } else {
            // If we didn't cross a tick, still ensure tick is consistent with price
            // and update pool state to maintain synchronization
            swap_state.tick = TickMath::get_tick_at_sqrt_ratio(swap_state.sqrt_price)?;
            pool.current_tick = swap_state.tick;
            pool.current_sqrt_price = swap_state.sqrt_price;
            pool.liquidity = swap_state.liquidity;
        }
    }

    Ok(swap_state.amount_calculated)
}

/// Compute a single swap step within the current tick range
/// 
/// This function calculates how much of the swap can be completed within
/// the current tick range before hitting a tick boundary or exhausting input.
/// 
/// # Mathematical Foundation:
/// The AMM formula x * y = k becomes L² = x * y in concentrated liquidity.
/// Within a tick range, liquidity L is constant, so we can calculate:
/// - Δx = L * (1/√P_b - 1/√P_a) for token0
/// - Δy = L * (√P_b - √P_a) for token1
/// 
/// # Parameters:
/// - `sqrt_price_current`: Current sqrt price in the pool
/// - `sqrt_price_target`: Target sqrt price (tick boundary or limit)
/// - `liquidity`: Active liquidity in the current tick range
/// - `amount_remaining`: Remaining input tokens to swap
/// - `fee_rate`: Fee rate in basis points (e.g., 30 = 0.3%)
/// - `zero_for_one`: Swap direction
/// 
/// # Returns:
/// SwapStep containing price movement, amounts, and fees for this step
fn compute_swap_step(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u16,
    zero_for_one: bool,
) -> Result<SwapStep> {
    // We always use "exact in" mode - consuming a specific input amount
    let exact_in = amount_remaining > 0;
    
    // Calculate the furthest price we can move given available liquidity
    // This may be limited by the amount remaining or by reaching a tick
    let sqrt_price_next = if exact_in {
        LiquidityMath::get_next_sqrt_price_from_input(
            sqrt_price_current,
            liquidity,
            amount_remaining,
            zero_for_one,
        )?
    } else {
        LiquidityMath::get_next_sqrt_price_from_output(
            sqrt_price_current,
            liquidity,
            amount_remaining,
            zero_for_one,
        )?
    };

    // Use the more restrictive of target price or calculated price
    let sqrt_price_next_bounded = if zero_for_one {
        sqrt_price_next.max(sqrt_price_target)
    } else {
        sqrt_price_next.min(sqrt_price_target)
    };

    // Calculate amounts based on price movement
    let amount_in = if zero_for_one {
        math_liquidity::get_amount_0_delta(sqrt_price_next_bounded, sqrt_price_current, liquidity, true)?
    } else {
        math_liquidity::get_amount_1_delta(sqrt_price_current, sqrt_price_next_bounded, liquidity, true)?
    };

    let amount_out = if zero_for_one {
        math_liquidity::get_amount_1_delta(sqrt_price_next_bounded, sqrt_price_current, liquidity, false)?
    } else {
        math_liquidity::get_amount_0_delta(sqrt_price_current, sqrt_price_next_bounded, liquidity, false)?
    };

    // Calculate fee based on the actual amount consumed in this step
    // The fee should always be calculated on amount_in (the step amount), 
    // never on amount_remaining (the total remaining for the entire swap)
    let fee_amount = ((amount_in as u128 * fee_rate as u128) / 10000) as u64;
    
    // Find the next initialized tick
    let tick_next = if sqrt_price_next_bounded == sqrt_price_target {
        TickMath::get_tick_at_sqrt_ratio(sqrt_price_target)?
    } else {
        0 // No tick crossed
    };

    Ok(SwapStep {
        sqrt_price_next: sqrt_price_next_bounded,
        sqrt_price_target,
        tick_next,
        amount_in, // Use the actual step amount, not including fees
        amount_out,
        fee_amount,
    })
}

/// Cross a tick boundary and update all related state consistently
/// 
/// This function ensures that both the swap state and pool state remain
/// consistent when crossing tick boundaries during a swap.
fn cross_tick<'info>(
    pool: &mut Pool,
    swap_state: &mut SwapState,
    tick_index: i32,
    zero_for_one: bool,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    // V128 Fix: Add validation before casting accounts
    // Find the tick array containing this tick
    for account_info in remaining_accounts.iter() {
        // Validate account is owned by the program before casting
        require!(
            account_info.owner == &crate::ID,
            PoolError::InvalidAccountOwner
        );
        
        // Validate account has expected data length for TickArray
        require!(
            account_info.data_len() == std::mem::size_of::<TickArray>() + 8,
            PoolError::InvalidTickArray
        );
        
        if let Ok(tick_array) = AccountLoader::<TickArray>::try_from(account_info) {
            let tick_array_data = tick_array.load()?;
            
            if tick_array_data.contains_tick(tick_index) {
                let tick = tick_array_data.get_tick(tick_index)?;
                
                // Calculate liquidity delta from crossing this tick
                // When crossing up (zero_for_one=false), we add liquidity_net
                // When crossing down (zero_for_one=true), we subtract liquidity_net
                let liquidity_delta = if zero_for_one {
                    -tick.liquidity_net
                } else {
                    tick.liquidity_net
                };
                
                // Update active liquidity in swap state
                let new_liquidity = if liquidity_delta >= 0 {
                    swap_state.liquidity.safe_add(liquidity_delta as u128)?
                } else {
                    swap_state.liquidity.safe_sub((-liquidity_delta) as u128)?
                };
                
                swap_state.liquidity = new_liquidity;
                
                // Immediately update pool's liquidity to maintain consistency
                // This ensures the pool state always reflects the current active liquidity
                pool.liquidity = new_liquidity;
                
                // Update pool's current tick and price to maintain complete consistency
                // This ensures all pool state fields are synchronized
                pool.current_tick = tick_index;
                pool.current_sqrt_price = swap_state.sqrt_price;
                
                break;
            }
        }
    }
    
    Ok(())
}

/// Helper struct for swap step calculations
#[derive(Debug)]
struct SwapStep {
    sqrt_price_next: u128,
    sqrt_price_target: u128,
    tick_next: i32,
    amount_in: u64,
    amount_out: u64,
    fee_amount: u64,
}

/// Data from a completed hop for event logging
#[derive(Debug)]
struct HopData {
    sqrt_price_after: u128,
    tick_after: i32,
    fees_paid: u64,
    protocol_fees: u64,
}

/// Update oracle observation with new price data
fn update_oracle_observation(
    oracle_state: &mut Account<ObservationState>,
    sqrt_price: u128,
    tick: i32,
    timestamp: i64,
) -> Result<()> {
    let current_index = oracle_state.observation_index as usize;
    let next_index = (current_index + 1) % 100;
    
    // Calculate cumulative tick properly for TWAP
    let cumulative_tick = if oracle_state.cardinality > 0 {
        // Find the most recent observation
        let prev_index = if current_index == 0 { 99 } else { current_index - 1 };
        let prev_observation = &oracle_state.observations[prev_index];
        
        if prev_observation.initialized {
            // Calculate time elapsed since last observation
            let time_elapsed = timestamp.saturating_sub(prev_observation.timestamp);
            
            // Accumulate: previous_cumulative + (current_tick * time_elapsed)
            // Using saturating arithmetic to prevent overflow
            let tick_delta = (tick as i128).saturating_mul(time_elapsed as i128);
            prev_observation.cumulative_tick.saturating_add(tick_delta)
        } else {
            // First observation, start at 0
            0i128
        }
    } else {
        // First observation, start at 0
        0i128
    };
    
    // Update observation at current index
    oracle_state.observations[current_index] = Observation {
        timestamp,
        sqrt_price_x96: sqrt_price,
        cumulative_tick,
        initialized: true,
    };
    
    // Move to next index
    oracle_state.observation_index = next_index as u16;
    oracle_state.last_update_timestamp = timestamp;
    
    // Update cardinality if we haven't filled the buffer yet
    if oracle_state.cardinality < 100 {
        oracle_state.cardinality += 1;
    }
    
    Ok(())
}

/// Execute a cross-token swap via FeelsSOL routing
/// For tokens that don't directly pair with FeelsSOL, route through two pools
pub fn execute_routed_swap_handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::ExecuteRoutedSwap<'info>>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
    sqrt_price_limit_2: Option<u128>,
) -> Result<u64> {
    require!(amount_in > 0, PoolError::InvalidAmount);
    
    let feelssol = &ctx.accounts.feelssol;
    
    // Determine routing strategy based on token types
    let route = SwapRoute::find(
        ctx.accounts.token_in_mint.key(),
        ctx.accounts.token_out_mint.key(),
        feelssol.feels_mint,
        ctx.program_id,
    );
    
    match route {
        SwapRoute::Direct(_pool_key) => {
            // Single hop swap - one of the tokens is FeelsSOL
            execute_single_hop_swap(
                &ctx,
                amount_in,
                amount_out_minimum,
                sqrt_price_limit_1,
            )
        }
        SwapRoute::TwoHop(_pool1_key, _pool2_key) => {
            // Two hop swap - neither token is FeelsSOL
            execute_two_hop_swap(
                &ctx,
                amount_in,
                amount_out_minimum,
                sqrt_price_limit_1,
                sqrt_price_limit_2.unwrap_or(0),
            )
        }
    }
}

/// Execute single hop swap (one token is FeelsSOL) using concentrated liquidity
fn execute_single_hop_swap(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
) -> Result<u64> {
    let mut pool = ctx.accounts.pool_1.load_mut()?;
    let clock = Clock::get()?;
    
    // Determine swap direction by checking which token is the input
    let token_in_mint = ctx.accounts.token_in_mint.key();
    let zero_for_one = pool.token_a_mint == token_in_mint;
    
    // Initialize swap state for this hop
    let mut swap_state = SwapState {
        amount_remaining: amount_in,
        amount_calculated: 0,
        sqrt_price: pool.current_sqrt_price,
        tick: pool.current_tick,
        fee_amount: 0,
        liquidity: pool.liquidity,
    };
    
    // Calculate fees using unified fee system
    let fee_breakdown = pool.calculate_swap_fees(amount_in)?;
    let amount_in_after_fee = amount_in.safe_sub(fee_breakdown.total_fee)?;
    
    swap_state.amount_remaining = amount_in_after_fee;
    swap_state.fee_amount = fee_breakdown.total_fee;
    
    // Execute concentrated liquidity swap using the main algorithm
    let amount_out = execute_concentrated_liquidity_swap(
        &mut swap_state,
        &mut pool,
        sqrt_price_limit_1,
        zero_for_one,
        &[], // For Phase 1, assume sufficient liquidity without tick crossing
    )?;
    
    // Validate slippage protection
    require!(amount_out >= amount_out_minimum, PoolError::SlippageExceeded);
    
    // Update pool state (already updated by execute_concentrated_liquidity_swap)
    // Update final metadata
    pool.last_update_slot = clock.slot;
    
    // Accumulate protocol fees
    if fee_breakdown.protocol_fee > 0 {
        pool.accumulate_protocol_fees(&fee_breakdown, zero_for_one)?;
    }
    
    // Update volume statistics using safe arithmetic
    if zero_for_one {
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_in as u128)?;
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_out as u128)?;
    } else {
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_in as u128)?;
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_out as u128)?;
    }
    
    // Execute token transfers for single hop swap
    execute_single_hop_transfers(ctx, amount_in, amount_out, zero_for_one)?;
    
    emit!(CrossTokenSwapEvent {
        user: ctx.accounts.user.key(),
        token_in: ctx.accounts.token_in_mint.key(),
        token_out: ctx.accounts.token_out_mint.key(),
        amount_in,
        amount_out,
        route: SwapRoute::Direct(ctx.accounts.pool_1.key()),
        intermediate_amount: None, // Single hop
        sqrt_price_after_hop1: None, // Single hop
        sqrt_price_after_final: pool.current_sqrt_price,
        tick_after_hop1: None, // Single hop
        tick_after_final: pool.current_tick,
        total_fees_paid: fee_breakdown.total_fee,
        protocol_fees_collected: fee_breakdown.protocol_fee,
        gas_used_estimate: 50_000, // Estimated for single hop
        timestamp: clock.unix_timestamp,
    });
    
    Ok(amount_out)
}

/// Execute two hop swap (neither token is FeelsSOL) using concentrated liquidity
fn execute_two_hop_swap(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
    sqrt_price_limit_2: u128,
) -> Result<u64> {
    // Step 1: Token A -> FeelsSOL (using pool_1)
    let intermediate_amount = execute_first_hop_concentrated(
        ctx, 
        amount_in, 
        sqrt_price_limit_1
    )?;
    
    // Capture hop 1 data for events
    let pool_1 = ctx.accounts.pool_1.load()?;
    let hop1_data = HopData {
        sqrt_price_after: pool_1.current_sqrt_price,
        tick_after: pool_1.current_tick,
        fees_paid: pool_1.calculate_swap_fees(amount_in)?.total_fee,
        protocol_fees: pool_1.calculate_swap_fees(amount_in)?.protocol_fee,
    };
    
    // Step 2: FeelsSOL -> Token B (using pool_2) 
    let final_amount = execute_second_hop_concentrated(
        ctx, 
        intermediate_amount, 
        sqrt_price_limit_2
    )?;
    
    // Capture hop 2 data for events
    let pool_2 = ctx.accounts.pool_2.load()?;
    let hop2_data = HopData {
        sqrt_price_after: pool_2.current_sqrt_price,
        tick_after: pool_2.current_tick,
        fees_paid: pool_2.calculate_swap_fees(intermediate_amount)?.total_fee,
        protocol_fees: pool_2.calculate_swap_fees(intermediate_amount)?.protocol_fee,
    };
    
    require!(final_amount >= amount_out_minimum, PoolError::SlippageExceeded);
    
    // Execute token transfers for two hop swap
    execute_two_hop_transfers(ctx, amount_in, intermediate_amount, final_amount)?;
    
    let clock = Clock::get()?;
    emit!(CrossTokenSwapEvent {
        user: ctx.accounts.user.key(),
        token_in: ctx.accounts.token_in_mint.key(),
        token_out: ctx.accounts.token_out_mint.key(),
        amount_in,
        amount_out: final_amount,
        route: SwapRoute::TwoHop(ctx.accounts.pool_1.key(), ctx.accounts.pool_2.key()),
        intermediate_amount: Some(intermediate_amount),
        sqrt_price_after_hop1: Some(hop1_data.sqrt_price_after),
        sqrt_price_after_final: hop2_data.sqrt_price_after,
        tick_after_hop1: Some(hop1_data.tick_after),
        tick_after_final: hop2_data.tick_after,
        total_fees_paid: hop1_data.fees_paid + hop2_data.fees_paid,
        protocol_fees_collected: hop1_data.protocol_fees + hop2_data.protocol_fees,
        gas_used_estimate: 95_000, // Estimated for two hops
        timestamp: clock.unix_timestamp,
    });
    
    Ok(final_amount)
}

/// Execute first hop of two-hop swap: Token A -> FeelsSOL
fn execute_first_hop_concentrated(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    sqrt_price_limit: u128,
) -> Result<u64> {
    let mut pool = ctx.accounts.pool_1.load_mut()?;
    
    // Determine swap direction: Token A -> FeelsSOL
    let token_in_mint = ctx.accounts.token_in_mint.key();
    let zero_for_one = pool.token_a_mint == token_in_mint;
    
    // Initialize swap state for first hop
    let mut swap_state = SwapState {
        amount_remaining: amount_in,
        amount_calculated: 0,
        sqrt_price: pool.current_sqrt_price,
        tick: pool.current_tick,
        fee_amount: 0,
        liquidity: pool.liquidity,
    };
    
    // Calculate fees and execute swap
    let fee_breakdown = pool.calculate_swap_fees(amount_in)?;
    let amount_in_after_fee = amount_in.safe_sub(fee_breakdown.total_fee)?;
    swap_state.amount_remaining = amount_in_after_fee;
    
    let amount_out = execute_concentrated_liquidity_swap(
        &mut swap_state,
        &mut pool,
        sqrt_price_limit,
        zero_for_one,
        &[], // Phase 1: assume no tick crossing needed
    )?;
    
    // Update pool state
    pool.last_update_slot = Clock::get()?.slot;
    if fee_breakdown.protocol_fee > 0 {
        pool.accumulate_protocol_fees(&fee_breakdown, zero_for_one)?;
    }
    
    // Update volume statistics
    if zero_for_one {
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_in as u128)?;
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_out as u128)?;
    } else {
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_in as u128)?;
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_out as u128)?;
    }
    
    Ok(amount_out)
}

/// Execute second hop of two-hop swap: FeelsSOL -> Token B
fn execute_second_hop_concentrated(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    sqrt_price_limit: u128,
) -> Result<u64> {
    let mut pool = ctx.accounts.pool_2.load_mut()?;
    
    // For second hop, FeelsSOL is always the input token
    let feelssol_mint = ctx.accounts.feelssol.feels_mint;
    let zero_for_one = pool.token_a_mint == feelssol_mint;
    
    // Initialize swap state for second hop
    let mut swap_state = SwapState {
        amount_remaining: amount_in,
        amount_calculated: 0,
        sqrt_price: pool.current_sqrt_price,
        tick: pool.current_tick,
        fee_amount: 0,
        liquidity: pool.liquidity,
    };
    
    // Calculate fees and execute swap
    let fee_breakdown = pool.calculate_swap_fees(amount_in)?;
    let amount_in_after_fee = amount_in.safe_sub(fee_breakdown.total_fee)?;
    swap_state.amount_remaining = amount_in_after_fee;
    
    let amount_out = execute_concentrated_liquidity_swap(
        &mut swap_state,
        &mut pool,
        sqrt_price_limit,
        zero_for_one,
        &[], // Phase 1: assume no tick crossing needed
    )?;
    
    // Update pool state
    pool.last_update_slot = Clock::get()?.slot;
    if fee_breakdown.protocol_fee > 0 {
        pool.accumulate_protocol_fees(&fee_breakdown, zero_for_one)?;
    }
    
    // Update volume statistics
    if zero_for_one {
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_in as u128)?;
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_out as u128)?;
    } else {
        pool.total_volume_1 = pool.total_volume_1.safe_add(amount_in as u128)?;
        pool.total_volume_0 = pool.total_volume_0.safe_add(amount_out as u128)?;
    }
    
    Ok(amount_out)
}

#[event]
pub struct CrossTokenSwapEvent {
    #[index]
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub route: SwapRoute,
    pub intermediate_amount: Option<u64>, // For two-hop swaps
    pub sqrt_price_after_hop1: Option<u128>, // Price after first hop
    pub sqrt_price_after_final: u128, // Final price state
    pub tick_after_hop1: Option<i32>, // Tick after first hop
    pub tick_after_final: i32, // Final tick state
    pub total_fees_paid: u64, // Sum of all fees across hops
    pub protocol_fees_collected: u64, // Protocol fees from all hops
    pub gas_used_estimate: u64, // Estimated compute units used
    pub timestamp: i64,
}

impl crate::logic::EventBase for CrossTokenSwapEvent {
    fn pool(&self) -> Pubkey {
        match self.route {
            SwapRoute::Direct(pool) => pool,
            SwapRoute::TwoHop(pool1, _pool2) => pool1, // Return first pool
        }
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn actor(&self) -> Pubkey {
        self.user
    }
}

/// Execute token transfers for single hop routed swap
#[allow(deprecated)]
fn execute_single_hop_transfers(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    amount_out: u64,
    _zero_for_one: bool,
) -> Result<()> {
    let pool = ctx.accounts.pool_1.load()?;
    
    // Derive the pool bump
    let (_, pool_bump) = Pubkey::find_program_address(
        &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &pool.fee_rate.to_le_bytes(),
        ],
        ctx.program_id,
    );
    
    // Transfer input token from user to pool
    #[allow(deprecated)]
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_in.to_account_info(),
                to: ctx.accounts.pool_1_token_in.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in,
    )?;
    
    // Transfer output token from pool to user using pool authority
    let seeds = &[
        b"pool",
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &pool.fee_rate.to_le_bytes(),
                    &[pool_bump],
    ];
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_1_token_out.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: ctx.accounts.pool_1.to_account_info(),
            },
            &[seeds],
        ),
        amount_out,
    )?;
    
    Ok(())
}

/// Execute token transfers for two hop routed swap
#[allow(deprecated)]
fn execute_two_hop_transfers(
    ctx: &Context<crate::ExecuteRoutedSwap>,
    amount_in: u64,
    intermediate_amount: u64,
    final_amount: u64,
) -> Result<()> {
    let pool_1 = ctx.accounts.pool_1.load()?;
    let pool_2 = ctx.accounts.pool_2.load()?;
    
    // Derive the pool bumps
    let (_, pool_1_bump) = Pubkey::find_program_address(
        &[
            b"pool",
            pool_1.token_a_mint.as_ref(),
            pool_1.token_b_mint.as_ref(),
            &pool_1.fee_rate.to_le_bytes(),
        ],
        ctx.program_id,
    );
    
    let (_, pool_2_bump) = Pubkey::find_program_address(
        &[
            b"pool",
            pool_2.token_a_mint.as_ref(),
            pool_2.token_b_mint.as_ref(),
            &pool_2.fee_rate.to_le_bytes(),
        ],
        ctx.program_id,
    );
    
    // Step 1: Transfer input token from user to pool_1
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_in.to_account_info(),
                to: ctx.accounts.pool_1_token_in.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in,
    )?;
    
    // Step 2: Transfer FeelsSOL from pool_1 to pool_2 (intermediate transfer)
    let seeds_1 = &[
        b"pool",
        pool_1.token_a_mint.as_ref(),
        pool_1.token_b_mint.as_ref(),
        &pool_1.fee_rate.to_le_bytes(),
        &[pool_1_bump],
    ];
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_1_token_out.to_account_info(),
                to: ctx.accounts.pool_2_token_in.to_account_info(),
                authority: ctx.accounts.pool_1.to_account_info(),
            },
            &[seeds_1],
        ),
        intermediate_amount,
    )?;
    
    // Step 3: Transfer output token from pool_2 to user
    let seeds_2 = &[
        b"pool",
        pool_2.token_a_mint.as_ref(),
        pool_2.token_b_mint.as_ref(),
        &pool_2.fee_rate.to_le_bytes(),
        &[pool_2_bump],
    ];
    
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_2_token_out.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: ctx.accounts.pool_2.to_account_info(),
            },
            &[seeds_2],
        ),
        final_amount,
    )?;
    
    Ok(())
}