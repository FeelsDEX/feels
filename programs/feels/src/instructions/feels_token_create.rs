use crate::FeelsTokenCreate;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Create a new user-defined Feels token
pub fn handler(
    ctx: Context<FeelsTokenCreate>,
    name: String,
    symbol: String,
    _uri: String,
    decimals: u8,
) -> Result<()> {
    // Initialize the user's Feels token mint
    token_2022::initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        decimals,
        &ctx.accounts.mint_authority.key(),
        Some(&ctx.accounts.mint_authority.key()),
    )?;

    msg!(
        "Created Feels token mint: {} | Name: {} | Symbol: {} | Decimals: {}",
        ctx.accounts.mint.key(),
        name,
        symbol,
        decimals
    );

    msg!("User-created Feels token ready for trading in protocol pools");

    Ok(())
}
