use anchor_lang::system_program::{transfer, Transfer};
use anchor_lang::{prelude::*, solana_program::sysvar::instructions::get_instruction_relative};
use anchor_spl::token_2022::spl_token_2022::extension::{
    BaseStateWithExtensions, PodStateWithExtensions,
};
use anchor_spl::token_2022::spl_token_2022::pod::PodMint;
use anchor_spl::token_interface::spl_token_metadata_interface::state::TokenMetadata;
use anchor_spl::token_interface::TokenMetadataInitialize;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        mint_to, set_authority, spl_token_2022::instruction::AuthorityType, MintTo, SetAuthority,
        Token2022,
    },
    token_interface::{token_metadata_initialize, Mint, TokenAccount},
};

use crate::{
    error::TokenFactoryError, events::TokenCreated, state::TokenFactory,
    token_validate::validate_token,
};

#[derive(Accounts)]
#[instruction(symbol: String, name: String, uri: String, decimals: u8, initial_supply: u64)]
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
        extensions::metadata_pointer::metadata_address = token_mint,
        extensions::metadata_pointer::authority = factory,
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
    symbol: String,
    name: String,
    uri: String,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    // Verify this is called from the feels protocol
    let ix = get_instruction_relative(0, &ctx.accounts.instructions)?;
    require!(
        ix.program_id == ctx.accounts.factory.feels_protocol,
        TokenFactoryError::UnauthorizedProtocol
    );

    // Validate token against restrictions and format requirements
    validate_token(&symbol, &name, decimals)?;

    // Fund the account with enough lamports to store the metadata
    let (required_lamports, current_lamports) = {
        let mint_info = ctx.accounts.token_mint.to_account_info();
        let mint_data = mint_info.try_borrow_data()?;
        let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

        let metadata = TokenMetadata {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            ..Default::default()
        };

        let new_len = mint_state.try_get_new_account_len_for_variable_len_extension(&metadata)?;
        let required_lamports = Rent::get()?.minimum_balance(new_len);
        let current_lamports = mint_info.lamports();

        (required_lamports, current_lamports)
    };

    if required_lamports > current_lamports {
        let lamport_diff = required_lamports - current_lamports;
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.token_mint.to_account_info(),
                },
            ),
            lamport_diff,
        )?;
        msg!(
            "Transferred {} lamports to mint account for metadata",
            lamport_diff
        );
    }

    // Initialize the metadata of the token
    let factory_seeds: &[&[u8]] = &[b"factory".as_ref(), &[ctx.bumps.factory]];
    let signer_seeds = &[factory_seeds];
    token_metadata_initialize(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataInitialize {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.token_mint.to_account_info(),
                update_authority: ctx.accounts.factory.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                mint_authority: ctx.accounts.factory.to_account_info(),
            },
            signer_seeds,
        ),
        name.clone(),
        symbol.clone(),
        uri,
    )?;

    // Mint initial supply to recipient if requested
    if initial_supply > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.factory.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(cpi_ctx, initial_supply)?;

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
    }

    // Increase the number of tokens created
    ctx.accounts.factory.tokens_created += 1;

    emit!(TokenCreated {
        mint: ctx.accounts.token_mint.key(),
        name,
        symbol,
        decimals,
        initial_supply,
    });

    Ok(())
}
