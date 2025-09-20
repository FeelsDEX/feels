use crate::{
    error::FeelsError,
    state::{Buffer, ProtocolConfig},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetProtocolOwnedOverride<'info> {
    /// Protocol config must exist
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// Buffer to update
    /// Note: We don't check buffer.authority here because it's set to the market creator,
    /// not the protocol authority. The protocol authority can still manage overrides
    /// as a governance function.
    #[account(mut)]
    pub buffer: Account<'info, Buffer>,

    /// Protocol authority - only they can set overrides
    #[account(
        constraint = authority.key() == protocol_config.authority @ FeelsError::InvalidAuthority
    )]
    pub authority: Signer<'info>,
}

pub fn set_protocol_owned_override(
    ctx: Context<SetProtocolOwnedOverride>,
    override_amount: u64,
) -> Result<()> {
    let buffer = &mut ctx.accounts.buffer;

    // Update the protocol owned override
    // If set to 0, it disables the override and uses dynamic calculation
    // This is a governance override that allows the protocol to set a fixed
    // protocol-owned amount instead of relying on dynamic calculations
    buffer.protocol_owned_override = override_amount;

    msg!(
        "Protocol override set for buffer {} to {} tokens",
        buffer.market,
        override_amount
    );

    Ok(())
}
