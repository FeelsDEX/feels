//! Unified swap instruction

use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;
use anchor_spl::token::{Token, TokenAccount};
use crate::{
    constants::{MARKET_AUTHORITY_SEED, VAULT_SEED, MAX_TICKS_CROSSED},
    error::FeelsError,
    events::SwapExecuted,
    logic::{
        maybe_pomm_add_liquidity, 
        SwapDirection, StepOutcome, compute_swap_step, TickArrayIterator, 
        update_fee_growth_segment, MAX_SWAP_STEPS, SwapContext
    },
    state::{Market, Buffer, FeeDomain, TICK_ARRAY_SIZE, OracleState},
    utils::{
        validate_amount, validate_slippage, validate_swap_route, 
        transfer_from_user_to_vault_unchecked, transfer_from_vault_to_user_unchecked,
        sqrt_price_from_tick, tick_from_sqrt_price, apply_liquidity_net,
    },
};

/// Swap accounts
#[derive(Accounts)]
pub struct Swap<'info> {
    /// User performing the swap
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = user.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub user: Signer<'info>,
    
    /// Market account
    #[account(
        mut,
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
        constraint = !market.is_paused @ FeelsError::MarketPaused,
    )]
    pub market: Account<'info, Market>,
    
    /// Vault for token 0 - derived from market and token_0
    /// CHECK: Validated as PDA in handler
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_0.as_ref()],
        bump,
    )]
    pub vault_0: UncheckedAccount<'info>,
    
    /// Vault for token 1 - derived from market and token_1
    /// CHECK: Validated as PDA in handler
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_1.as_ref()],
        bump,
    )]
    pub vault_1: UncheckedAccount<'info>,
    
    /// Unified market authority PDA
    /// CHECK: PDA signer for all market operations
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump,
    )]
    pub market_authority: AccountInfo<'info>,
    
    /// Buffer for fee collection
    #[account(
        mut,
        constraint = buffer.market == market.key() @ FeelsError::InvalidBuffer,
    )]
    pub buffer: Account<'info, Buffer>,
    
    /// Oracle account for TWAP tracking
    #[account(
        mut,
        seeds = [b"oracle", market.key().as_ref()],
        bump = market.oracle_bump,
    )]
    pub oracle: Account<'info, OracleState>,
    
    /// User's token account for input token
    /// CHECK: Validated in handler
    #[account(mut)]
    pub user_token_in: UncheckedAccount<'info>,
    
    /// User's token account for output token
    /// CHECK: Validated in handler
    #[account(mut)]
    pub user_token_out: UncheckedAccount<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// Clock sysvar
    pub clock: Sysvar<'info, Clock>,
}

/// Swap parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SwapParams {
    /// Amount of input token to swap
    pub amount_in: u64,
    /// Minimum amount of output token to receive
    pub minimum_amount_out: u64,
    /// Optional: maximum number of ticks to cross (0 = unlimited)
    pub max_ticks_crossed: u8,
}


/// Swap handler
#[allow(unused_assignments)]
pub fn swap<'info>(
    ctx: Context<'_, '_, 'info, 'info, Swap<'info>>,
    params: SwapParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer;
    let oracle = &mut ctx.accounts.oracle;
    let clock = &ctx.accounts.clock;
    
    // Validate inputs
    validate_amount(params.amount_in)?;
    
    // Validate tick crossing limit to prevent griefing
    if params.max_ticks_crossed > 0 {
        require!(
            params.max_ticks_crossed <= MAX_TICKS_CROSSED,
            FeelsError::TooManyTicksCrossed
        );
    }
    
    // Manually deserialize vault accounts (already validated as PDAs in account constraints)
    let _vault_0 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_0.data.borrow()[..])?;
    let _vault_1 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_1.data.borrow()[..])?;
    
    // Manually deserialize and validate user token accounts
    let user_token_in = TokenAccount::try_deserialize(&mut &ctx.accounts.user_token_in.data.borrow()[..])?;
    let user_token_out = TokenAccount::try_deserialize(&mut &ctx.accounts.user_token_out.data.borrow()[..])?;
    
    // Validate user token accounts
    require!(
        user_token_in.owner == ctx.accounts.user.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        user_token_out.owner == ctx.accounts.user.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        user_token_in.mint == market.token_0 || user_token_in.mint == market.token_1,
        FeelsError::InvalidMint
    );
    require!(
        user_token_out.mint == market.token_0 || user_token_out.mint == market.token_1,
        FeelsError::InvalidMint
    );
    require!(
        user_token_out.mint != user_token_in.mint,
        FeelsError::InvalidMint
    );
    
    // Explicit mint checks to prevent surprising routes
    let token_in = user_token_in.mint;
    let token_out = user_token_out.mint;
    
    // Ensure mints match the market tokens
    require!(
        (token_in == market.token_0 && token_out == market.token_1) ||
        (token_in == market.token_1 && token_out == market.token_0),
        FeelsError::InvalidMint
    );
    
    // Check if epoch is due
    if market.epoch_due(clock.unix_timestamp) {
        market.epoch_number += 1;
        market.last_epoch_update = clock.unix_timestamp;
        
        // Only emit epoch events in debug/telemetry builds
        #[cfg(feature = "telemetry")]
        {
            emit!(crate::events::EpochBumped {
                market: market.key(),
                old_epoch: market.epoch_number - 1,
                new_epoch: market.epoch_number,
                timestamp: clock.unix_timestamp,
                version: 1,
            });
        }
    }
    
    // Validate route through FeelsSOL
    let _route = validate_swap_route(token_in, token_out, market.feelssol_mint)?;
    
    // Determine swap direction (already validated above)
    let (is_token_0_to_1, direction) = if token_in == market.token_0 {
        (true, SwapDirection::ZeroForOne)
    } else {
        (false, SwapDirection::OneForZero)
    };
    
    // Ensure market has liquidity and price is in bounds
    require!(market.liquidity > 0, FeelsError::InsufficientLiquidity);
    require!(
        market.current_tick >= market.global_lower_tick && 
        market.current_tick <= market.global_upper_tick,
        FeelsError::InvalidPrice
    );
    
    // Initialize swap state
    let mut amount_remaining = params.amount_in;
    let mut amount_out = 0u64;
    let mut total_fee_paid = 0u64;
    let mut sqrt_price = market.sqrt_price;
    let mut current_tick = market.current_tick;
    let mut liquidity = market.liquidity;
    let mut ticks_crossed = 0u8;
    let mut steps_taken = 0u16;
    
    // Cache bound sqrt prices to avoid recomputation
    let floor_lower_sqrt = sqrt_price_from_tick(market.global_lower_tick)?;
    let floor_upper_sqrt = sqrt_price_from_tick(market.global_upper_tick)?;
    
    // Create tick array iterator from remaining accounts
    let tick_arrays = TickArrayIterator::new(
        &ctx.remaining_accounts,
        current_tick,
        market.tick_spacing,
        direction,
        &market.key(),
    )?;
    
    // Track fee growth updates for per-segment accounting
    // Only the input token's delta will be non-zero
    let mut fee_growth_global_delta_0 = 0u128;
    let mut fee_growth_global_delta_1 = 0u128;
    
    // Create swap context
    let mut swap_ctx = SwapContext::new(
        direction,
        sqrt_price,
        liquidity,
        market.base_fee_bps,
        market.global_lower_tick,
        market.global_upper_tick,
        market.tick_spacing,
    );
    
    // Execute swap in steps with maximum step guard
    while amount_remaining > 0 && steps_taken < MAX_SWAP_STEPS {
        steps_taken += 1;
        // Check tick crossing limit
        if params.max_ticks_crossed > 0 && ticks_crossed >= params.max_ticks_crossed {
            break;
        }
        
        // SECURITY: Hard limit on tick crossings to prevent griefing
        // Even if user sets max_ticks_crossed to 0 (unlimited), we enforce a protocol limit
        require!(
            ticks_crossed < MAX_TICKS_CROSSED,
            FeelsError::TooManyTicksCrossed
        );
        
        // Find next initialized tick and precompute target sqrt price
        let next_tick_result = tick_arrays.next_initialized_tick(current_tick)?;
        let (target_tick_opt, target_sqrt_price) = match next_tick_result {
            Some((tick, _array)) => {
                // Precompute sqrt price for the target tick
                let target_sqrt = sqrt_price_from_tick(tick)?;
                (Some(tick), target_sqrt)
            }
            None => {
                // No more initialized ticks found
                // Check if we're missing tick array coverage
                let expected_array_start = match direction {
                    SwapDirection::ZeroForOne => {
                        ((current_tick - 1) / (TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32)) 
                            * TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32
                    }
                    SwapDirection::OneForZero => {
                        ((current_tick + 1) / (TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32)) 
                            * TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32
                    }
                };
                
                // If we're not at the bounds, this might be a missing array issue
                let at_bound = match direction {
                    SwapDirection::ZeroForOne => current_tick <= market.global_lower_tick,
                    SwapDirection::OneForZero => current_tick >= market.global_upper_tick,
                };
                
                if !at_bound && tick_arrays.find_array_for_tick(expected_array_start)?.is_none() {
                    #[cfg(feature = "telemetry")]
                    msg!("Missing tick array coverage: expected start index {} for spacing {}", expected_array_start, market.tick_spacing);
                    return Err(FeelsError::MissingTickArrayCoverage.into());
                }
                
                // Use precomputed bound sqrt prices
                match direction {
                    SwapDirection::ZeroForOne => (None, floor_lower_sqrt),
                    SwapDirection::OneForZero => (None, floor_upper_sqrt),
                }
            }
        };
        
        // Compute swap step with bound awareness
        let step = compute_swap_step(
            &swap_ctx,
            target_sqrt_price,
            target_tick_opt,
            amount_remaining,
        )?;
        
        // Update state - all fee logic is now in the engine
        amount_remaining = amount_remaining.saturating_sub(step.gross_in_used);
        amount_out = amount_out.saturating_add(step.out);
        total_fee_paid = total_fee_paid.saturating_add(step.fee);
        sqrt_price = step.sqrt_next;
        
        // Update swap context
        swap_ctx.sqrt_price = sqrt_price;
        
        // Update fee growth for this segment before crossing tick
        if step.fee > 0 && liquidity > 0 {
            let segment_fee_growth = update_fee_growth_segment(
                step.fee,
                liquidity,
                is_token_0_to_1,
            )?;
            // Add to the appropriate token's delta based on swap direction
            if is_token_0_to_1 {
                fee_growth_global_delta_0 = fee_growth_global_delta_0
                    .checked_add(segment_fee_growth)
                    .ok_or(FeelsError::MathOverflow)?;
            } else {
                fee_growth_global_delta_1 = fee_growth_global_delta_1
                    .checked_add(segment_fee_growth)
                    .ok_or(FeelsError::MathOverflow)?;
            }
        }
        
        // Handle step outcome with simplified branching
        match step.outcome {
            StepOutcome::ReachedTarget => {
                if let Some(crossed_tick_idx) = step.crossed_tick {
                    current_tick = crossed_tick_idx;
                    ticks_crossed += 1;
                    
                    // Find the tick array containing this tick
                    if let Some((_, array_loader)) = next_tick_result {
                let mut array = array_loader.load_mut()?;
                
                // Get liquidity_net before calling flip_fee_growth_outside
                let liquidity_net = {
                    let tick = array.get_tick(current_tick, market.tick_spacing)?;
                    tick.liquidity_net
                };
                
                // Flip fee growth outside
                // Use the effective globals with accumulated deltas
                let effective_global_0 = market.fee_growth_global_0_x64
                    .checked_add(fee_growth_global_delta_0)
                    .ok_or(FeelsError::MathOverflow)?;
                let effective_global_1 = market.fee_growth_global_1_x64
                    .checked_add(fee_growth_global_delta_1)
                    .ok_or(FeelsError::MathOverflow)?;
                
                array.flip_fee_growth_outside(
                    current_tick,
                    market.tick_spacing,
                    effective_global_0,
                    effective_global_1,
                )?;
                
                // Update liquidity using the helper
                liquidity = apply_liquidity_net(direction, liquidity, liquidity_net)?;
                
                        require!(liquidity > 0, FeelsError::InsufficientLiquidity);
                        
                        // Update context liquidity
                        swap_ctx.liquidity = liquidity;
                    }
                } else {
                    // No tick crossed, don't update current_tick yet (lazy update)
                }
            }
            StepOutcome::PartialAtBound => {
                // We've hit a bound - update tick to the bound tick
                current_tick = match direction {
                    SwapDirection::ZeroForOne => market.global_lower_tick,
                    SwapDirection::OneForZero => market.global_upper_tick,
                };
                // Stop the swap as we've reached the bound
                break;
            }
            StepOutcome::PartialByAmount => {
                // Amount exhausted, don't update tick here (will do final update after loop)
            }
        }
    }
    
    // Check if we hit the maximum steps guard
    require!(
        steps_taken < MAX_SWAP_STEPS || amount_remaining == 0,
        FeelsError::TooManySteps
    );
    
    // Final tick update - compute current tick from final sqrt price
    current_tick = tick_from_sqrt_price(sqrt_price)?;
    
    // Check slippage
    validate_slippage(amount_out, params.minimum_amount_out)?;
    
    // Execute transfers
    let (vault_in, vault_out) = if is_token_0_to_1 {
        (&ctx.accounts.vault_0.to_account_info(), &ctx.accounts.vault_1.to_account_info())
    } else {
        (&ctx.accounts.vault_1.to_account_info(), &ctx.accounts.vault_0.to_account_info())
    };
    
    // Transfer input tokens from user
    transfer_from_user_to_vault_unchecked(
        &ctx.accounts.user_token_in.to_account_info(),
        vault_in,
        &ctx.accounts.user,
        &ctx.accounts.token_program,
        params.amount_in,
    )?;
    
    // Transfer output tokens to user
    // Use stored bump for performance (avoids PDA derivation)
    let market_authority_bump = market.market_authority_bump;
    let market_key = market.key();
    let seeds = &[
        MARKET_AUTHORITY_SEED,
        market_key.as_ref(),
        &[market_authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    transfer_from_vault_to_user_unchecked(
        vault_out,
        &ctx.accounts.user_token_out.to_account_info(),
        &ctx.accounts.market_authority,
        &ctx.accounts.token_program,
        signer_seeds,
        amount_out,
    )?;
    
    // Update buffer with fees
    let token_index = if is_token_0_to_1 { 0 } else { 1 };
    buffer.collect_fee(total_fee_paid, token_index, FeeDomain::Spot)?;
    
    // Update global fee growth
    market.fee_growth_global_0_x64 = market.fee_growth_global_0_x64
        .checked_add(fee_growth_global_delta_0)
        .ok_or(FeelsError::MathOverflow)?;
    market.fee_growth_global_1_x64 = market.fee_growth_global_1_x64
        .checked_add(fee_growth_global_delta_1)
        .ok_or(FeelsError::MathOverflow)?;
    
    // Update market state
    market.sqrt_price = sqrt_price;
    market.current_tick = current_tick;
    market.liquidity = liquidity;
    
    // Always update the oracle (Uniswap V3 style)
    oracle.update(current_tick, clock.unix_timestamp)?;
    
    // Emit event
    emit!(SwapExecuted {
        market: market.key(),
        user: ctx.accounts.user.key(),
        token_in,
        token_out,
        amount_in: params.amount_in,
        amount_out,
        fee_paid: total_fee_paid,
        sqrt_price_after: sqrt_price,
        timestamp: clock.unix_timestamp,
        version: 2, // Version 2 indicates new engine
    });
    
    // Hook: Protocol-owned market maker maintenance
    maybe_pomm_add_liquidity(
        market,
        buffer,
        oracle,
        clock.unix_timestamp,
    )?;
    
    Ok(())
}
