use anchor_lang::prelude::*;

use crate::{events::TokenFactoryInitialized, state::TokenFactory};

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Token factory account
    #[account(
        init,
        payer = payer,
        space = 8 + TokenFactory::INIT_SPACE,
        seeds = [b"factory"],
        bump
    )]
    pub token_factory: Account<'info, TokenFactory>,

    /// Account that pays (operational wallet)
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_token_factory(ctx: Context<Initialize>, feels_protocol: Pubkey) -> Result<()> {
    let token_factory = &mut ctx.accounts.token_factory;
    token_factory.tokens_created = 0;
    token_factory.feels_protocol = feels_protocol;

    emit!(TokenFactoryInitialized { feels_protocol });

    Ok(())
}
