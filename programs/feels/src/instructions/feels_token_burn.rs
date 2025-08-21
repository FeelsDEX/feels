use crate::FeelsTokenBurn;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Burn user-created Feels tokens
pub fn handler(ctx: Context<FeelsTokenBurn>, amount: u64) -> Result<()> {
    let cpi_accounts = token_2022::Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token_2022::burn(cpi_ctx, amount)?;

    msg!(
        "Burned {} Feels tokens from {}",
        amount,
        ctx.accounts.token_account.key()
    );

    Ok(())
}
