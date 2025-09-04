use crate::{error::FeelsSolError, events::WithdrawEvent, state::FeelsSolController};
use anchor_lang::{prelude::*, solana_program::sysvar::instructions::get_instruction_relative};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Token},
    token_2022::{self, Burn, Token2022},
    token_interface::{Mint, TokenAccount},
};
use feels_keeper::state::Keeper;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// FeelsSOL controller account
    #[account(
        mut,
        seeds = [b"feelssol"],
        bump
    )]
    pub feelssol: Account<'info, FeelsSolController>,

    #[account(
        mut,
        address = feelssol.feels_mint
    )]
    pub feels_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = feelssol.underlying_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_lst: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = feels_mint,
        associated_token::authority = user,
        associated_token::token_program = token_2022_program
    )]
    pub user_feelssol: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault"],
        bump,
        token::mint = underlying_mint,
        token::authority = feelssol,
    )]
    pub lst_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        address = feelssol.underlying_mint
    )]
    pub underlying_mint: InterfaceAccount<'info, Mint>,

    #[account(address = feelssol.keeper)]
    pub keeper: Account<'info, Keeper>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// SPL Token program (for LST transfers like JitoSOL)
    pub token_program: Program<'info, Token>,

    /// Token2022 program (for FeelsSOL burning)
    pub token_2022_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// Instructions sysvar to check the calling program
    /// CHECK: This is the instructions sysvar
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    // Verify this is called from the feels protocol
    let ix = get_instruction_relative(0, &ctx.accounts.instructions)?;
    require!(
        ix.program_id == ctx.accounts.feelssol.feels_protocol,
        FeelsSolError::UnauthorizedProtocol
    );

    if amount == 0 {
        return Err(FeelsSolError::InvalidAmount.into());
    }

    // Ensure user has enough FeelsSOL tokens to burn
    require!(
        ctx.accounts.user_feelssol.amount >= amount,
        FeelsSolError::InsufficientBalance
    );

    // Calculate the LST amount to return
    let output_amount = (amount as u128)
        .checked_mul(ctx.accounts.keeper.feelssol_to_lst_rate_numerator as u128)
        .ok_or(FeelsSolError::MathOverflow)?
        .checked_div(ctx.accounts.keeper.feelssol_to_lst_rate_denominator as u128)
        .ok_or(FeelsSolError::MathOverflow)?
        .try_into()
        .map_err(|_| FeelsSolError::MathOverflow)?;

    // Burn FeelsSOL tokens from user
    let burn_ctx = CpiContext::new(
        ctx.accounts.token_2022_program.to_account_info(),
        Burn {
            mint: ctx.accounts.feels_mint.to_account_info(),
            from: ctx.accounts.user_feelssol.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token_2022::burn(burn_ctx, amount)?;

    // Transfer LST tokens from vault to user
    let seeds = &[b"feelssol".as_ref(), &[ctx.bumps.feelssol]];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.lst_vault.to_account_info(),
            to: ctx.accounts.user_lst.to_account_info(),
            authority: ctx.accounts.feelssol.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, output_amount)?;

    // Update the amount of LST wrapped
    let new_total = ctx
        .accounts
        .feelssol
        .total_wrapped
        .saturating_sub(output_amount);

    ctx.accounts.feelssol.total_wrapped = new_total;

    emit!(WithdrawEvent {
        user: ctx.accounts.user.key(),
        feelssol_burned: amount,
        lst_withdrawn: output_amount,
        current_lst_amount_wrapped: ctx.accounts.feelssol.total_wrapped,
    });

    Ok(())
}
