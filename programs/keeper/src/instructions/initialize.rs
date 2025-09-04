use anchor_lang::prelude::*;

use crate::{error::KeeperError, events::FeelsKeepersInitialized, state::Keeper};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Keeper::INIT_SPACE,
        seeds = [b"keeper"],
        bump
    )]
    pub keeper: Account<'info, Keeper>,

    /// CHECK: Keeper authority (performs updates)
    pub authority: UncheckedAccount<'info>,

    /// Account that pays (operational wallet)
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_feels_keeper(
    ctx: Context<Initialize>,
    feelssol_to_lst_rate_numerator: u64,
    feelssol_to_lst_rate_denominator: u64,
) -> Result<()> {
    let keeper = &mut ctx.accounts.keeper;
    keeper.authority = ctx.accounts.authority.key();

    // Validate rates are not zero
    require!(
        feelssol_to_lst_rate_numerator != 0 && feelssol_to_lst_rate_denominator != 0,
        KeeperError::ZeroRate
    );

    keeper.feelssol_to_lst_rate_numerator = feelssol_to_lst_rate_numerator;
    keeper.feelssol_to_lst_rate_denominator = feelssol_to_lst_rate_denominator;

    emit!(FeelsKeepersInitialized {
        feelssol_to_lst_rate_numerator,
        feelssol_to_lst_rate_denominator,
    });

    Ok(())
}
