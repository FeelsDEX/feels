//! Manage POMM (Protocol-Owned Market Making) positions
//!
//! This instruction provides automated floor liquidity management through proper Position NFTs
//! instead of the semi-manual approach that overwrites global ticks.

use crate::{
    constants::*,
    error::FeelsError,
    events::PommPositionUpdated,
    state::*,
    utils::{liquidity_from_amounts, sqrt_price_from_tick, transfer_from_buffer_vault},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[derive(Accounts)]
pub struct ManagePommPosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Market account
    #[account(mut)]
    pub market: Account<'info, Market>,

    /// Buffer account  
    #[account(mut)]
    pub buffer: Account<'info, Buffer>,

    /// POMM position account - must be initialized separately
    /// Uses a PDA derived from market and position index
    #[account(mut)]
    pub pomm_position: Account<'info, Position>,

    /// CHECK: Oracle account, validated in handler
    #[account(mut)]
    pub oracle: UncheckedAccount<'info>,

    /// CHECK: Vault 0, validated in handler
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// CHECK: Vault 1, validated in handler
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// CHECK: Buffer vault 0, validated in handler
    #[account(mut)]
    pub buffer_vault_0: UncheckedAccount<'info>,

    /// CHECK: Buffer vault 1, validated in handler
    #[account(mut)]
    pub buffer_vault_1: UncheckedAccount<'info>,

    /// CHECK: PDA authority for buffer vaults, validated in handler
    pub buffer_authority: UncheckedAccount<'info>,

    /// CHECK: Protocol config, validated in handler
    pub protocol_config: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ManagePommParams {
    /// Position index (0-7 for up to 8 POMM positions)
    pub position_index: u8,
    /// Action to take
    pub action: PommAction,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum PommAction {
    /// Add liquidity from buffer fees
    AddLiquidity,
    /// Remove liquidity back to buffer
    RemoveLiquidity { liquidity_amount: u128 },
    /// Rebalance position to new range
    Rebalance {
        new_tick_lower: i32,
        new_tick_upper: i32,
    },
    /// Collect accumulated fees from position
    CollectFees,
}

#[inline(never)]
pub fn manage_pomm_position(
    ctx: Context<ManagePommPosition>,
    params: ManagePommParams,
) -> Result<()> {
    // Process immediately to minimize stack usage
    process_pomm_action(ctx, params)
}

/// Process POMM action with minimal stack footprint
#[inline(never)]
fn process_pomm_action(
    mut ctx: Context<ManagePommPosition>,
    params: ManagePommParams,
) -> Result<()> {
    // Basic validation
    require!(
        params.position_index < MAX_POMM_POSITIONS,
        FeelsError::InvalidPositionIndex
    );

    // Validate all constraints in separate function to reduce stack usage
    validate_pomm_constraints(&ctx)?;

    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    
    // Check cooldown
    if now <= ctx.accounts.buffer.last_floor_placement + POMM_COOLDOWN_SECONDS {
        return Err(FeelsError::PommCooldownActive.into());
    }

    // Load the oracle account from UncheckedAccount and get TWAP tick
    let twap_tick = {
        let oracle_data = ctx.accounts.oracle.try_borrow_data()?;
        let oracle: OracleState = OracleState::try_deserialize(&mut &oracle_data[8..])?;
        // Get TWAP price for manipulation resistance
        oracle.get_twap_tick(now, POMM_TWAP_SECONDS)?
    };

    match params.action {
        PommAction::AddLiquidity => {
            handle_add_liquidity(&mut ctx, params.position_index, twap_tick, now)?;
        }

        PommAction::RemoveLiquidity { liquidity_amount } => {
            handle_remove_liquidity(&ctx, liquidity_amount, now)?;
        }

        PommAction::Rebalance {
            new_tick_lower,
            new_tick_upper,
        } => {
            handle_rebalance(&ctx, new_tick_lower, new_tick_upper, now)?;
        }

        PommAction::CollectFees => {
            handle_collect_fees(&ctx, now)?;
        }
    }

    Ok(())
}

/// Handle adding liquidity to a POMM position
#[inline(never)]
fn handle_add_liquidity(
    ctx: &mut Context<ManagePommPosition>,
    position_index: u8,
    twap_tick: i32,
    now: i64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer;
    let pomm_position = &mut ctx.accounts.pomm_position;
    
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    let twap_sqrt_price = sqrt_price_from_tick(twap_tick)?;

    // Prevent reusing a non-empty position to avoid double-counting liquidity
    require!(
        pomm_position.liquidity == 0,
        FeelsError::PositionNotEmpty
    );
    require!(
        pomm_position.market == Pubkey::default()
            || pomm_position.market == market.key(),
        FeelsError::InvalidMarket
    );
    require!(
        pomm_position.owner == Pubkey::default()
            || pomm_position.owner == buffer.key(),
        FeelsError::InvalidAuthority
    );

    // Check if buffer has enough fees
    let threshold_u128 = buffer.floor_placement_threshold as u128;
    let total_fees = buffer.fees_token_0.saturating_add(buffer.fees_token_1);

    if total_fees < threshold_u128 {
        return Err(FeelsError::InsufficientBufferFees.into());
    }

    // Determine amounts to use
    let amount_0 = buffer.fees_token_0.min(u64::MAX as u128) as u64;
    let amount_1 = buffer.fees_token_1.min(u64::MAX as u128) as u64;

    if amount_0 == 0 && amount_1 == 0 {
        return Err(FeelsError::InsufficientBufferFees.into());
    }

    let total_amount_u128 = (amount_0 as u128).saturating_add(amount_1 as u128);
    require!(
        buffer.tau_spot >= total_amount_u128,
        FeelsError::InsufficientBufferBalance
    );

    // Calculate tick range
    let (tick_lower, tick_upper) = calculate_pomm_range(market, twap_tick, amount_0, amount_1)?;

    // Calculate liquidity
    let sqrt_pl = sqrt_price_from_tick(tick_lower)?;
    let sqrt_pu = sqrt_price_from_tick(tick_upper)?;
    let liquidity = liquidity_from_amounts(twap_sqrt_price, sqrt_pl, sqrt_pu, amount_0, amount_1)?;

    require!(
        liquidity >= MIN_LIQUIDITY,
        FeelsError::LiquidityBelowMinimum
    );

    // Transfer tokens
    let market_key = market.key();
    let (_, buffer_authority_bump) = Pubkey::find_program_address(
        &[b"buffer_authority", market_key.as_ref()],
        &crate::ID,
    );
    let buffer_authority_seeds = &[
        b"buffer_authority",
        market_key.as_ref(),
        &[buffer_authority_bump],
    ];

    if amount_0 > 0 {
        transfer_from_buffer_vault(
            &ctx.accounts.buffer_vault_0.to_account_info(),
            &ctx.accounts.vault_0.to_account_info(),
            &ctx.accounts.buffer_authority.to_account_info(),
            buffer_authority_seeds,
            amount_0,
            &ctx.accounts.token_program,
        )?;
    }

    if amount_1 > 0 {
        transfer_from_buffer_vault(
            &ctx.accounts.buffer_vault_1.to_account_info(),
            &ctx.accounts.vault_1.to_account_info(),
            &ctx.accounts.buffer_authority.to_account_info(),
            buffer_authority_seeds,
            amount_1,
            &ctx.accounts.token_program,
        )?;
    }

    // Update accounts
    update_pomm_position_state(
        pomm_position, market, buffer, position_index, tick_lower, tick_upper, 
        liquidity, amount_0, amount_1, total_amount_u128, current_slot, now
    )?;

    Ok(())
}

/// Calculate POMM position range based on available tokens
#[inline(never)]
fn calculate_pomm_range(
    market: &Market,
    twap_tick: i32,
    amount_0: u64,
    amount_1: u64,
) -> Result<(i32, i32)> {
    let pomm_tick_width = (market.tick_spacing as i32)
        .saturating_mul(POMM_WIDTH_MULTIPLIER)
        .clamp(POMM_MIN_WIDTH, POMM_MAX_WIDTH);

    let tick_spacing_i32 = market.tick_spacing as i32;
    let align_down = |value: i32| value - value.rem_euclid(tick_spacing_i32);
    let align_up = |value: i32| {
        let rem = value.rem_euclid(tick_spacing_i32);
        if rem == 0 {
            value
        } else {
            value + (tick_spacing_i32 - rem)
        }
    };

    // Determine range based on available tokens
    let (raw_lower, raw_upper) = if amount_0 > 0 && amount_1 == 0 {
        // Only token_0: place below current price
        (twap_tick - pomm_tick_width, twap_tick)
    } else if amount_0 == 0 && amount_1 > 0 {
        // Only token_1: place above current price
        (twap_tick, twap_tick + pomm_tick_width)
    } else {
        // Both tokens: symmetric range
        (twap_tick - pomm_tick_width, twap_tick + pomm_tick_width)
    };

    let tick_lower = align_down(raw_lower).max(market.global_lower_tick);
    let tick_upper = align_up(raw_upper).min(market.global_upper_tick);

    require!(tick_lower < tick_upper, FeelsError::InvalidTickRange);
    
    Ok((tick_lower, tick_upper))
}


/// Update POMM position and market state
#[inline(never)]
fn update_pomm_position_state(
    pomm_position: &mut Account<Position>,
    market: &mut Account<Market>,
    buffer: &mut Account<Buffer>,
    position_index: u8,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    amount_0: u64,
    amount_1: u64,
    total_amount_u128: u128,
    current_slot: u64,
    now: i64,
) -> Result<()> {
    // Update position
    pomm_position.market = market.key();
    pomm_position.owner = buffer.key();
    pomm_position.tick_lower = tick_lower;
    pomm_position.tick_upper = tick_upper;
    pomm_position.liquidity = liquidity;
    pomm_position.fee_growth_inside_0_last_x64 = market.fee_growth_global_0_x64;
    pomm_position.fee_growth_inside_1_last_x64 = market.fee_growth_global_1_x64;
    pomm_position.tokens_owed_0 = 0;
    pomm_position.tokens_owed_1 = 0;
    pomm_position.last_updated_slot = current_slot;

    // Update market liquidity if in range
    if market.current_tick >= tick_lower && market.current_tick <= tick_upper {
        market.liquidity = market
            .liquidity
            .checked_add(liquidity)
            .ok_or(FeelsError::MathOverflow)?;
    }

    // Update buffer accounting
    buffer.fees_token_0 = buffer.fees_token_0.saturating_sub(amount_0 as u128);
    buffer.fees_token_1 = buffer.fees_token_1.saturating_sub(amount_1 as u128);
    buffer.tau_spot = buffer
        .tau_spot
        .checked_sub(total_amount_u128)
        .ok_or(FeelsError::InsufficientBufferBalance)?;
    buffer.last_floor_placement = now;
    buffer.total_distributed = buffer
        .total_distributed
        .saturating_add(total_amount_u128);

    // Emit event
    emit!(PommPositionUpdated {
        market: market.key(),
        position_index,
        action: "add_liquidity".to_string(),
        tick_lower,
        tick_upper,
        liquidity,
        amount_0,
        amount_1,
        timestamp: now,
    });

    Ok(())
}

/// Handle removing liquidity from a POMM position
#[inline(never)]
fn handle_remove_liquidity(
    _ctx: &Context<ManagePommPosition>,
    _liquidity_amount: u128,
    _now: i64,
) -> Result<()> {
    // TODO: Implement remove liquidity logic
    Ok(())
}

/// Handle rebalancing a POMM position
#[inline(never)]
fn handle_rebalance(
    _ctx: &Context<ManagePommPosition>,
    _new_tick_lower: i32,
    _new_tick_upper: i32,
    _now: i64,
) -> Result<()> {
    // TODO: Implement rebalance logic
    Ok(())
}

/// Handle collecting fees from a POMM position
#[inline(never)]
fn handle_collect_fees(
    _ctx: &Context<ManagePommPosition>,
    _now: i64,
) -> Result<()> {
    // TODO: Implement collect fees logic
    Ok(())
}

// Add new constants
/// Validate all POMM constraints to reduce main function stack usage
#[inline(never)]
fn validate_pomm_constraints(ctx: &Context<ManagePommPosition>) -> Result<()> {
    require!(
        ctx.accounts.market.buffer == ctx.accounts.buffer.key(),
        FeelsError::InvalidBuffer
    );
    require!(
        ctx.accounts.market.hub_protocol == Some(ctx.accounts.protocol_config.key()),
        FeelsError::InvalidProtocol
    );
    require!(
        ctx.accounts.buffer.market == ctx.accounts.market.key(),
        FeelsError::InvalidBuffer
    );
    require!(
        ctx.accounts.oracle.key() == ctx.accounts.market.oracle,
        FeelsError::InvalidOracle
    );
    require!(
        ctx.accounts.vault_0.key() == ctx.accounts.market.vault_0,
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_1.key() == ctx.accounts.market.vault_1,
        FeelsError::InvalidVault
    );
    
    // Load buffer vaults to check ownership
    let buffer_vault_0_info = ctx.accounts.buffer_vault_0.to_account_info();
    let buffer_vault_0_data = buffer_vault_0_info.data.borrow();
    if buffer_vault_0_data.len() > 0 {
        let buffer_vault_0 = TokenAccount::try_deserialize(&mut &buffer_vault_0_data[..])?;
        require!(
            buffer_vault_0.owner == ctx.accounts.buffer_authority.key(),
            FeelsError::InvalidBufferVault
        );
    }
    
    let buffer_vault_1_info = ctx.accounts.buffer_vault_1.to_account_info();
    let buffer_vault_1_data = buffer_vault_1_info.data.borrow();
    if buffer_vault_1_data.len() > 0 {
        let buffer_vault_1 = TokenAccount::try_deserialize(&mut &buffer_vault_1_data[..])?;
        require!(
            buffer_vault_1.owner == ctx.accounts.buffer_authority.key(),
            FeelsError::InvalidBufferVault
        );
    }
    
    // Validate buffer authority PDA
    let (expected_buffer_authority, _bump) = Pubkey::find_program_address(
        &[b"buffer_authority", ctx.accounts.market.key().as_ref()],
        &crate::ID,
    );
    require!(
        ctx.accounts.buffer_authority.key() == expected_buffer_authority,
        FeelsError::InvalidAuthority
    );
    
    Ok(())
}

pub const MAX_POMM_POSITIONS: u8 = 8;
pub const POMM_COOLDOWN_SECONDS: i64 = 60;
pub const POMM_TWAP_SECONDS: u32 = 300;
pub const POMM_WIDTH_MULTIPLIER: i32 = 20;
pub const POMM_MIN_WIDTH: i32 = 10;
pub const POMM_MAX_WIDTH: i32 = 2000;
