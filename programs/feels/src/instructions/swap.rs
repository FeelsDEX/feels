//! Unified swap instruction for the Feels Protocol concentrated liquidity AMM
//!
//! This instruction orchestrates the core swap functionality:
//! - Account validation and input parameter checks
//! - Coordination of swap execution logic (extracted to logic/swap_execution.rs)
//! - Fee processing and distribution (extracted to logic/swap_fees.rs)
//! - Final state updates and event emission

use crate::{
    constants::{MARKET_AUTHORITY_SEED, VAULT_SEED},
    error::FeelsError,
    events::{FeeSplitApplied, SwapExecuted},
    logic::{
        execute_swap_steps, SwapParams, SwapState,
        finalize_fee_state, split_and_apply_fees,
        SwapDirection,
    },
    state::{Buffer, Market, OracleState, ProtocolConfig, ProtocolToken},
    utils::{
        transfer_from_user_to_vault_unchecked, transfer_from_vault_to_user_unchecked,
        validate_amount, validate_slippage, validate_swap_route,
    },
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

// =============================================================================
// DATA STRUCTURES & TYPES
// =============================================================================

// Swap execution types are now imported from logic::swap_execution
// - SwapParams: Swap input parameters
// - SwapState: Internal swap state tracking
// - SwapResult: Final swap execution result

/// Account validation struct for swap operations
///
/// This struct defines all the accounts required for a swap and their validation
/// constraints. The Feels Protocol uses a hub-and-spoke model where all swaps
/// must go through FeelsSOL, requiring specific token account arrangements.
#[derive(Accounts)]
pub struct Swap<'info> {
    /// The user initiating the swap transaction
    /// Must be a system account (not a PDA) to prevent identity confusion attacks
    #[account(mut)]
    pub user: Signer<'info>,

    /// Source token account owned by the user (input tokens)
    /// Tokens will be transferred from this account during the swap
    #[account(
        mut,
        constraint = user_token_account_in.owner == user.key()
    )]
    pub user_token_account_in: Account<'info, TokenAccount>,

    /// Destination token account owned by the user (output tokens)
    /// Tokens will be transferred to this account after the swap
    #[account(
        mut,
        constraint = user_token_account_out.owner == user.key()
    )]
    pub user_token_account_out: Account<'info, TokenAccount>,

    /// The market account containing trading pair configuration and state
    /// All swaps occur within a specific market context
    #[account(
        mut,
        has_one = token_0,
        has_one = token_1,
        constraint = !market.reentrancy_guard @ FeelsError::ReentrancyDetected
    )]
    pub market: Account<'info, Market>,

    /// Vault holding token_0 reserves for the market
    /// Used for liquidity management and atomic transfers
    #[account(mut)]
    pub vault_0: Account<'info, TokenAccount>,

    /// Vault holding token_1 reserves for the market  
    /// Used for liquidity management and atomic transfers
    #[account(mut)]
    pub vault_1: Account<'info, TokenAccount>,

    /// Market buffer account for fee collection and protocol operations
    /// Accumulates fees and manages protocol-owned liquidity
    #[account(mut)]
    pub buffer: UncheckedAccount<'info>,

    /// Oracle state for TWAP price tracking
    /// Used for fee calculations and market monitoring
    #[account(mut)]
    pub oracle: UncheckedAccount<'info>,

    /// Global protocol configuration
    /// Contains fee rates, limits, and protocol parameters
    pub protocol_config: UncheckedAccount<'info>,

    /// Clock sysvar for timestamp access
    pub clock: Sysvar<'info, Clock>,

    /// Token_0 mint account
    pub token_0: UncheckedAccount<'info>,

    /// Token_1 mint account  
    pub token_1: UncheckedAccount<'info>,

    /// Input token mint (either token_0 or token_1)
    pub token_in: UncheckedAccount<'info>,

    /// Output token mint (either token_0 or token_1)
    pub token_out: UncheckedAccount<'info>,

    /// Market authority PDA
    /// CHECK: Validated as PDA in handler
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump = market.market_authority_bump,
    )]
    pub market_authority: UncheckedAccount<'info>,

    /// SPL Token program
    pub token_program: Program<'info, Token>,

    /// Treasury account for protocol fees (optional - can be None)
    /// Used only when protocol fees are being collected
    #[account(mut)]
    pub treasury: Option<UncheckedAccount<'info>>,

    /// Protocol token account for creator fees (optional)
    /// Used only for protocol-minted tokens with creator fees
    pub protocol_token: Option<Account<'info, ProtocolToken>>,

    /// Creator token account for creator fees (optional)
    /// Used only when creator fees are being distributed
    #[account(mut)]
    pub creator_token_account: Option<UncheckedAccount<'info>>,
}

// =============================================================================
// VALIDATION FUNCTIONS
// =============================================================================

/// Validate all unchecked accounts
#[inline(never)]
fn validate_swap_accounts(ctx: &Context<Swap>) -> Result<()> {
    let market = &ctx.accounts.market;

    // Validate vaults
    let vault_0_pda = Pubkey::create_program_address(
        &[
            VAULT_SEED,
            market.token_0.as_ref(),
            market.token_1.as_ref(),
            b"0",
            &[market.vault_0_bump],
        ],
        ctx.program_id,
    )
    .map_err(|_| FeelsError::InvalidPDA)?;
    require!(
        vault_0_pda == ctx.accounts.vault_0.key(),
        FeelsError::InvalidVault
    );

    let vault_1_pda = Pubkey::create_program_address(
        &[
            VAULT_SEED,
            market.token_0.as_ref(),
            market.token_1.as_ref(),
            b"1",
            &[market.vault_1_bump],
        ],
        ctx.program_id,
    )
    .map_err(|_| FeelsError::InvalidPDA)?;
    require!(
        vault_1_pda == ctx.accounts.vault_1.key(),
        FeelsError::InvalidVault
    );

    // Validate buffer
    let (buffer_pda, _) =
        Pubkey::find_program_address(&[b"buffer", market.key().as_ref()], ctx.program_id);
    require!(
        buffer_pda == ctx.accounts.buffer.key(),
        FeelsError::InvalidBuffer
    );

    // Validate oracle
    let (oracle_pda, _) =
        Pubkey::find_program_address(&[b"oracle", market.key().as_ref()], ctx.program_id);
    require!(
        oracle_pda == ctx.accounts.oracle.key(),
        FeelsError::InvalidOracle
    );

    Ok(())
}

/// Validate swap inputs and determine swap direction
#[inline(never)]
fn validate_swap_inputs(
    ctx: &Context<Swap>,
    params: &SwapParams,
    market: &Market,
) -> Result<(Pubkey, Pubkey, bool, SwapDirection)> {
    // Basic parameter validation
    validate_amount(params.amount_in)?;
    validate_slippage(params.minimum_amount_out, params.amount_in)?;

    // Validate fee parameters
    if params.max_total_fee_bps > 0 {
        require!(
            market.base_fee_bps <= params.max_total_fee_bps,
            FeelsError::FeeTooHigh
        );
    }

    // Determine input/output tokens and validate hub-and-spoke routing
    let token_in = ctx.accounts.token_in.key();
    let token_out = ctx.accounts.token_out.key();

    // Validate routing constraints (hub-and-spoke model)
    // In hub-and-spoke model, one of the market tokens must be FeelsSOL
    let feelssol_mint = if market.token_0 < market.token_1 {
        market.token_0 // FeelsSOL is always token_0 in hub-and-spoke
    } else {
        market.token_1 // Should never happen, but handle it
    };
    validate_swap_route(token_in, token_out, feelssol_mint)?;

    // Determine swap direction
    let is_token_0_to_1 = token_in == market.token_0;
    let direction = if is_token_0_to_1 {
        SwapDirection::ZeroForOne
    } else {
        SwapDirection::OneForZero
    };

    Ok((token_in, token_out, is_token_0_to_1, direction))
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
    // Validate all unchecked accounts
    validate_swap_accounts(&ctx)?;

    // Load the unchecked accounts manually to avoid lifetime issues
    let buffer_data = ctx.accounts.buffer.try_borrow_data()?;
    let mut buffer: Buffer = Buffer::try_deserialize(&mut &buffer_data[8..])?;

    let oracle_data = ctx.accounts.oracle.try_borrow_data()?;
    let mut oracle: OracleState = OracleState::try_deserialize(&mut &oracle_data[8..])?;

    let protocol_config_data = ctx.accounts.protocol_config.try_borrow_data()?;
    let protocol_config: ProtocolConfig =
        ProtocolConfig::try_deserialize(&mut &protocol_config_data[8..])?;

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

    // --- SWAP EXECUTION ---

    // Initialize swap state tracking
    let swap_state = SwapState::new(
        params.amount_in,
        ctx.accounts.market.sqrt_price,
        ctx.accounts.market.current_tick,
        ctx.accounts.market.liquidity,
    );

    // Execute the core swap logic (extracted to logic module)
    let market_key = ctx.accounts.market.key();
    let final_state = execute_swap_steps(
        ctx.remaining_accounts,
        &market_key,
        &params,
        &ctx.accounts.market,
        &mut buffer,
        swap_state,
        direction,
        is_token_0_to_1,
        ctx.accounts.market.jit_enabled, // Use market's JIT config
        &ctx.accounts.user.key(),
    )?;

    // Create swap result for transfer processing
    let amount_in_used = params
        .amount_in
        .checked_sub(final_state.amount_remaining)
        .ok_or(FeelsError::MathOverflow)?;

    let swap_execution_result = final_state.to_result(ctx.accounts.market.current_tick, params.amount_in);

    // --- FEE PROCESSING ---

    // Split and apply fees (extracted to logic module)
    let fee_split = split_and_apply_fees(
        &ctx.accounts.market,
        &mut buffer,
        &protocol_config,
        ctx.accounts.protocol_token.as_ref(),
        swap_execution_result.total_fee_paid,
        if is_token_0_to_1 { 0 } else { 1 },
    )?;

    // --- TOKEN TRANSFERS ---

    // Transfer input tokens from user to vault
    let (vault_in, vault_out) = if is_token_0_to_1 {
        (&ctx.accounts.vault_0, &ctx.accounts.vault_1)
    } else {
        (&ctx.accounts.vault_1, &ctx.accounts.vault_0)
    };

    transfer_from_user_to_vault_unchecked(
        &ctx.accounts.user_token_account_in.to_account_info(),
        &vault_in.to_account_info(),
        &ctx.accounts.user,
        &ctx.accounts.token_program,
        amount_in_used,
    )?;

    // Transfer output tokens from vault to user
    let market_key = ctx.accounts.market.key();
    let authority_seeds = &[
        MARKET_AUTHORITY_SEED,
        market_key.as_ref(),
        &[ctx.accounts.market.market_authority_bump],
    ];

    transfer_from_vault_to_user_unchecked(
        &vault_out.to_account_info(),
        &ctx.accounts.user_token_account_out.to_account_info(),
        &ctx.accounts.market_authority.to_account_info(),
        &ctx.accounts.token_program,
        &[authority_seeds],
        swap_execution_result.amount_out,
    )?;

    // --- STATE UPDATES ---

    // Update market state
    ctx.accounts.market.sqrt_price = swap_execution_result.final_sqrt_price;
    ctx.accounts.market.current_tick = swap_execution_result.final_tick;
    ctx.accounts.market.liquidity = swap_execution_result.final_liquidity;
    ctx.accounts.market.fee_growth_global_0 = ctx
        .accounts
        .market
        .fee_growth_global_0
        .checked_add(swap_execution_result.fee_growth_global_delta_0)
        .ok_or(FeelsError::MathOverflow)?;
    ctx.accounts.market.fee_growth_global_1 = ctx
        .accounts
        .market
        .fee_growth_global_1
        .checked_add(swap_execution_result.fee_growth_global_delta_1)
        .ok_or(FeelsError::MathOverflow)?;

    // Update oracle with new price observation
    oracle.update(swap_execution_result.final_tick, ctx.accounts.clock.unix_timestamp)?;

    // Finalize fee-related state (extracted to logic module)
    finalize_fee_state(
        &mut ctx.accounts.market,
        &mut buffer,
        swap_execution_result.jit_consumed_quote as u64,
        swap_execution_result.base_fees_skipped,
        is_token_0_to_1,
        &ctx.accounts.clock,
    )?;

    // Clear reentrancy guard
    ctx.accounts.market.reentrancy_guard = false;

    // --- EVENTS ---

    // Emit fee split event
    emit!(FeeSplitApplied {
        market: ctx.accounts.market.key(),
        base_fee_bps: 30,  // Default base fee
        impact_fee_bps: 0, // Will be computed by impact fee system
        total_fee_bps: 30, // For now, same as base
        fee_denom_mint: token_out,
        fee_amount: swap_execution_result.total_fee_paid,
        to_buffer_amount: fee_split.buffer_amount,
        to_treasury_amount: fee_split.protocol_amount,
        to_creator_amount: fee_split.creator_amount,
        jit_consumed_quote: 0, // Will be added when JIT is implemented
        timestamp: ctx.accounts.clock.unix_timestamp,
    });

    // Emit main swap event
    emit!(SwapExecuted {
        market: ctx.accounts.market.key(),
        user: ctx.accounts.user.key(),
        token_in,
        token_out,
        amount_in: amount_in_used,
        amount_out: swap_execution_result.amount_out,
        fee_paid: swap_execution_result.total_fee_paid,
        base_fee_paid: fee_split.protocol_amount,
        impact_bps: 0, // Will be computed by impact fee system
        sqrt_price_after: swap_execution_result.final_sqrt_price,
        timestamp: ctx.accounts.clock.unix_timestamp,
        version: 1,
    });

    Ok(())
}
