use crate::FeelsSOLCreate;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Create the FeelsSOL token mint (protocol's synthetic token)
pub fn handler(ctx: Context<FeelsSOLCreate>) -> Result<()> {
    // Initialize FeelsSOL mint with 9 decimals (standard SPL token decimals)
    token_2022::initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        9, // 9 decimals for FeelsSOL
        &ctx.accounts.mint_authority.key(),
        Some(&ctx.accounts.mint_authority.key()),
    )?;

    msg!("Created FeelsSOL token mint: {}", ctx.accounts.mint.key());

    msg!("FeelsSOL is the protocol's synthetic token backed by JitoSOL");

    Ok(())
}
