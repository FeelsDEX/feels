use anchor_lang::{prelude::*, solana_program::sysvar::instructions::get_instruction_relative};
use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{mint_to, set_authority, MintTo, SetAuthority, Token};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount},
};

use crate::{error::TokenFactoryError, events::TokenCreated, state::TokenFactory};

const MAX_DECIMALS: u8 = 18;

#[derive(Accounts)]
#[instruction(decimals: u8, initial_supply: u64)]
pub struct CreateToken<'info> {
    /// Token factory (becomes mint authority)
    #[account(
        mut,
        seeds = [b"factory"],
        bump,
    )]
    pub factory: Account<'info, TokenFactory>,

    /// New token mint - FACTORY becomes mint authority
    #[account(
        init,
        signer,
        payer = payer,
        mint::decimals = decimals,
        mint::authority = factory,
        mint::freeze_authority = factory,
        mint::token_program = token_program,
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// Recipient token account for initial mint
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program,
    )]
    pub recipient_token: InterfaceAccount<'info, TokenAccount>,

    /// Token recipient
    /// CHECK: Can be any account
    pub recipient: UncheckedAccount<'info>,

    /// Payer for accounts
    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// Instructions sysvar to check the calling program
    /// CHECK: This is the instructions sysvar
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn create_token(ctx: Context<CreateToken>, decimals: u8, initial_supply: u64) -> Result<()> {
    // Verify this is called from the feels protocol
    let ix = get_instruction_relative(0, &ctx.accounts.instructions)?;
    require!(
        ix.program_id == ctx.accounts.factory.feels_protocol,
        TokenFactoryError::UnauthorizedProtocol
    );

    // Validate token decimals
    require!(
        decimals <= MAX_DECIMALS,
        TokenFactoryError::DecimalsTooLarge
    );

    // Get the signer seeds for the factory PDA
    let factory_seeds: &[&[u8]] = &[b"factory".as_ref(), &[ctx.bumps.factory]];
    let signer_seeds = &[factory_seeds];

    // Mint initial supply to recipient if requested
    if initial_supply > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.recipient_token.to_account_info(),
            authority: ctx.accounts.factory.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(cpi_ctx, initial_supply)?;
    }

    // Put mint authority to None to avoid further minting
    let set_authority_accounts = SetAuthority {
        account_or_mint: ctx.accounts.token_mint.to_account_info(),
        current_authority: ctx.accounts.factory.to_account_info(),
    };

    let set_authority_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        set_authority_accounts,
        signer_seeds,
    );

    set_authority(set_authority_ctx, AuthorityType::MintTokens, None)?;

    // Increase the number of tokens created
    ctx.accounts.factory.tokens_created += 1;

    emit!(TokenCreated {
        mint: ctx.accounts.token_mint.key(),
        decimals,
        initial_supply,
    });

    Ok(())
}
