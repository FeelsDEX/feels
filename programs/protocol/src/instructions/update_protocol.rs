use anchor_lang::prelude::*;

use crate::{
    error::ProtocolError,
    events::ProtocolUpdated,
    instructions::{MAX_POOL_FEE_RATE, MAX_PROTOCOL_FEE_RATE},
    state::protocol::ProtocolState,
};

#[derive(Accounts)]
pub struct UpdateProtocol<'info> {
    #[account(
        mut,
        seeds = [b"protocol"],
        bump,
        has_one = authority @ ProtocolError::InvalidAuthority
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

pub fn update_protocol(
    ctx: Context<UpdateProtocol>,
    new_default_protocol_fee_rate: Option<u16>,
    new_max_pool_fee_rate: Option<u16>,
    new_paused: Option<bool>,
    new_pool_creation_allowed: Option<bool>,
) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;

    // Validate fee rates if provided
    if let Some(fee_rate) = new_default_protocol_fee_rate {
        require!(
            fee_rate <= MAX_PROTOCOL_FEE_RATE,
            ProtocolError::ProtocolFeeTooHigh
        );
        protocol_state.default_protocol_fee_rate = fee_rate;
    }

    if let Some(fee_rate) = new_max_pool_fee_rate {
        require!(fee_rate <= MAX_POOL_FEE_RATE, ProtocolError::PoolFeeTooHigh);
        protocol_state.max_pool_fee_rate = fee_rate;
    }

    // Validate paused state if provided
    if let Some(paused) = new_paused {
        protocol_state.paused = paused;
    }

    // Validate pool creation allowed state if provided
    if let Some(pool_creation_allowed) = new_pool_creation_allowed {
        protocol_state.pool_creation_allowed = pool_creation_allowed;
    }

    // Log the update
    protocol_state.last_updated = Clock::get()?.unix_timestamp;

    emit!(ProtocolUpdated {
        authority: protocol_state.authority,
        treasury: protocol_state.treasury,
        default_protocol_fee_rate: protocol_state.default_protocol_fee_rate,
        max_pool_fee_rate: protocol_state.max_pool_fee_rate,
        paused: protocol_state.paused,
        pool_creation_allowed: protocol_state.pool_creation_allowed,
    });

    Ok(())
}
