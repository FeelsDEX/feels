use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_2022::Token2022};

// Import your token factory for CPI
use feels_token_factory::cpi::{
    accounts::CreateToken as TokenFactoryCreateToken, create_token as token_factory_create_token,
};

use crate::{error::ProtocolError, state::protocol::ProtocolState};

#[derive(Accounts)]
#[instruction(symbol: String, name: String, uri: String, decimals: u8, initial_supply: u64)]
pub struct CreateToken<'info> {
    #[account(
        mut,
        seeds = [b"protocol"],
        bump,
        has_one = authority @ ProtocolError::InvalidAuthority,
        has_one = token_factory @ ProtocolError::InvalidTokenFactory
    )]
    pub protocol: Account<'info, ProtocolState>,

    /// CHECK: Will be the PDA used by token factory program
    #[account(mut)]
    pub factory: UncheckedAccount<'info>,

    /// New token mint that will be created
    /// CHECK: Will be initialized by token factory program
    #[account(mut, signer)]
    pub token_mint: UncheckedAccount<'info>,

    /// Recipient token account for initial mint
    /// CHECK: Will be created/validated by token factory program  
    #[account(mut)]
    pub recipient_token: UncheckedAccount<'info>,

    /// Token recipient
    /// CHECK: Can be any account
    pub recipient: UncheckedAccount<'info>,

    /// Payer and authority for accounts
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Token factory program
    /// CHECK: This is checked via constraint
    pub token_factory: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// Instructions sysvar
    /// CHECK: This is the instructions sysvar
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn create_token_via_factory(
    ctx: Context<CreateToken>,
    symbol: String,
    name: String,
    uri: String,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    // TODO: Add feels protocol specific logic here
    // For example: validation, fees, etc.

    // Check if protocol is paused
    if ctx.accounts.protocol.paused {
        return Err(ProtocolError::ProtocolPaused.into());
    }

    // Set up the CPI context
    let cpi_program = ctx.accounts.token_factory.to_account_info();
    let cpi_accounts = TokenFactoryCreateToken {
        factory: ctx.accounts.factory.to_account_info(),
        token_mint: ctx.accounts.token_mint.to_account_info(),
        recipient_token: ctx.accounts.recipient_token.to_account_info(),
        recipient: ctx.accounts.recipient.to_account_info(),
        payer: ctx.accounts.authority.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
        instructions: ctx.accounts.instructions.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    // Make the CPI call to the token factory
    token_factory_create_token(cpi_ctx, symbol, name, uri, decimals, initial_supply)?;

    Ok(())
}
