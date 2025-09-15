//! Unified swap instruction for the Feels Protocol concentrated liquidity AMM
//!
//! This instruction implements the core swap functionality with:
//! - Hub-and-spoke routing (all swaps go through FeelsSOL)
//! - Dynamic impact-based fees with multi-tier distribution
//! - Fee routing to Buffer (τ), Treasury (protocol), and Creator (optional)
//! - Protocol-owned market making (POMM) integration
//! - Floor ratchet mechanism for price protection
//! - JIT liquidity provisioning (experimental)

use crate::{
    constants::{MARKET_AUTHORITY_SEED, MAX_TICKS_CROSSED, VAULT_SEED},
    error::FeelsError,
    events::{FeeSplitApplied, SwapExecuted},
    logic::fees::{calculate_impact_bps, combine_base_and_impact},
    logic::{
        compute_swap_step, maybe_pomm_add_liquidity, update_fee_growth_segment, StepOutcome,
        SwapContext, SwapDirection, TickArrayIterator, MAX_SWAP_STEPS,
    },
    state::{Buffer, FeeDomain, Market, OracleState, ProtocolConfig, ProtocolToken, TICK_ARRAY_SIZE},
    utils::{
        apply_liquidity_net, sqrt_price_from_tick, tick_from_sqrt_price,
        transfer_from_user_to_vault_unchecked, transfer_from_vault_to_user_unchecked,
        validate_amount, validate_slippage, validate_swap_route,
    },
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

// =============================================================================
// DATA STRUCTURES & TYPES
// =============================================================================

/// Swap state tracking during execution
#[derive(Debug)]
struct SwapState {
    pub amount_remaining: u64,
    pub amount_out: u64,
    pub total_fee_paid: u64,
    pub sqrt_price: u128,
    pub current_tick: i32,
    pub liquidity: u128,
    pub ticks_crossed: u8,
    pub steps_taken: u16,
    pub fee_growth_global_delta_0: u128,
    pub fee_growth_global_delta_1: u128,
    pub jit_consumed_quote: u64,
}

/// Final swap result for transfer and fee processing
#[derive(Debug)]
struct SwapResult {
    pub amount_out: u64,
    pub total_fee_paid: u64,
    pub start_tick: i32,
    pub final_tick: i32,
    pub final_sqrt_price: u128,
    pub final_liquidity: u128,
    pub fee_growth_global_delta_0: u128,
    pub fee_growth_global_delta_1: u128,
    pub jit_consumed_quote: u64,
}

/// Parameters for swap execution
///
/// These parameters control swap behavior including slippage protection,
/// tick crossing limits, and fee caps for user protection.
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SwapParams {
    /// Amount of input token to swap (gross amount before fees)
    pub amount_in: u64,
    /// Minimum amount of output token to receive (after all fees)
    /// Used for slippage protection
    pub minimum_amount_out: u64,
    /// Maximum number of ticks to cross during swap (0 = unlimited)
    /// Prevents compute unit exhaustion and potential griefing
    pub max_ticks_crossed: u8,
    /// Maximum total fee in basis points (0 = no cap)
    /// Provides user protection against excessive fees
    pub max_total_fee_bps: u16,
}

/// Account validation struct for swap operations
///
/// This struct defines all the accounts required for a swap and their validation
/// constraints. The Feels Protocol uses a hub-and-spoke model where all swaps
/// must go through FeelsSOL, requiring specific token account arrangements.
#[derive(Accounts)]
pub struct Swap<'info> {
    /// The user initiating the swap transaction
    /// Must be a system account (not a PDA) to prevent identity confusion attacks
    #[account(
        mut,
        constraint = user.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub user: Signer<'info>,

    /// The market account containing trading pair state
    /// Must be initialized, not paused, and not currently in a reentrant call
    #[account(
        mut,
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
        constraint = !market.is_paused @ FeelsError::MarketPaused,
        constraint = !market.reentrancy_guard @ FeelsError::ReentrancyDetected,
    )]
    pub market: Account<'info, Market>,

    /// Protocol-owned vault holding token_0 reserves
    /// PDA derived from market tokens with deterministic ordering
    #[account(
        mut,
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"0"],
        bump = market.vault_0_bump,
    )]
    pub vault_0: Account<'info, TokenAccount>,

    /// Protocol-owned vault holding token_1 reserves
    /// PDA derived from market tokens with deterministic ordering
    #[account(
        mut,
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"1"],
        bump = market.vault_1_bump,
    )]
    pub vault_1: Account<'info, TokenAccount>,

    /// Market authority PDA that controls vault operations
    /// Used as signer for transferring tokens from vaults to users
    /// CHECK: PDA signer for all market operations
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump,
    )]
    pub market_authority: AccountInfo<'info>,

    /// Buffer account for fee collection and protocol-owned market making
    /// Accumulates impact fees for later deployment as liquidity
    #[account(
        mut,
        constraint = buffer.market == market.key() @ FeelsError::InvalidBuffer,
    )]
    pub buffer: Account<'info, Buffer>,

    /// Oracle account for tracking time-weighted average prices (TWAP)
    /// Updated on every swap to maintain accurate price history
    #[account(
        mut,
        seeds = [b"oracle", market.key().as_ref()],
        bump = market.oracle_bump,
    )]
    pub oracle: Account<'info, OracleState>,

    /// User's token account for the input token being swapped
    /// Ownership and mint validation performed in handler
    /// CHECK: Validated in handler
    #[account(mut)]
    pub user_token_in: UncheckedAccount<'info>,

    /// User's token account for the output token being received
    /// Ownership and mint validation performed in handler  
    /// CHECK: Validated in handler
    #[account(mut)]
    pub user_token_out: UncheckedAccount<'info>,

    /// SPL Token program for executing transfers
    pub token_program: Program<'info, Token>,

    /// Clock sysvar for timestamp and epoch tracking
    pub clock: Sysvar<'info, Clock>,

    /// Protocol configuration account for fee rates
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// Protocol treasury token account (mandatory for protocol fees)
    /// CHECK: Validated against protocol_config.treasury in handler
    #[account(mut)]
    pub protocol_treasury: UncheckedAccount<'info>,

    /// Protocol token registry entry (optional - only for protocol-minted tokens)
    /// CHECK: Validated in handler if present
    pub protocol_token: Option<Account<'info, ProtocolToken>>,

    /// Creator token account (optional - only if creator fees > 0 and protocol token present)
    /// CHECK: Validated against protocol_token.creator in handler
    #[account(mut)]
    pub creator_token_account: Option<UncheckedAccount<'info>>,
}

// =============================================================================
// VALIDATION FUNCTIONS
// =============================================================================

/// Validate swap inputs and user accounts
///
/// Performs comprehensive validation of swap parameters, user token accounts,
/// and ensures the swap is valid within the hub-and-spoke routing model.
fn validate_swap_inputs(
    ctx: &Context<'_, '_, '_, '_, Swap<'_>>,
    params: &SwapParams,
    market: &Market,
) -> Result<(Pubkey, Pubkey, bool, SwapDirection)> {
    // Validate swap amount is reasonable (prevents dust/overflow attacks)
    validate_amount(params.amount_in)?;

    // Validate tick crossing limit to prevent compute exhaustion griefing
    if params.max_ticks_crossed > 0 {
        require!(
            params.max_ticks_crossed <= MAX_TICKS_CROSSED,
            FeelsError::TooManyTicksCrossed
        );
    }

    // Ensure protocol vaults match the market configuration
    require!(
        ctx.accounts.vault_0.mint == market.token_0,
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_1.mint == market.token_1,
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_0.owner == ctx.accounts.market_authority.key(),
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_1.owner == ctx.accounts.market_authority.key(),
        FeelsError::InvalidVault
    );

    // Deserialize and validate user token accounts
    let user_token_in = TokenAccount::try_deserialize(
        &mut &ctx.accounts.user_token_in.to_account_info().data.borrow()[..],
    )?;
    let user_token_out = TokenAccount::try_deserialize(
        &mut &ctx.accounts.user_token_out.to_account_info().data.borrow()[..],
    )?;

    // Verify user owns both token accounts
    require!(
        user_token_in.owner == ctx.accounts.user.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        user_token_out.owner == ctx.accounts.user.key(),
        FeelsError::InvalidAuthority
    );

    // Verify token accounts are for the correct mints
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

    // Extract token mints for route validation
    let token_in = user_token_in.mint;
    let token_out = user_token_out.mint;

    // Ensure this is a valid token pair for this market
    require!(
        (token_in == market.token_0 && token_out == market.token_1)
            || (token_in == market.token_1 && token_out == market.token_0),
        FeelsError::InvalidMint
    );

    // Validate hub-and-spoke routing (must go through FeelsSOL)
    let _route = validate_swap_route(token_in, token_out, market.feelssol_mint)?;

    // Determine swap direction based on input token
    let (is_token_0_to_1, direction) = if token_in == market.token_0 {
        (true, SwapDirection::ZeroForOne)
    } else {
        (false, SwapDirection::OneForZero)
    };

    Ok((token_in, token_out, is_token_0_to_1, direction))
}

// =============================================================================
// EXECUTION FUNCTIONS
// =============================================================================

/// Initialize JIT (Just-In-Time) liquidity provisioning
///
/// Sets up experimental JIT liquidity boost by reserving quote tokens from the buffer
/// and calculating ephemeral liquidity to add for this swap only.
fn initialize_jit_liquidity(
    market: &Market,
    buffer: &mut Account<Buffer>,
    current_tick: i32,
    sqrt_price: u128,
    direction: SwapDirection,
    swap_ctx: &mut SwapContext,
) -> Result<u64> {
    let mut jit_consumed_quote: u64 = 0;

    if !market.jit_enabled {
        return Ok(jit_consumed_quote);
    }

    // Initialize JIT budget tracking
    #[allow(unused_mut)]
    let mut _jit_budget = Some(crate::logic::jit::JitBudget::begin(
        buffer,
        Clock::get()?.slot,
        market.jit_per_swap_q_bps,
        market.jit_per_slot_q_bps,
    ));

    // Calculate safe ask tick to prevent floor violations
    let safe_ask_tick = market.floor_tick.saturating_add(market.floor_buffer_ticks);
    let floor_guard_ok = current_tick >= safe_ask_tick;

    // Reserve JIT quote tokens for liquidity boost (if conditions met)
    if let Some(b) = &mut _jit_budget {
        if floor_guard_ok {
            let desired_q = b.per_swap_cap_q;
            let used_q = b.reserve(buffer, desired_q);
            jit_consumed_quote = used_q.min(u128::from(u64::MAX)) as u64;
        }
    }

    // Apply ephemeral JIT liquidity boost for this swap
    if jit_consumed_quote > 0 {
        // Calculate contrarian tick to place JIT liquidity
        let sqrt_current = sqrt_price;
        let neighbor_tick = match direction {
            SwapDirection::ZeroForOne => current_tick.saturating_sub(market.tick_spacing as i32),
            SwapDirection::OneForZero => current_tick.saturating_add(market.tick_spacing as i32),
        };
        let sqrt_neighbor = sqrt_price_from_tick(neighbor_tick)?;
        let width = if sqrt_neighbor > sqrt_current {
            sqrt_neighbor - sqrt_current
        } else {
            sqrt_current - sqrt_neighbor
        };

        if width > 0 {
            // Calculate liquidity from quote amount: L ≈ amount1 * Q64 / (sqrt_p - sqrt_pl)
            let l_from_quote = ((jit_consumed_quote as u128) << 64) / width;
            // Cap boost to 5% of existing liquidity to prevent excessive impact
            let cap = market.liquidity / 20;
            let jit_liq_boost = l_from_quote.min(cap);
            // Apply temporary boost to swap context only
            swap_ctx.liquidity = swap_ctx.liquidity.saturating_add(jit_liq_boost);
        }
    }

    Ok(jit_consumed_quote)
}

/// Execute the core swap loop with tick array traversal
///
/// Iterates through tick arrays, computing swap steps and crossing ticks as needed.
/// Returns the final swap state after execution.
fn execute_swap_steps<'info>(
    ctx: &Context<'_, '_, 'info, 'info, Swap<'info>>,
    params: &SwapParams,
    market: &Market,
    mut swap_state: SwapState,
    direction: SwapDirection,
    is_token_0_to_1: bool,
) -> Result<SwapState> {
    // Pre-compute boundary sqrt prices to avoid recalculation
    let floor_lower_sqrt = sqrt_price_from_tick(market.global_lower_tick)?;
    let floor_upper_sqrt = sqrt_price_from_tick(market.global_upper_tick)?;

    // Initialize tick array iterator for traversing liquidity
    let tick_arrays = TickArrayIterator::new(
        ctx.remaining_accounts,
        swap_state.current_tick,
        market.tick_spacing,
        direction,
        &ctx.accounts.market.key(),
    )?;

    // Create swap execution context
    let mut swap_ctx = SwapContext::new(
        direction,
        swap_state.sqrt_price,
        swap_state.liquidity,
        market.base_fee_bps,
        market.global_lower_tick,
        market.global_upper_tick,
        market.tick_spacing,
    );

    // Initialize JIT liquidity if enabled
    let jit_consumed_quote = initialize_jit_liquidity(
        market,
        &mut ctx.accounts.buffer.clone(),
        swap_state.current_tick,
        swap_state.sqrt_price,
        direction,
        &mut swap_ctx,
    )?;

    // Execute swap in discrete steps, crossing ticks as needed
    while swap_state.amount_remaining > 0 && swap_state.steps_taken < MAX_SWAP_STEPS {
        swap_state.steps_taken += 1;

        // Check user-specified tick crossing limit
        if params.max_ticks_crossed > 0 && swap_state.ticks_crossed >= params.max_ticks_crossed {
            break;
        }

        // Enforce protocol-level tick crossing limit to prevent griefing
        require!(
            swap_state.ticks_crossed < MAX_TICKS_CROSSED,
            FeelsError::TooManyTicksCrossed
        );

        // Find next initialized tick and precompute target sqrt price
        let next_tick_result = tick_arrays.next_initialized_tick(swap_state.current_tick)?;
        let (target_tick_opt, target_sqrt_price) = match next_tick_result {
            Some((tick, _array)) => {
                let target_sqrt = sqrt_price_from_tick(tick)?;
                (Some(tick), target_sqrt)
            }
            None => {
                // No more initialized ticks found - check for missing coverage
                let expected_array_start = match direction {
                    SwapDirection::ZeroForOne => {
                        ((swap_state.current_tick - 1)
                            / (TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32))
                            * TICK_ARRAY_SIZE as i32
                            * market.tick_spacing as i32
                    }
                    SwapDirection::OneForZero => {
                        ((swap_state.current_tick + 1)
                            / (TICK_ARRAY_SIZE as i32 * market.tick_spacing as i32))
                            * TICK_ARRAY_SIZE as i32
                            * market.tick_spacing as i32
                    }
                };

                let at_bound = match direction {
                    SwapDirection::ZeroForOne => {
                        swap_state.current_tick <= market.global_lower_tick
                    }
                    SwapDirection::OneForZero => {
                        swap_state.current_tick >= market.global_upper_tick
                    }
                };

                if !at_bound
                    && tick_arrays
                        .find_array_for_tick(expected_array_start)?
                        .is_none()
                {
                    #[cfg(feature = "telemetry")]
                    msg!(
                        "Missing tick array coverage: expected start index {} for spacing {}",
                        expected_array_start,
                        market.tick_spacing
                    );
                    return Err(FeelsError::MissingTickArrayCoverage.into());
                }

                // Use precomputed boundary prices
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
            swap_state.amount_remaining,
        )?;

        // Update swap state
        swap_state.amount_remaining = swap_state
            .amount_remaining
            .saturating_sub(step.gross_in_used);
        swap_state.amount_out = swap_state.amount_out.saturating_add(step.out);
        swap_state.total_fee_paid = swap_state.total_fee_paid.saturating_add(step.fee);
        swap_state.sqrt_price = step.sqrt_next;
        swap_ctx.sqrt_price = step.sqrt_next;

        // Update fee growth for this segment before crossing tick
        if step.fee > 0 && swap_state.liquidity > 0 {
            let segment_fee_growth =
                update_fee_growth_segment(step.fee, swap_state.liquidity, is_token_0_to_1)?;

            // Add to the appropriate token's delta based on swap direction
            if is_token_0_to_1 {
                swap_state.fee_growth_global_delta_0 = swap_state
                    .fee_growth_global_delta_0
                    .checked_add(segment_fee_growth)
                    .ok_or(FeelsError::MathOverflow)?;
            } else {
                swap_state.fee_growth_global_delta_1 = swap_state
                    .fee_growth_global_delta_1
                    .checked_add(segment_fee_growth)
                    .ok_or(FeelsError::MathOverflow)?;
            }
        }

        // Handle step outcome with simplified branching
        match step.outcome {
            StepOutcome::ReachedTarget => {
                if let Some(crossed_tick_idx) = step.crossed_tick {
                    swap_state.current_tick = crossed_tick_idx;
                    swap_state.ticks_crossed += 1;

                    // Find the tick array containing this tick and update liquidity
                    if let Some((_, array_loader)) = next_tick_result {
                        let mut array = array_loader.load_mut()?;

                        // Get liquidity_net before calling flip_fee_growth_outside
                        let liquidity_net = {
                            let tick =
                                array.get_tick(swap_state.current_tick, market.tick_spacing)?;
                            tick.liquidity_net
                        };

                        // Flip fee growth outside using effective globals with accumulated deltas
                        let effective_global_0 = market
                            .fee_growth_global_0_x64
                            .checked_add(swap_state.fee_growth_global_delta_0)
                            .ok_or(FeelsError::MathOverflow)?;
                        let effective_global_1 = market
                            .fee_growth_global_1_x64
                            .checked_add(swap_state.fee_growth_global_delta_1)
                            .ok_or(FeelsError::MathOverflow)?;

                        array.flip_fee_growth_outside(
                            swap_state.current_tick,
                            market.tick_spacing,
                            effective_global_0,
                            effective_global_1,
                        )?;

                        // Update liquidity using the helper
                        swap_state.liquidity =
                            apply_liquidity_net(direction, swap_state.liquidity, liquidity_net)?;
                        require!(swap_state.liquidity > 0, FeelsError::InsufficientLiquidity);

                        // Update context liquidity
                        swap_ctx.liquidity = swap_state.liquidity;
                    }
                }
            }
            StepOutcome::PartialAtBound => {
                // Hit a bound - update tick to the bound tick and stop
                swap_state.current_tick = match direction {
                    SwapDirection::ZeroForOne => market.global_lower_tick,
                    SwapDirection::OneForZero => market.global_upper_tick,
                };
                break;
            }
            StepOutcome::PartialByAmount => {
                // Amount exhausted, will do final tick update after loop
            }
        }
    }

    // Check if we hit the maximum steps guard
    require!(
        swap_state.steps_taken < MAX_SWAP_STEPS || swap_state.amount_remaining == 0,
        FeelsError::TooManySteps
    );

    // Final tick update - compute current tick from final sqrt price
    swap_state.current_tick = tick_from_sqrt_price(swap_state.sqrt_price)?;
    swap_state.jit_consumed_quote = jit_consumed_quote;

    Ok(swap_state)
}

// =============================================================================
// POST-PROCESSING FUNCTIONS
// =============================================================================

/// Split and apply impact fees according to the protocol fee distribution model
///
/// Distributes fees among Buffer (τ mechanism), treasury (protocol fees), 
/// and creator (for protocol-minted tokens) based on protocol configuration.
/// 
/// Fee Distribution:
/// - Buffer: Remaining amount after protocol and creator fees (for POMM)
/// - Treasury: Protocol fee percentage (mandatory, configurable)
/// - Creator: Creator fee percentage (optional, only for protocol-minted tokens)
fn split_and_apply_fees(
    _market: &Market,
    buffer: &mut Account<Buffer>,
    protocol_config: &Account<ProtocolConfig>,
    protocol_token: Option<&Account<ProtocolToken>>,
    fee_amount: u64,
    token_index: usize,
) -> Result<(u64, u64, u64)> {
    if fee_amount == 0 {
        return Ok((0, 0, 0));
    }

    // Calculate fee splits based on protocol configuration
    let protocol_fee_rate = protocol_config.default_protocol_fee_rate; // e.g., 1000 = 10%
    let creator_fee_rate = if protocol_token.is_some() { 
        protocol_config.default_creator_fee_rate 
    } else { 
        0 
    };
    
    let protocol_amount = (fee_amount as u128 * protocol_fee_rate as u128 / 10_000) as u64;
    let creator_amount = (fee_amount as u128 * creator_fee_rate as u128 / 10_000) as u64;
    let buffer_amount = fee_amount.saturating_sub(protocol_amount).saturating_sub(creator_amount);

    // Apply buffer fees (remaining amount after protocol and creator fees)
    buffer.collect_fee(buffer_amount, token_index, FeeDomain::Spot)?;
    
    // Return amounts for transfer processing in main handler
    // Protocol and creator amounts will be transferred by the caller
    Ok((buffer_amount, protocol_amount, creator_amount))
}

/// Calculate the current candidate floor tick based on market position
///
/// The floor is set as a buffer distance below the current tick to allow for
/// natural price movement while preventing excessive downside.
fn current_candidate_floor(market: &Market) -> Result<i32> {
    // Placeholder remains: subtract buffer from current tick.
    // Note: A state-driven floor calculation using pool reserves and circulating
    // supply can be wired by reading Buffer and vault balances here.
    Ok(market.current_tick.saturating_sub(market.floor_buffer_ticks))
}

/// Execute floor ratchet mechanism to protect against excessive downside
///
/// The floor ratchet prevents the market from falling too far below recent highs,
/// providing some protection for long-term liquidity providers. It operates on
/// a cooldown timer to prevent excessive manipulation.
fn do_floor_ratchet(market: &mut Account<Market>, clock: &Sysvar<Clock>) -> Result<()> {
    let old_floor = market.floor_tick;

    // Check if cooldown period has passed
    if clock
        .unix_timestamp
        .saturating_sub(market.last_floor_ratchet_ts)
        >= market.floor_cooldown_secs
    {
        let candidate = current_candidate_floor(market)?;

        // Only ratchet up, never down
        if candidate > market.floor_tick {
            market.floor_tick = candidate;
            market.last_floor_ratchet_ts = clock.unix_timestamp;

            emit!(crate::events::FloorRatcheted {
                market: market.key(),
                old_floor_tick: old_floor,
                new_floor_tick: market.floor_tick,
                timestamp: clock.unix_timestamp,
            });
        }
    }
    Ok(())
}

// =============================================================================
// MAIN SWAP HANDLER
// =============================================================================

/// Primary swap execution handler
///
/// This function orchestrates the complete swap process by delegating to
/// specialized helper functions for each major phase of swap execution.
#[allow(unused_assignments)]
pub fn swap<'info>(
    ctx: Context<'_, '_, 'info, 'info, Swap<'info>>,
    params: SwapParams,
) -> Result<()> {
    // Set reentrancy guard to prevent recursive calls
    ctx.accounts.market.reentrancy_guard = true;

    // Validate inputs and extract swap direction information
    let (token_in, token_out, is_token_0_to_1, direction) =
        validate_swap_inputs(&ctx, &params, &ctx.accounts.market)?;

    // Ensure market has liquidity and current price is within bounds
    require!(
        ctx.accounts.market.liquidity > 0,
        FeelsError::InsufficientLiquidity
    );
    require!(
        ctx.accounts.market.current_tick >= ctx.accounts.market.global_lower_tick
            && ctx.accounts.market.current_tick <= ctx.accounts.market.global_upper_tick,
        FeelsError::InvalidPrice
    );

    // Check and advance epoch if needed (time-based market phases)
    if ctx
        .accounts
        .market
        .epoch_due(ctx.accounts.clock.unix_timestamp)
    {
        ctx.accounts.market.epoch_number += 1;
        ctx.accounts.market.last_epoch_update = ctx.accounts.clock.unix_timestamp;

        // Emit epoch event for telemetry builds
        #[cfg(feature = "telemetry")]
        {
            emit!(crate::events::EpochBumped {
                market: ctx.accounts.market.key(),
                old_epoch: ctx.accounts.market.epoch_number - 1,
                new_epoch: ctx.accounts.market.epoch_number,
                timestamp: ctx.accounts.clock.unix_timestamp,
                version: 1,
            });
        }
    }

    // --- SWAP EXECUTION

    // Initialize swap state tracking
    let swap_state = SwapState {
        amount_remaining: params.amount_in,
        amount_out: 0,
        total_fee_paid: 0,
        sqrt_price: ctx.accounts.market.sqrt_price,
        current_tick: ctx.accounts.market.current_tick,
        liquidity: ctx.accounts.market.liquidity,
        ticks_crossed: 0,
        steps_taken: 0,
        fee_growth_global_delta_0: 0,
        fee_growth_global_delta_1: 0,
        jit_consumed_quote: 0,
    };

    // Execute the core swap logic
    let final_state = execute_swap_steps(
        &ctx,
        &params,
        &ctx.accounts.market,
        swap_state,
        direction,
        is_token_0_to_1,
    )?;

    // Create swap result for transfer processing
    let swap_result = SwapResult {
        amount_out: final_state.amount_out,
        total_fee_paid: final_state.total_fee_paid,
        start_tick: ctx.accounts.market.current_tick, // Original tick before swap
        final_tick: final_state.current_tick,
        final_sqrt_price: final_state.sqrt_price,
        final_liquidity: final_state.liquidity,
        fee_growth_global_delta_0: final_state.fee_growth_global_delta_0,
        fee_growth_global_delta_1: final_state.fee_growth_global_delta_1,
        jit_consumed_quote: final_state.jit_consumed_quote,
    };

    // --- TRANSFERS AND STATE UPDATES

    // Execute token transfers and fee distribution
    {
        // Calculate post-swap impact fee (MVP: base + impact; impact applied on output)
        let impact_bps = calculate_impact_bps(swap_result.start_tick, swap_result.final_tick);
        let (total_fee_bps, impact_only_bps) =
            combine_base_and_impact(ctx.accounts.market.base_fee_bps, impact_bps);

        // Enforce optional caller-provided fee cap
        if params.max_total_fee_bps > 0 {
            require!(
                total_fee_bps <= params.max_total_fee_bps,
                FeelsError::FeeCapExceeded
            );
        }

        let impact_fee_amount = ((swap_result.amount_out as u128)
            .saturating_mul(impact_only_bps as u128)
            / 10_000u128) as u64;
        let amount_out_net = swap_result.amount_out.saturating_sub(impact_fee_amount);

        // Check slippage against net amount
        validate_slippage(amount_out_net, params.minimum_amount_out)?;

        // Execute transfers
        let (vault_in, vault_out) = if is_token_0_to_1 {
            (
                &ctx.accounts.vault_0.to_account_info(),
                &ctx.accounts.vault_1.to_account_info(),
            )
        } else {
            (
                &ctx.accounts.vault_1.to_account_info(),
                &ctx.accounts.vault_0.to_account_info(),
            )
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
        let market_authority_bump = ctx.accounts.market.market_authority_bump;
        let market_key = ctx.accounts.market.key();
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
            amount_out_net,
        )?;

        // Apply impact fee distribution: Buffer + Treasury (mandatory) + Creator (optional)
        let token_index = if is_token_0_to_1 { 1 } else { 0 };

        // Determine if this is a protocol token for creator fees
        let protocol_token = if let Some(pt) = &ctx.accounts.protocol_token {
            let expected_token = if is_token_0_to_1 { token_in } else { token_out };
            if pt.mint == expected_token { Some(pt) } else { None }
        } else { None };

        let (to_buffer, to_treasury, to_creator) = split_and_apply_fees(
            &ctx.accounts.market,
            &mut ctx.accounts.buffer,
            &ctx.accounts.protocol_config,
            protocol_token,
            impact_fee_amount,
            token_index,
        )?;

        // Execute treasury transfer (always required if protocol fees > 0)
        if to_treasury > 0 {
            // Validate treasury account matches protocol config
            let treasury_token_account = TokenAccount::try_deserialize(
                &mut &ctx.accounts.protocol_treasury.to_account_info().data.borrow()[..]
            )?;
            require!(
                treasury_token_account.owner == ctx.accounts.protocol_config.treasury,
                FeelsError::InvalidAuthority
            );
            require!(
                treasury_token_account.mint == token_out,
                FeelsError::InvalidMint
            );
            
            // Transfer protocol fees to treasury
            transfer_from_vault_to_user_unchecked(
                vault_out,
                &ctx.accounts.protocol_treasury.to_account_info(),
                &ctx.accounts.market_authority,
                &ctx.accounts.token_program,
                signer_seeds,
                to_treasury,
            )?;
        }

        // Execute creator transfer if creator fee > 0  
        if to_creator > 0 && ctx.accounts.creator_token_account.is_some() && protocol_token.is_some() {
            // Validate creator account matches protocol token creator
            let creator_account = ctx.accounts.creator_token_account.as_ref().unwrap();
            let creator_token_account = TokenAccount::try_deserialize(
                &mut &creator_account.to_account_info().data.borrow()[..]
            )?;
            require!(
                creator_token_account.owner == protocol_token.unwrap().creator,
                FeelsError::InvalidAuthority
            );
            require!(
                creator_token_account.mint == token_out,
                FeelsError::InvalidMint
            );
            
            // Transfer creator fees
            transfer_from_vault_to_user_unchecked(
                vault_out,
                &creator_account.to_account_info(),
                &ctx.accounts.market_authority,
                &ctx.accounts.token_program,
                signer_seeds,
                to_creator,
            )?;
        }

        // Emit fee split event
        emit!(FeeSplitApplied {
            market: ctx.accounts.market.key(),
            base_fee_bps: ctx.accounts.market.base_fee_bps,
            impact_fee_bps: impact_only_bps,
            total_fee_bps,
            fee_denom_mint: token_out,
            fee_amount: impact_fee_amount,
            to_buffer_amount: to_buffer,
            to_treasury_amount: to_treasury,
            to_creator_amount: to_creator,
            jit_consumed_quote: swap_result.jit_consumed_quote,
            timestamp: ctx.accounts.clock.unix_timestamp,
        });
    }

    // Update global fee growth in market state
    ctx.accounts.market.fee_growth_global_0_x64 = ctx
        .accounts
        .market
        .fee_growth_global_0_x64
        .checked_add(swap_result.fee_growth_global_delta_0)
        .ok_or(FeelsError::MathOverflow)?;
    ctx.accounts.market.fee_growth_global_1_x64 = ctx
        .accounts
        .market
        .fee_growth_global_1_x64
        .checked_add(swap_result.fee_growth_global_delta_1)
        .ok_or(FeelsError::MathOverflow)?;

    // Update final market state
    ctx.accounts.market.sqrt_price = swap_result.final_sqrt_price;
    ctx.accounts.market.current_tick = swap_result.final_tick;
    ctx.accounts.market.liquidity = swap_result.final_liquidity;

    // Always update the oracle
    ctx.accounts
        .oracle
        .update(swap_result.final_tick, ctx.accounts.clock.unix_timestamp)?;

    // --- POST-SWAP OPERATIONS

    // Execute floor ratchet mechanism to protect against downside
    do_floor_ratchet(&mut ctx.accounts.market, &ctx.accounts.clock)?;

    // Emit swap execution event
    emit!(SwapExecuted {
        market: ctx.accounts.market.key(),
        user: ctx.accounts.user.key(),
        token_in,
        token_out,
        amount_in: params.amount_in,
        amount_out: swap_result.amount_out.saturating_sub(
            // Subtract impact fee to show net amount user received
            ((swap_result.amount_out as u128
                * calculate_impact_bps(swap_result.start_tick, swap_result.final_tick) as u128)
                / 10_000u128) as u64
        ),
        fee_paid: swap_result.total_fee_paid,
        base_fee_paid: swap_result.total_fee_paid,
        sqrt_price_after: swap_result.final_sqrt_price,
        timestamp: ctx.accounts.clock.unix_timestamp,
        version: 2, // Version 2 indicates refactored engine
    });

    // Execute protocol-owned market maker maintenance if enabled
    maybe_pomm_add_liquidity(
        &mut ctx.accounts.market,
        &mut ctx.accounts.buffer,
        &ctx.accounts.oracle,
        ctx.accounts.clock.unix_timestamp,
    )?;

    // Clear reentrancy guard and complete swap
    ctx.accounts.market.reentrancy_guard = false;
    Ok(())
}
