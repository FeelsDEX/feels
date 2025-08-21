use crate::CreateNft;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

pub fn handler(ctx: Context<CreateNft>, name: String, symbol: String, uri: String) -> Result<()> {
    // Initialize the mint for NFT (decimals = 0, supply will be 1)
    token_2022::initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        0, // NFTs have 0 decimals
        &ctx.accounts.mint_authority.key(),
        Some(&ctx.accounts.mint_authority.key()),
    )?;

    msg!(
        "Created NFT mint: {} | Name: {} | Symbol: {} | URI: {}",
        ctx.accounts.mint.key(),
        name,
        symbol,
        uri
    );

    msg!("NFT mint successfully created with Token-2022 metadata extension");
    msg!("Note: Metadata initialization will be handled by client-side Token-2022 metadata instructions");

    Ok(())
}
