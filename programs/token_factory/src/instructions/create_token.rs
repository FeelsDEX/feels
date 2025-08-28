use anchor_lang::{prelude::*, solana_program::sysvar::instructions::get_instruction_relative};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{mint_to, MintTo, Token2022},
    token_interface::{Mint, TokenAccount},
};

use crate::{
    error::TokenFactoryError,
    events::TokenCreated,
    state::{factory::TokenFactory, metadata::TokenMetadata},
    token_validate::validate_token,
};

#[derive(Accounts)]
#[instruction(ticker: String, name: String, symbol: String, decimals: u8, initial_supply: u64)]
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
        payer = payer,
        mint::decimals = decimals,
        mint::authority = factory,
        mint::freeze_authority = factory,
        mint::token_program = token_program,
    )]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// Token metadata
    #[account(
        init,
        payer = payer,
        space = 8 + TokenMetadata::INIT_SPACE,
        seeds = [
            b"metadata",
            token_mint.key().as_ref()
        ],
        bump
    )]
    pub token_metadata: Account<'info, TokenMetadata>,

    /// Recipient token account for initial mint
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program,
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Token recipient
    /// CHECK: Can be any account
    pub recipient: UncheckedAccount<'info>,

    /// Payer for accounts
    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// Instructions sysvar to check the calling program
    /// CHECK: This is the instructions sysvar
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn create_token(
    ctx: Context<CreateToken>,
    ticker: String,
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    // Verify this is called from the feels protocol
    let ix = get_instruction_relative(0, &ctx.accounts.instructions)?;
    require!(
        ix.program_id == ctx.accounts.factory.feels_protocol,
        TokenFactoryError::UnauthorizedProtocol
    );

    // Validate ticker against restrictions and format requirements
    validate_token(&ticker, &name, &symbol, decimals)?;

    // Initialize token metadata
    let token_metadata = &mut ctx.accounts.token_metadata;
    token_metadata.ticker = ticker.clone();
    token_metadata.name = name.clone();
    token_metadata.symbol = symbol.clone();
    token_metadata.mint = ctx.accounts.token_mint.key();
    token_metadata.authority = ctx.accounts.factory.key();
    token_metadata.created_at = Clock::get()?.unix_timestamp;

    // Mint initial supply to recipient if requested
    if initial_supply > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.factory.to_account_info(),
        };

        let factory_seeds: &[&[u8]] = &[b"factory".as_ref(), &[ctx.bumps.factory]];
        let signer_seeds = &[factory_seeds];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(cpi_ctx, initial_supply)?;
    }

    // Increase the number of tokens created
    ctx.accounts.factory.tokens_created += 1;

    emit!(TokenCreated {
        mint: ctx.accounts.token_mint.key(),
        ticker: ctx.accounts.token_metadata.ticker.clone(),
        name: ctx.accounts.token_metadata.name.clone(),
        symbol: ctx.accounts.token_metadata.symbol.clone(),
        decimals: ctx.accounts.token_mint.decimals,
        initial_supply,
    });

    Ok(())
}
