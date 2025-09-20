//! Open position instruction (core logic)

use crate::{
    constants::{MIN_LIQUIDITY, POSITION_SEED},
    error::FeelsError,
    events::{PositionOperation, PositionUpdated},
    logic::{amounts_from_liquidity, calculate_position_fee_accrual},
    state::{Market, Position, TickArray},
    utils::{add_liquidity, sqrt_price_from_tick, transfer_from_user_to_vault_unchecked},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};

/// Open position accounts
#[derive(Accounts)]
pub struct OpenPosition<'info> {
    /// Liquidity provider
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = provider.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub provider: Signer<'info>,

    /// Market state
    #[account(
        mut,
        constraint = market.is_initialized @ FeelsError::MarketNotInitialized,
        constraint = !market.is_paused @ FeelsError::MarketPaused,
    )]
    pub market: Account<'info, Market>,

    /// Position mint - a simple SPL token representing ownership
    #[account(
        init,
        payer = provider,
        mint::decimals = 0,
        mint::authority = position,
        mint::freeze_authority = position,
    )]
    pub position_mint: Account<'info, Mint>,

    /// Position token account - where the position token is minted
    #[account(
        init,
        payer = provider,
        token::mint = position_mint,
        token::authority = provider,
    )]
    pub position_token_account: Account<'info, TokenAccount>,

    /// Position account (PDA) - stores all position state
    #[account(
        init,
        payer = provider,
        space = Position::LEN,
        seeds = [POSITION_SEED, position_mint.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,

    /// Provider's token account for token 0
    /// CHECK: Validated in handler
    #[account(mut)]
    pub provider_token_0: UncheckedAccount<'info>,

    /// Provider's token account for token 1
    /// CHECK: Validated in handler
    #[account(mut)]
    pub provider_token_1: UncheckedAccount<'info>,

    /// Market vault for token 0
    /// CHECK: Validated in handler
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// Market vault for token 1
    /// CHECK: Validated in handler
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// Tick array containing the lower tick
    #[account(
        mut,
        constraint = lower_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray,
    )]
    pub lower_tick_array: AccountLoader<'info, TickArray>,

    /// Tick array containing the upper tick
    #[account(
        mut,
        constraint = upper_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray,
    )]
    pub upper_tick_array: AccountLoader<'info, TickArray>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

/// Helper to validate tick arrays (moved out to reduce stack usage)
#[inline(never)]
fn validate_tick_arrays(
    lower_tick_array: &AccountLoader<TickArray>,
    upper_tick_array: &AccountLoader<TickArray>,
    tick_lower: i32,
    tick_upper: i32,
    tick_spacing: u16,
) -> Result<()> {
    let lower_array = lower_tick_array.load()?;
    let upper_array = upper_tick_array.load()?;
    crate::utils::validate_tick_array_for_tick(&lower_array, tick_lower, tick_spacing)?;
    crate::utils::validate_tick_array_for_tick(&upper_array, tick_upper, tick_spacing)?;
    Ok(())
}

/// Open position handler
pub fn open_position(
    ctx: Context<OpenPosition>,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    // Manually deserialize and validate vault accounts
    let _vault_0 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_0.data.borrow()[..])?;
    let _vault_1 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_1.data.borrow()[..])?;

    // Validate vaults match derived addresses
    let (expected_vault_0, _) =
        Market::derive_vault_address(&market.key(), &market.token_0, ctx.program_id);
    let (expected_vault_1, _) =
        Market::derive_vault_address(&market.key(), &market.token_1, ctx.program_id);
    require!(
        ctx.accounts.vault_0.key() == expected_vault_0,
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_1.key() == expected_vault_1,
        FeelsError::InvalidVault
    );

    // Manually deserialize and validate provider token accounts
    let provider_token_0 =
        TokenAccount::try_deserialize(&mut &ctx.accounts.provider_token_0.data.borrow()[..])?;
    let provider_token_1 =
        TokenAccount::try_deserialize(&mut &ctx.accounts.provider_token_1.data.borrow()[..])?;

    // Validate provider token accounts
    require!(
        provider_token_0.owner == ctx.accounts.provider.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        provider_token_1.owner == ctx.accounts.provider.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        provider_token_0.mint == market.token_0,
        FeelsError::InvalidMint
    );
    require!(
        provider_token_1.mint == market.token_1,
        FeelsError::InvalidMint
    );

    // Validate tick range and alignment
    crate::utils::validate_tick_range(tick_lower, tick_upper, market.tick_spacing)?;
    
    // Use the new parameter validation for tick range
    crate::utils::validate_tick_range_params(tick_lower, tick_upper, market.tick_spacing)?;
    
    require!(liquidity_amount > 0, FeelsError::ZeroLiquidity);

    // Check against minimum liquidity to prevent dust positions
    require!(
        liquidity_amount >= MIN_LIQUIDITY,
        FeelsError::LiquidityBelowMinimum
    );
    
    // Use the new parameter validation for liquidity amount
    crate::utils::validate_liquidity_amount(liquidity_amount)?;

    // Validate that tick arrays match the expected ticks
    validate_tick_arrays(
        &ctx.accounts.lower_tick_array,
        &ctx.accounts.upper_tick_array,
        tick_lower,
        tick_upper,
        market.tick_spacing,
    )?;

    // Calculate the amounts of token0 and token1 needed for the specified liquidity
    let sqrt_price_lower = sqrt_price_from_tick(tick_lower)?;
    let sqrt_price_upper = sqrt_price_from_tick(tick_upper)?;
    let sqrt_price_current = market.sqrt_price;

    // Use unified amount calculation function (same as swap logic)
    let (amount_0, amount_1) = amounts_from_liquidity(
        sqrt_price_current,
        sqrt_price_lower,
        sqrt_price_upper,
        liquidity_amount,
    )?;

    // Mint position token to provider (before mutating position)
    let position_bump = ctx.bumps.position;
    let position_mint_key = ctx.accounts.position_mint.key();
    let seeds = &[POSITION_SEED, position_mint_key.as_ref(), &[position_bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.position_mint.to_account_info(),
        to: ctx.accounts.position_token_account.to_account_info(),
        authority: ctx.accounts.position.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    token::mint_to(cpi_ctx, 1)?;

    // CRITICAL: Initialize ticks FIRST before reading fee growth values
    // This ensures fee_growth_outside is properly set based on tick position
    {
        let mut lower_array = ctx.accounts.lower_tick_array.load_mut()?;
        lower_array.init_tick(
            tick_lower,
            market.tick_spacing,
            market.current_tick,
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
        )?;
        lower_array.update_liquidity(
            tick_lower,
            market.tick_spacing,
            liquidity_amount as i128,
            false,
        )?;
    }
    {
        let mut upper_array = ctx.accounts.upper_tick_array.load_mut()?;
        upper_array.init_tick(
            tick_upper,
            market.tick_spacing,
            market.current_tick,
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
        )?;
        upper_array.update_liquidity(
            tick_upper,
            market.tick_spacing,
            liquidity_amount as i128,
            true,
        )?;
    }

    // NOW take fee growth snapshot after ticks are initialized
    let (fee_growth_inside_0, fee_growth_inside_1) = {
        let lower_array = ctx.accounts.lower_tick_array.load()?;
        let upper_array = ctx.accounts.upper_tick_array.load()?;
        let lower_tick = lower_array.get_tick(tick_lower, market.tick_spacing)?;
        let upper_tick = upper_array.get_tick(tick_upper, market.tick_spacing)?;

        // Calculate initial fee growth using the function
        let fee_accrual = calculate_position_fee_accrual(
            market.current_tick,
            tick_lower,
            tick_upper,
            0, // No liquidity yet for fee calculation
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
            lower_tick,
            upper_tick,
            0, // No previous fee growth
            0, // No previous fee growth
        )?;

        (
            fee_accrual.fee_growth_inside_0,
            fee_accrual.fee_growth_inside_1,
        )
    };

    // Initialize position state
    let position = &mut ctx.accounts.position;
    position.nft_mint = ctx.accounts.position_mint.key();
    position.market = market.key();
    position.owner = ctx.accounts.provider.key();
    position.tick_lower = tick_lower;
    position.tick_upper = tick_upper;
    position.liquidity = liquidity_amount;
    position.fee_growth_inside_0_last_x64 = fee_growth_inside_0;
    position.fee_growth_inside_1_last_x64 = fee_growth_inside_1;
    position.tokens_owed_0 = 0;
    position.tokens_owed_1 = 0;
    position.position_bump = ctx.bumps.position;

    // Update market liquidity if position is in range
    if market.current_tick >= tick_lower && market.current_tick < tick_upper {
        market.liquidity = add_liquidity(market.liquidity, liquidity_amount)?;
    }

    // Transfer tokens from provider to vaults
    if amount_0 > 0 {
        transfer_from_user_to_vault_unchecked(
            &ctx.accounts.provider_token_0.to_account_info(),
            &ctx.accounts.vault_0.to_account_info(),
            &ctx.accounts.provider,
            &ctx.accounts.token_program,
            amount_0,
        )?;
    }

    if amount_1 > 0 {
        transfer_from_user_to_vault_unchecked(
            &ctx.accounts.provider_token_1.to_account_info(),
            &ctx.accounts.vault_1.to_account_info(),
            &ctx.accounts.provider,
            &ctx.accounts.token_program,
            amount_1,
        )?;
    }

    // Emit unified event
    emit!(PositionUpdated {
        position: position.key(),
        position_mint: ctx.accounts.position_mint.key(),
        market: market.key(),
        owner: ctx.accounts.provider.key(),
        tick_lower,
        tick_upper,
        liquidity: liquidity_amount,
        amount_0,
        amount_1,
        fees_collected_0: 0,
        fees_collected_1: 0,
        operation: PositionOperation::Open,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
