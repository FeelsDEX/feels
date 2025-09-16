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

    #[account(
        mut,
        has_one = buffer,
        constraint = market.hub_protocol == Some(protocol_config.key()) @ FeelsError::InvalidProtocol,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        constraint = buffer.market == market.key() @ FeelsError::InvalidBuffer,
    )]
    pub buffer: Account<'info, Buffer>,

    /// POMM position account - must be initialized separately
    /// Uses a PDA derived from market and position index
    #[account(mut)]
    pub pomm_position: Account<'info, Position>,

    #[account(
        mut,
        constraint = oracle.key() == market.oracle @ FeelsError::InvalidOracle,
    )]
    pub oracle: Account<'info, OracleState>,

    #[account(
        mut,
        constraint = vault_0.key() == market.vault_0 @ FeelsError::InvalidVault,
    )]
    pub vault_0: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_1.key() == market.vault_1 @ FeelsError::InvalidVault,
    )]
    pub vault_1: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = buffer_vault_0.owner == buffer_authority.key() @ FeelsError::InvalidBufferVault,
    )]
    pub buffer_vault_0: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = buffer_vault_1.owner == buffer_authority.key() @ FeelsError::InvalidBufferVault,
    )]
    pub buffer_vault_1: Account<'info, TokenAccount>,

    /// CHECK: PDA authority for buffer vaults
    #[account(
        seeds = [b"buffer_authority", market.key().as_ref()],
        bump,
    )]
    pub buffer_authority: UncheckedAccount<'info>,

    #[account(
        constraint = protocol_config.key() == market.hub_protocol.unwrap_or_default() @ FeelsError::InvalidProtocol,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

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
    Rebalance { new_tick_lower: i32, new_tick_upper: i32 },
}

pub fn manage_pomm_position(ctx: Context<ManagePommPosition>, params: ManagePommParams) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer;
    let oracle = &ctx.accounts.oracle;
    let pomm_position = &mut ctx.accounts.pomm_position;
    
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    let current_slot = clock.slot;

    // Validate position index
    require!(
        params.position_index < MAX_POMM_POSITIONS,
        FeelsError::InvalidPositionIndex
    );

    // Check cooldown
    if now <= buffer.last_floor_placement + POMM_COOLDOWN_SECONDS {
        return Err(FeelsError::PommCooldownActive.into());
    }

    // Get TWAP price for manipulation resistance
    let twap_tick = oracle.get_twap_tick(now, POMM_TWAP_SECONDS)?;
    let twap_sqrt_price = sqrt_price_from_tick(twap_tick)?;

    match params.action {
        PommAction::AddLiquidity => {
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

            // Calculate POMM range based on market parameters
            let pomm_tick_width = (market.tick_spacing as i32)
                .saturating_mul(POMM_WIDTH_MULTIPLIER)
                .clamp(POMM_MIN_WIDTH, POMM_MAX_WIDTH);

            // Determine range based on available tokens
            let (tick_lower, tick_upper) = if amount_0 > 0 && amount_1 == 0 {
                // Only token_0: place below current price
                (twap_tick - pomm_tick_width, twap_tick)
            } else if amount_0 == 0 && amount_1 > 0 {
                // Only token_1: place above current price
                (twap_tick, twap_tick + pomm_tick_width)
            } else {
                // Both tokens: symmetric range
                (twap_tick - pomm_tick_width, twap_tick + pomm_tick_width)
            };

            // Calculate liquidity
            let sqrt_pl = sqrt_price_from_tick(tick_lower)?;
            let sqrt_pu = sqrt_price_from_tick(tick_upper)?;
            let liquidity = liquidity_from_amounts(
                twap_sqrt_price,
                sqrt_pl,
                sqrt_pu,
                amount_0,
                amount_1,
            )?;

            require!(
                liquidity >= MIN_LIQUIDITY,
                FeelsError::LiquidityBelowMinimum
            );

            // Transfer tokens from buffer vaults to market vaults
            let market_key = market.key();
            let buffer_authority_seeds = &[
                b"buffer_authority",
                market_key.as_ref(),
                &[ctx.bumps.buffer_authority],
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

            // Update position
            pomm_position.market = market.key();
            pomm_position.owner = buffer.key();
            pomm_position.tick_lower = tick_lower;
            pomm_position.tick_upper = tick_upper;
            pomm_position.liquidity = liquidity;
            pomm_position.fee_growth_inside_0_last = market.fee_growth_global_0;
            pomm_position.fee_growth_inside_1_last = market.fee_growth_global_1;
            pomm_position.fees_owed_0 = 0;
            pomm_position.fees_owed_1 = 0;
            pomm_position.is_pomm = true;
            pomm_position.last_updated_slot = current_slot;

            // Update market liquidity if in range
            if market.current_tick >= tick_lower && market.current_tick <= tick_upper {
                market.liquidity = market.liquidity
                    .checked_add(liquidity)
                    .ok_or(FeelsError::MathOverflow)?;
            }

            // Update buffer accounting
            buffer.fees_token_0 = buffer.fees_token_0.saturating_sub(amount_0 as u128);
            buffer.fees_token_1 = buffer.fees_token_1.saturating_sub(amount_1 as u128);
            buffer.tau_spot = buffer.tau_spot
                .saturating_sub((amount_0 + amount_1) as u128);
            buffer.last_floor_placement = now;
            buffer.total_distributed = buffer.total_distributed
                .saturating_add((amount_0 + amount_1) as u128);
            buffer.pomm_position_count = buffer.pomm_position_count.saturating_add(1);

            // Emit event
            emit!(PommPositionUpdated {
                market: market.key(),
                position_index: params.position_index,
                action: "add_liquidity".to_string(),
                tick_lower,
                tick_upper,
                liquidity,
                amount_0,
                amount_1,
                timestamp: now,
            });
        }

        PommAction::RemoveLiquidity { liquidity_amount } => {
            // Validate position exists and has liquidity
            require!(
                pomm_position.liquidity >= liquidity_amount,
                FeelsError::InsufficientLiquidity
            );

            // Calculate amounts to receive
            let _sqrt_pl = sqrt_price_from_tick(pomm_position.tick_lower)?;
            let _sqrt_pu = sqrt_price_from_tick(pomm_position.tick_upper)?;
            
            // This would require proper tick math to calculate amounts
            // For MVP, we'll return an error indicating this needs implementation
            return Err(FeelsError::NotImplemented.into());
        }

        PommAction::Rebalance { new_tick_lower, new_tick_upper } => {
            // Validate new range
            require!(
                new_tick_lower < new_tick_upper,
                FeelsError::InvalidTickRange
            );
            
            // This would require removing liquidity and re-adding at new range
            // For MVP, we'll return an error indicating this needs implementation
            return Err(FeelsError::NotImplemented.into());
        }
    }

    Ok(())
}

// Add new constants
pub const MAX_POMM_POSITIONS: u8 = 8;
pub const POMM_COOLDOWN_SECONDS: i64 = 60;
pub const POMM_TWAP_SECONDS: u32 = 300;
pub const POMM_WIDTH_MULTIPLIER: i32 = 20;
pub const POMM_MIN_WIDTH: i32 = 10;
pub const POMM_MAX_WIDTH: i32 = 2000;
