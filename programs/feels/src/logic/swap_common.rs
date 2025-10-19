//! Common swap utilities shared between regular and exact output swaps
//!
//! This module contains shared validation, transfer, fee distribution, and state
//! update logic used by both swap and swap_exact_out instructions.

use crate::{
    constants::{BASIS_POINTS_DIVISOR, MARKET_AUTHORITY_SEED},
    error::FeelsError,
    events::SwapExecuted,
    state::{Buffer, Market, OracleState},
    utils::{
        transfer_from_user_to_vault_unchecked, transfer_from_vault_to_user_unchecked,
        validate_amount,
    },
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

/// Common result structure for swap operations
#[derive(Debug, Clone)]
pub struct SwapResult {
    /// Amount of input token consumed
    pub amount_in: u64,
    /// Amount of output token received
    pub amount_out: u64,
    /// Total fee paid (base + impact)
    pub fee_amount: u64,
    /// Base fee portion
    pub base_fee: u64,
    /// Impact fee portion
    pub impact_fee: u64,
    /// Final sqrt price after swap
    pub sqrt_price_after: u128,
    /// Final tick after swap
    pub tick_after: i32,
    /// Price impact in basis points
    pub impact_bps: u16,
    /// Whether swap moved from token_0 to token_1
    pub is_token_0_to_1: bool,
}

/// Validate swap parameters and accounts
pub fn validate_swap_params(
    amount: u64,
    max_ticks_crossed: u8,
    user_account_0: &TokenAccount,
    user_account_1: &TokenAccount,
    market: &Market,
    user_key: Pubkey,
) -> Result<bool> {
    // Validate amount
    validate_amount(amount)?;

    // Validate user owns the accounts
    require!(
        user_account_0.owner == user_key,
        FeelsError::InvalidAuthority
    );
    require!(
        user_account_1.owner == user_key,
        FeelsError::InvalidAuthority
    );

    // Validate market state
    require!(!market.is_paused, FeelsError::MarketPaused);
    require!(!market.reentrancy_guard, FeelsError::ReentrancyDetected);

    // Determine swap direction
    let is_token_0_to_1 = user_account_0.mint == market.token_0;
    let is_token_1_to_0 = user_account_1.mint == market.token_0;

    require!(
        is_token_0_to_1 ^ is_token_1_to_0,
        FeelsError::InvalidSwapDirection
    );

    // Validate tick crossing limit if specified
    if max_ticks_crossed > 0 {
        require!(max_ticks_crossed <= 100, FeelsError::InvalidParameter);
    }

    Ok(is_token_0_to_1)
}

/// Execute token transfers for a swap
#[allow(clippy::too_many_arguments)]
pub fn execute_swap_transfers<'info>(
    user_src: &Account<'info, TokenAccount>,
    user_dst: &Account<'info, TokenAccount>,
    vault_src: &Account<'info, TokenAccount>,
    vault_dst: &Account<'info, TokenAccount>,
    user: &Signer<'info>,
    market_authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    market_key: Pubkey,
    market_authority_bump: u8,
    amount_in: u64,
    amount_out: u64,
) -> Result<()> {
    // Transfer input from user to vault
    transfer_from_user_to_vault_unchecked(
        &user_src.to_account_info(),
        &vault_src.to_account_info(),
        user,
        token_program,
        amount_in,
    )?;

    // Transfer output from vault to user
    let market_authority_seeds = &[
        MARKET_AUTHORITY_SEED,
        market_key.as_ref(),
        &[market_authority_bump],
    ];
    transfer_from_vault_to_user_unchecked(
        &vault_dst.to_account_info(),
        &user_dst.to_account_info(),
        market_authority,
        token_program,
        &[market_authority_seeds],
        amount_out,
    )
}

/// Distribute fees to buffer, treasury, and creator
pub fn distribute_swap_fees(
    buffer: &mut Buffer,
    base_fee: u64,
    impact_fee: u64,
    is_token_0_to_1: bool,
    treasury_allocation_bps: u16,
    creator_allocation_bps: u16,
) -> Result<(u64, u64, u64)> {
    let total_fee = base_fee.saturating_add(impact_fee);

    // Calculate allocations
    let treasury_amount = (total_fee as u128)
        .saturating_mul(treasury_allocation_bps as u128)
        .checked_div(BASIS_POINTS_DIVISOR as u128)
        .unwrap_or(0) as u64;

    let creator_amount = (total_fee as u128)
        .saturating_mul(creator_allocation_bps as u128)
        .checked_div(BASIS_POINTS_DIVISOR as u128)
        .unwrap_or(0) as u64;

    let buffer_amount = total_fee
        .saturating_sub(treasury_amount)
        .saturating_sub(creator_amount);

    // Update buffer fees
    if is_token_0_to_1 {
        buffer.fees_token_0 = buffer.fees_token_0.saturating_add(buffer_amount as u128);
    } else {
        buffer.fees_token_1 = buffer.fees_token_1.saturating_add(buffer_amount as u128);
    }

    // Update buffer tau values
    buffer.tau_spot = buffer.tau_spot.saturating_add(base_fee as u128);
    buffer.tau_leverage = buffer.tau_leverage.saturating_add(impact_fee as u128);

    Ok((buffer_amount, treasury_amount, creator_amount))
}

/// Update market state after swap
pub fn update_market_state(market: &mut Market, result: &SwapResult, clock: &Clock) -> Result<()> {
    // Update price and tick
    market.sqrt_price = result.sqrt_price_after;
    market.current_tick = result.tick_after;

    // Update volume tracking
    if result.is_token_0_to_1 {
        market.total_volume_token_0 = market.total_volume_token_0.saturating_add(result.amount_in);
    } else {
        market.total_volume_token_1 = market.total_volume_token_1.saturating_add(result.amount_in);
    }

    // Update last snapshot timestamp
    market.last_snapshot_timestamp = clock.unix_timestamp;

    Ok(())
}

/// Update oracle state with new price observation
pub fn update_oracle_state(oracle: &mut OracleState, tick: i32, timestamp: i64) -> Result<()> {
    oracle.update(tick, timestamp)
}

/// Emit swap executed event
pub fn emit_swap_event(
    result: &SwapResult,
    market_key: Pubkey,
    user_key: Pubkey,
    token_in: Pubkey,
    token_out: Pubkey,
    timestamp: i64,
) -> Result<()> {
    emit!(SwapExecuted {
        market: market_key,
        user: user_key,
        token_in,
        token_out,
        amount_in: result.amount_in,
        amount_out: result.amount_out,
        fee_paid: result.fee_amount,
        base_fee_paid: result.base_fee,
        impact_bps: result.impact_bps,
        sqrt_price_after: result.sqrt_price_after,
        timestamp,
        version: 2,
    });

    msg!(
        "Swap executed: {} {} -> {} {} (fee: {} bps, impact: {} bps)",
        result.amount_in,
        if result.is_token_0_to_1 {
            "token0"
        } else {
            "token1"
        },
        result.amount_out,
        if result.is_token_0_to_1 {
            "token1"
        } else {
            "token0"
        },
        result.fee_amount * 10000 / result.amount_in,
        result.impact_bps
    );

    Ok(())
}

/// Calculate slippage protection for exact output swaps
pub fn validate_slippage_exact_out(amount_in_required: u64, maximum_amount_in: u64) -> Result<()> {
    require!(
        amount_in_required <= maximum_amount_in,
        FeelsError::SlippageExceeded
    );
    Ok(())
}

/// Calculate slippage protection for regular swaps
pub fn validate_slippage(amount_out_received: u64, minimum_amount_out: u64) -> Result<()> {
    require!(
        amount_out_received >= minimum_amount_out,
        FeelsError::SlippageExceeded
    );
    Ok(())
}

/// Validate fee cap if specified
pub fn validate_fee_cap(fee_amount: u64, swap_amount: u64, max_total_fee_bps: u16) -> Result<()> {
    if max_total_fee_bps > 0 {
        let fee_bps = (fee_amount as u128)
            .saturating_mul(10000)
            .checked_div(swap_amount as u128)
            .unwrap_or(0) as u16;

        require!(fee_bps <= max_total_fee_bps, FeelsError::FeeCapExceeded);
    }
    Ok(())
}

/// Common swap account getter to reduce duplication
pub struct SwapAccounts<'info> {
    pub user_src: &'info Account<'info, TokenAccount>,
    pub user_dst: &'info Account<'info, TokenAccount>,
    pub vault_src: &'info Account<'info, TokenAccount>,
    pub vault_dst: &'info Account<'info, TokenAccount>,
}

/// Get swap accounts based on direction
pub fn get_swap_accounts<'info>(
    user_account_0: &'info Account<'info, TokenAccount>,
    user_account_1: &'info Account<'info, TokenAccount>,
    vault_0: &'info Account<'info, TokenAccount>,
    vault_1: &'info Account<'info, TokenAccount>,
    is_token_0_to_1: bool,
) -> SwapAccounts<'info> {
    if is_token_0_to_1 {
        SwapAccounts {
            user_src: user_account_0,
            user_dst: user_account_1,
            vault_src: vault_0,
            vault_dst: vault_1,
        }
    } else {
        SwapAccounts {
            user_src: user_account_1,
            user_dst: user_account_0,
            vault_src: vault_1,
            vault_dst: vault_0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_distribution() {
        use crate::state::Buffer;

        let mut buffer = Buffer {
            market: Pubkey::default(),
            authority: Pubkey::default(),
            feelssol_mint: Pubkey::default(),
            fees_token_0: 0,
            fees_token_1: 0,
            tau_spot: 0,
            tau_time: 0,
            tau_leverage: 0,
            floor_tick_spacing: 0,
            floor_placement_threshold: 0,
            last_floor_placement: 0,
            last_rebase: 0,
            total_distributed: 0,
            buffer_authority_bump: 0,
            jit_last_slot: 0,
            jit_slot_used_q: 0,
            jit_rolling_consumption: 0,
            jit_rolling_window_start: 0,
            jit_last_heavy_usage_slot: 0,
            jit_total_consumed_epoch: 0,
            initial_tau_spot: 0,
            protocol_owned_override: 0,
            pomm_position_count: 0,
            _padding: [0; 7],
        };

        let (buffer_amt, treasury_amt, creator_amt) =
            distribute_swap_fees(&mut buffer, 1000, 500, true, 1000, 500).unwrap();

        assert_eq!(buffer_amt + treasury_amt + creator_amt, 1500);
        assert_eq!(treasury_amt, 150); // 10% of 1500
        assert_eq!(creator_amt, 75); // 5% of 1500
        assert_eq!(buffer_amt, 1275); // Rest goes to buffer
    }

    #[test]
    fn test_slippage_validation() {
        // Regular swap slippage
        assert!(validate_slippage(100, 90).is_ok());
        assert!(validate_slippage(90, 100).is_err());

        // Exact out slippage
        assert!(validate_slippage_exact_out(100, 110).is_ok());
        assert!(validate_slippage_exact_out(110, 100).is_err());
    }

    #[test]
    fn test_fee_cap_validation() {
        // 100 bps fee on 10000 amount
        assert!(validate_fee_cap(100, 10000, 200).is_ok());
        assert!(validate_fee_cap(100, 10000, 50).is_err());

        // No cap (0 = unlimited)
        assert!(validate_fee_cap(1000, 10000, 0).is_ok());
    }
}
