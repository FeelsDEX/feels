use anchor_lang::prelude::*;
use crate::{
    state::{PoolRegistry, ProtocolConfig},
    error::FeelsError,
};

#[derive(Accounts)]
pub struct InitializePoolRegistry<'info> {
    /// Protocol config must exist
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    /// Pool registry to initialize
    #[account(
        init,
        payer = payer,
        space = PoolRegistry::INITIAL_SIZE,
        seeds = [PoolRegistry::SEED],
        bump,
    )]
    pub pool_registry: Account<'info, PoolRegistry>,
    
    /// Authority must match protocol authority
    #[account(
        mut,
        constraint = authority.key() == protocol_config.authority @ FeelsError::InvalidAuthority
    )]
    pub authority: Signer<'info>,
    
    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

pub fn initialize_pool_registry(ctx: Context<InitializePoolRegistry>) -> Result<()> {
    let registry = &mut ctx.accounts.pool_registry;
    
    registry.authority = ctx.accounts.protocol_config.authority;
    registry.pool_count = 0;
    registry.pools = Vec::new();
    registry.bump = ctx.bumps.pool_registry;
    registry._reserved = [0; 128];
    
    Ok(())
}