use crate::{
    error::ProtocolError,
    events::ProtocolInitialized,
    instructions::{MAX_POOL_FEE_RATE, MAX_PROTOCOL_FEE_RATE},
    state::{protocol::ProtocolState, treasury::Treasury},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = ProtocolState::SIZE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    #[account(
        init,
        payer = authority,
        space = Treasury::SIZE,
        seeds = [b"treasury"],
        bump
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_protocol(
    ctx: Context<Initialize>,
    default_protocol_fee_rate: u16,
    max_pool_fee_rate: u16,
) -> Result<()> {
    require!(
        default_protocol_fee_rate <= MAX_PROTOCOL_FEE_RATE,
        ProtocolError::ProtocolFeeTooHigh
    );
    require!(
        max_pool_fee_rate <= MAX_POOL_FEE_RATE,
        ProtocolError::PoolFeeTooHigh
    );

    // Get keys before creating mutable borrows
    let treasury_key = ctx.accounts.treasury.key();
    let protocol_state_key = ctx.accounts.protocol_state.key();
    let authority_key = ctx.accounts.authority.key();

    let protocol_state = &mut ctx.accounts.protocol_state;
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;

    // Initialize protocol state
    protocol_state.authority = authority_key;
    protocol_state.treasury = treasury_key;
    protocol_state.default_protocol_fee_rate = default_protocol_fee_rate;
    protocol_state.max_pool_fee_rate = max_pool_fee_rate;
    protocol_state.paused = false;
    protocol_state.pool_creation_allowed = true;
    protocol_state.total_pools = 0;
    protocol_state.total_fees_collected = 0;
    protocol_state.total_volume = 0;
    protocol_state.initialized_at = clock.unix_timestamp;
    protocol_state.last_updated = clock.unix_timestamp;
    protocol_state.pending_authority = None;
    protocol_state.authority_transfer_initiated_at = None;

    // Initialize treasury
    treasury.protocol = protocol_state_key;
    treasury.authority = authority_key;
    treasury.total_collected = 0;
    treasury.total_withdrawn = 0;
    treasury.last_withdrawal = 0;

    emit!(ProtocolInitialized {
        authority: protocol_state.authority,
        treasury: protocol_state.treasury,
        default_protocol_fee_rate,
        max_pool_fee_rate,
    });

    Ok(())
}
