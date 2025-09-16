use anchor_lang::prelude::*;
use crate::{
    error::FeelsError,
    state::{Buffer, ProtocolConfig},
};

#[derive(Accounts)]
pub struct SetProtocolOwnedOverride<'info> {
    /// Protocol config must exist
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    /// Buffer to update
    #[account(
        mut,
        constraint = buffer.authority == protocol_config.authority @ FeelsError::InvalidAuthority
    )]
    pub buffer: Account<'info, Buffer>,
    
    /// Protocol authority
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
    buffer.protocol_owned_override = override_amount;
    
    Ok(())
}