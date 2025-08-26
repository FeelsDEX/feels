use crate::{
    error::ProtocolError,
    events::{AuthorityTransferAccepted, AuthorityTransferCancelled, AuthorityTransferInitiated},
    instructions::AUTHORITY_TRANSFER_DELAY,
    state::{protocol::ProtocolState, treasury::Treasury},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitiateAuthorityTransfer<'info> {
    #[account(
        mut,
        seeds = [b"protocol"],
        bump,
        has_one = authority @ ProtocolError::InvalidAuthority
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: This is the proposed new authority, validated by the current authority
    pub new_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CancelAuthorityTransfer<'info> {
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

#[derive(Accounts)]
pub struct AcceptAuthorityTransfer<'info> {
    #[account(
        mut,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol_state: Account<'info, ProtocolState>,

    #[account(
        mut,
        seeds = [b"treasury"],
        bump
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(mut)]
    pub new_authority: Signer<'info>,
}

pub fn initiate_authority_transfer(ctx: Context<InitiateAuthorityTransfer>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    let clock = Clock::get()?;

    // Ensure no pending transfer exists
    require!(
        protocol_state.pending_authority.is_none(),
        ProtocolError::PendingAuthorityTransferExists
    );

    let new_authority_key = ctx.accounts.new_authority.key();
    let transfer_initiated_at = clock.unix_timestamp;
    let transfer_can_be_accepted_at = transfer_initiated_at + AUTHORITY_TRANSFER_DELAY;

    // Set pending authority transfer
    protocol_state.pending_authority = Some(new_authority_key);
    protocol_state.authority_transfer_initiated_at = Some(transfer_initiated_at);
    protocol_state.last_updated = transfer_initiated_at;

    emit!(AuthorityTransferInitiated {
        current_authority: protocol_state.authority,
        new_authority: new_authority_key,
        initiated_at: transfer_initiated_at,
        can_be_accepted_at: transfer_can_be_accepted_at,
    });

    Ok(())
}

pub fn cancel_authority_transfer(ctx: Context<CancelAuthorityTransfer>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    let clock = Clock::get()?;

    // Ensure there's a pending transfer to cancel
    let cancelled_authority = protocol_state
        .pending_authority
        .ok_or(ProtocolError::NoPendingAuthorityTransfer)?;
    let cancelled_at = clock.unix_timestamp;

    // Clear pending authority transfer
    protocol_state.pending_authority = None;
    protocol_state.authority_transfer_initiated_at = None;
    protocol_state.last_updated = cancelled_at;

    emit!(AuthorityTransferCancelled {
        current_authority: protocol_state.authority,
        cancelled_authority,
        cancelled_at,
    });

    Ok(())
}

pub fn accept_authority_transfer(ctx: Context<AcceptAuthorityTransfer>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;

    // Ensure there's a pending transfer
    let pending_authority = protocol_state
        .pending_authority
        .ok_or(ProtocolError::NoPendingAuthorityTransfer)?;

    // Ensure the signer is the pending new authority
    require!(
        pending_authority == ctx.accounts.new_authority.key(),
        ProtocolError::NotPendingAuthority
    );

    // Ensure the delay period has passed
    let initiated_at = protocol_state
        .authority_transfer_initiated_at
        .ok_or(ProtocolError::NoPendingAuthorityTransfer)?;
    let current_time = clock.unix_timestamp;
    require!(
        current_time >= initiated_at + AUTHORITY_TRANSFER_DELAY,
        ProtocolError::AuthorityTransferDelayNotMet
    );

    let old_authority = protocol_state.authority;
    let new_authority = ctx.accounts.new_authority.key();

    // Transfer authority
    protocol_state.authority = new_authority;
    protocol_state.pending_authority = None;
    protocol_state.authority_transfer_initiated_at = None;
    protocol_state.last_updated = current_time;

    // Update treasury authority
    treasury.authority = new_authority;

    emit!(AuthorityTransferAccepted {
        old_authority,
        new_authority,
        accepted_at: current_time,
    });

    Ok(())
}
