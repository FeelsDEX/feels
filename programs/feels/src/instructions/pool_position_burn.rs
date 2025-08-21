use crate::PoolPositionBurn;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Burn a Position NFT (when closing/exiting a liquidity position)
pub fn handler(ctx: Context<PoolPositionBurn>) -> Result<()> {
    let cpi_accounts = token_2022::Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token_2022::burn(cpi_ctx, 1)?; // Position NFTs always burn exactly 1 token

    msg!(
        "Burned Position NFT from {}",
        ctx.accounts.token_account.key()
    );

    Ok(())
}
