//! Initialize protocol configuration
//! 
//! One-time setup instruction to initialize global protocol parameters

use anchor_lang::prelude::*;
use crate::{
    error::FeelsError,
    state::ProtocolConfig,
};

/// Initialize protocol parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeProtocolParams {
    /// Initial mint fee in FeelsSOL lamports
    pub mint_fee: u64,
    /// Treasury account to receive fees
    pub treasury: Pubkey,
}

/// Initialize protocol accounts
#[derive(Accounts)]
#[instruction(params: InitializeProtocolParams)]
pub struct InitializeProtocol<'info> {
    /// Protocol authority (deployer)
    #[account(
        mut,
        constraint = authority.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub authority: Signer<'info>,
    
    /// Protocol config account
    #[account(
        init,
        payer = authority,
        space = ProtocolConfig::LEN,
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Initialize protocol handler
pub fn initialize_protocol(
    ctx: Context<InitializeProtocol>,
    params: InitializeProtocolParams,
) -> Result<()> {
    let config = &mut ctx.accounts.protocol_config;
    
    // Set initial configuration
    config.authority = ctx.accounts.authority.key();
    config.mint_fee = params.mint_fee;
    config.treasury = params.treasury;
    config.token_expiration_seconds = 7 * 24 * 60 * 60; // 7 days default
    config._reserved = [0; 24];
    
    msg!("Protocol initialized with:");
    msg!("  Authority: {}", config.authority);
    msg!("  Mint fee: {} FeelsSOL", config.mint_fee);
    msg!("  Treasury: {}", config.treasury);
    
    Ok(())
}

/// Update protocol configuration parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UpdateProtocolParams {
    /// New mint fee (None to keep current)
    pub mint_fee: Option<u64>,
    /// New treasury (None to keep current)
    pub treasury: Option<Pubkey>,
    /// New authority (None to keep current)
    pub authority: Option<Pubkey>,
}

/// Update protocol accounts
#[derive(Accounts)]
pub struct UpdateProtocol<'info> {
    /// Current protocol authority
    #[account(
        mut,
        constraint = authority.key() == protocol_config.authority @ FeelsError::UnauthorizedSigner
    )]
    pub authority: Signer<'info>,
    
    /// Protocol config account
    #[account(
        mut,
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
}

/// Update protocol handler
pub fn update_protocol(
    ctx: Context<UpdateProtocol>,
    params: UpdateProtocolParams,
) -> Result<()> {
    let config = &mut ctx.accounts.protocol_config;
    
    // Update parameters if provided
    if let Some(mint_fee) = params.mint_fee {
        config.mint_fee = mint_fee;
        msg!("Updated mint fee to: {} FeelsSOL", mint_fee);
    }
    
    if let Some(treasury) = params.treasury {
        config.treasury = treasury;
        msg!("Updated treasury to: {}", treasury);
    }
    
    if let Some(authority) = params.authority {
        config.authority = authority;
        msg!("Updated authority to: {}", authority);
    }
    
    Ok(())
}