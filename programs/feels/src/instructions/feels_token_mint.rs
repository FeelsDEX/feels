use crate::FeelsTokenMint;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Mint user-created Feels tokens
pub fn handler(ctx: Context<FeelsTokenMint>, amount: u64) -> Result<()> {
    // Create associated token account if it doesn't exist
    if ctx.accounts.token_account.data_is_empty() {
        let create_ata_accounts = anchor_spl::associated_token::Create {
            payer: ctx.accounts.payer.to_account_info(),
            associated_token: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.recipient.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };

        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            create_ata_accounts,
        ))?;
    }

    // Mint user's Feels tokens
    let cpi_accounts = token_2022::MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token_2022::mint_to(cpi_ctx, amount)?;

    msg!(
        "Minted {} Feels tokens to {}",
        amount,
        ctx.accounts.token_account.key()
    );

    Ok(())
}
