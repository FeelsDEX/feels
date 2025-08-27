use anchor_lang::prelude::*;
use anchor_spl::{token_2022::Token2022, token_interface::Mint};

use crate::state::FeelsSOLWrapper;

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// FeelsSOL wrapper account
    #[account(
        init,
        payer = payer,
        space = FeelsSOLWrapper::SIZE,
        seeds = [b"feelssol"],
        bump
    )]
    pub feelssol: Account<'info, FeelsSOLWrapper>,

    /// FeelsSOL Token-2022 mint
    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = feelssol,
        mint::freeze_authority = feelssol,
        mint::token_program = token_program,
        seeds = [b"feels_mint"],  // Add seeds for deterministic derivation
        bump
    )]
    pub feels_mint: InterfaceAccount<'info, Mint>,

    /// Account that pays (operational wallet)
    #[account(mut)]
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_feelssol(
    ctx: Context<Initialize>,
    underlying_mint: Pubkey,
    feels_protocol: Pubkey,
) -> Result<()> {
    let feelssol = &mut ctx.accounts.feelssol;
    feelssol.underlying_mint = underlying_mint;
    feelssol.total_wrapped = 0;
    feelssol.virtual_reserves = 0;
    feelssol.yield_accumulator = 0;
    feelssol.last_update_slot = 0;
    feelssol.feels_protocol = feels_protocol;
    Ok(())
}
