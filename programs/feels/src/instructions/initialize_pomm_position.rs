//! Initialize POMM (Protocol-Owned Market Making) position
//!
//! This instruction creates a new position NFT specifically for POMM operations.
//! POMM positions are special protocol-owned positions used for automated floor liquidity.

use crate::{
    constants::MAX_POMM_POSITIONS,
    error::FeelsError,
    state::{Buffer, Market, Position, ProtocolConfig},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(position_index: u8)]
pub struct InitializePommPosition<'info> {
    /// Authority that can initialize POMM positions
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Market for this POMM position
    /// CHECK: Validated in handler to reduce stack usage
    #[account(has_one = buffer)]
    pub market: Account<'info, Market>,

    /// Buffer that will own this POMM position
    /// CHECK: Validated in handler to reduce stack usage
    pub buffer: Account<'info, Buffer>,

    /// POMM position account to initialize
    /// Uses a PDA derived from market and position index
    #[account(
        init,
        payer = authority,
        space = Position::LEN,
        seeds = [
            b"pomm_position",
            market.key().as_ref(),
            &[position_index],
        ],
        bump,
    )]
    pub pomm_position: Account<'info, Position>,

    /// Protocol config to validate authority
    /// CHECK: Validated in handler to reduce stack usage
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// System program
    pub system_program: Program<'info, System>,
}

pub fn initialize_pomm_position(
    ctx: Context<InitializePommPosition>,
    position_index: u8,
) -> Result<()> {
    // Validate position index
    require!(
        position_index < MAX_POMM_POSITIONS,
        FeelsError::InvalidPositionIndex
    );

    // Validate constraints (moved from struct to save stack space)
    require!(
        ctx.accounts.market.hub_protocol == Some(ctx.accounts.protocol_config.key()),
        FeelsError::InvalidProtocol
    );
    require!(
        ctx.accounts.buffer.market == ctx.accounts.market.key(),
        FeelsError::InvalidBuffer
    );
    require!(
        ctx.accounts.authority.key() == ctx.accounts.protocol_config.authority,
        FeelsError::InvalidAuthority
    );

    let pomm_position = &mut ctx.accounts.pomm_position;
    let market = &ctx.accounts.market;
    let buffer = &ctx.accounts.buffer;

    // Initialize POMM position with empty state
    // The actual liquidity will be added via manage_pomm_position
    pomm_position.nft_mint = Pubkey::default(); // POMM positions don't have NFTs
    pomm_position.market = market.key();
    pomm_position.owner = buffer.key(); // Buffer owns POMM positions
    pomm_position.tick_lower = 0;
    pomm_position.tick_upper = 0;
    pomm_position.liquidity = 0;
    pomm_position.fee_growth_inside_0_last_x64 = 0;
    pomm_position.fee_growth_inside_1_last_x64 = 0;
    pomm_position.tokens_owed_0 = 0;
    pomm_position.tokens_owed_1 = 0;
    pomm_position.position_bump = ctx.bumps.pomm_position;
    pomm_position.is_pomm = true; // Mark as POMM position
    pomm_position.last_updated_slot = Clock::get()?.slot;
    pomm_position.fee_growth_inside_0_last = 0;
    pomm_position.fee_growth_inside_1_last = 0;
    pomm_position.fees_owed_0 = 0;
    pomm_position.fees_owed_1 = 0;

    msg!(
        "Initialized POMM position {} for market {}",
        position_index,
        market.key()
    );

    Ok(())
}
