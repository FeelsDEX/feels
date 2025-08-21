use crate::Create;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

pub fn handler(
    ctx: Context<Create>,
    name: String,
    symbol: String,
    uri: String,
    decimals: u8,
) -> Result<()> {
    // Initialize the mint using Token-2022
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

    // Log the token creation with metadata
    msg!(
        "Created Token-2022 mint: {} | Name: {} | Symbol: {} | URI: {} | Decimals: {}",
        ctx.accounts.mint.key(),
        name,
        symbol,
        uri,
        decimals
    );

    msg!("Token-2022 mint successfully created with metadata parameters");

    Ok(())
}
