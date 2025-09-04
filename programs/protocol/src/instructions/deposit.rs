use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

// Import the feelsSOL controller for CPI
use feelssol_controller::cpi::{
    accounts::Deposit as FeelsSOLDeposit, deposit as feelssol_controller_deposit,
};

use crate::{error::ProtocolError, state::protocol::ProtocolState};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"protocol"],
        bump,
        has_one = feelssol_controller @ ProtocolError::InvalidFeelsSOLController,
    )]
    pub protocol: Account<'info, ProtocolState>,

    /// CHECK: This is the FeelsSOL pda used by the controller
    #[account(mut)]
    pub feelssol: UncheckedAccount<'info>,

    /// CHECK: This is the FeelsSOL controller program - validated by has_one constraint above
    pub feelssol_controller: UncheckedAccount<'info>,

    /// CHECK: FeelsSOL mint account, validated by the FeelsSOL program
    #[account(mut)]
    pub feels_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: User's LST token account, validated by the FeelsSOL program
    #[account(mut)]
    pub user_lst: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: User's FeelsSOL token account, validated by the FeelsSOL program
    #[account(mut)]
    pub user_feelssol: UncheckedAccount<'info>,

    /// CHECK: LST vault account, validated by the FeelsSOL program
    #[account(mut)]
    pub lst_vault: UncheckedAccount<'info>,

    /// CHECK: Underlying mint account, validated by the FeelsSOL program
    pub underlying_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Stake pool account, validated by the FeelsSOL program
    pub keeper: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program (for LST transfers)
    pub token_program: Program<'info, Token>,

    /// Token2022 program (for FeelsSOL minting)
    pub token_2022_program: Program<'info, Token2022>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// Instructions sysvar - required by FeelsSOL for caller verification
    /// CHECK: This is the instructions sysvar
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn deposit_via_feelssol_controller(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // Check if protocol is paused
    if ctx.accounts.protocol.paused {
        return Err(ProtocolError::ProtocolPaused.into());
    }

    // Set up the CPI context
    let cpi_program = ctx.accounts.feelssol_controller.to_account_info();
    let cpi_accounts = FeelsSOLDeposit {
        feelssol: ctx.accounts.feelssol.to_account_info(),
        feels_mint: ctx.accounts.feels_mint.to_account_info(),
        user_lst: ctx.accounts.user_lst.to_account_info(),
        user_feelssol: ctx.accounts.user_feelssol.to_account_info(),
        lst_vault: ctx.accounts.lst_vault.to_account_info(),
        underlying_mint: ctx.accounts.underlying_mint.to_account_info(),
        keeper: ctx.accounts.keeper.to_account_info(),
        user: ctx.accounts.user.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        token_2022_program: ctx.accounts.token_2022_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
        instructions: ctx.accounts.instructions.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    // Make the CPI call to the feelssol controller
    feelssol_controller_deposit(cpi_ctx, amount)?;

    Ok(())
}
