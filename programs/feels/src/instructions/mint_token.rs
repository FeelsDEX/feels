//! Mint token instruction
//!
//! Creates a new SPL token with distribution and buffer

use crate::{
    constants::{ESCROW_AUTHORITY_SEED, ESCROW_SEED, TOKEN_DECIMALS, TOTAL_SUPPLY},
    error::FeelsError,
    events::TokenMinted,
    state::{PreLaunchEscrow, ProtocolConfig},
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use mpl_token_metadata::{instructions::CreateMetadataAccountV3, types::DataV2};

/// Parameters for minting a new token
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintTokenParams {
    pub ticker: String,
    pub name: String,
    pub uri: String,
}

#[derive(Accounts)]
#[instruction(params: MintTokenParams)]
pub struct MintToken<'info> {
    /// Token creator
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = creator.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub creator: Signer<'info>,

    /// New token mint to create
    #[account(
        init,
        payer = creator,
        mint::decimals = TOKEN_DECIMALS,
        mint::authority = creator.key(), // Initially set to creator, will be revoked
        mint::freeze_authority = creator.key(), // Initially set to creator, will be revoked
    )]
    pub token_mint: Account<'info, Mint>,

    /// Pre-launch escrow account for this token
    #[account(
        init,
        payer = creator,
        space = PreLaunchEscrow::LEN,
        seeds = [ESCROW_SEED, token_mint.key().as_ref()],
        bump,
    )]
    pub escrow: Box<Account<'info, PreLaunchEscrow>>,

    /// Escrow's token vault (holds all minted tokens)
    #[account(
        init,
        payer = creator,
        associated_token::mint = token_mint,
        associated_token::authority = escrow_authority,
    )]
    pub escrow_token_vault: Box<Account<'info, TokenAccount>>,

    /// Escrow's FeelsSOL vault (holds mint fee)
    #[account(
        init,
        payer = creator,
        associated_token::mint = feelssol_mint,
        associated_token::authority = escrow_authority,
    )]
    pub escrow_feelssol_vault: Box<Account<'info, TokenAccount>>,

    /// Escrow authority PDA
    /// CHECK: PDA that controls escrow vaults
    #[account(
        seeds = [ESCROW_AUTHORITY_SEED, escrow.key().as_ref()],
        bump,
    )]
    pub escrow_authority: AccountInfo<'info>,

    /// Metadata account
    /// CHECK: Created by Metaplex CPI
    #[account(
        mut,
        seeds = [b"metadata", mpl_token_metadata::ID.as_ref(), token_mint.key().as_ref()],
        seeds::program = mpl_token_metadata::ID,
        bump,
    )]
    pub metadata: AccountInfo<'info>,

    /// FeelsSOL mint
    pub feelssol_mint: Account<'info, Mint>,

    /// Creator's FeelsSOL account for paying mint fee
    #[account(
        mut,
        constraint = creator_feelssol.owner == creator.key() @ FeelsError::InvalidAuthority,
        constraint = creator_feelssol.mint == feelssol_mint.key() @ FeelsError::InvalidMint,
    )]
    pub creator_feelssol: Box<Account<'info, TokenAccount>>,

    /// Protocol config account
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Box<Account<'info, ProtocolConfig>>,

    /// Metaplex token metadata program
    /// CHECK: The metadata program ID is validated by address constraint
    #[account(address = mpl_token_metadata::ID)]
    pub metadata_program: AccountInfo<'info>,

    /// Protocol token registry entry
    #[account(
        init,
        payer = creator,
        space = crate::state::ProtocolToken::LEN,
        seeds = [crate::constants::PROTOCOL_TOKEN_SEED, token_mint.key().as_ref()],
        bump,
    )]
    pub protocol_token: Box<Account<'info, crate::state::ProtocolToken>>,

    /// Associated token program
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

pub fn mint_token(ctx: Context<MintToken>, params: MintTokenParams) -> Result<()> {
    // Early validation - fail fast before any state changes

    // 1. Validate parameters first
    require!(params.ticker.len() <= 10, FeelsError::InvalidPrice);
    require!(params.name.len() <= 32, FeelsError::InvalidPrice);
    require!(params.uri.len() <= 200, FeelsError::InvalidPrice); // Reasonable URI length limit

    // 2. Validate mint fee and balance
    let mint_fee = ctx.accounts.protocol_config.mint_fee;
    require!(
        ctx.accounts.creator_feelssol.amount >= mint_fee,
        FeelsError::InsufficientBalance
    );

    // 3. Validate account ownership
    require!(
        ctx.accounts.creator_feelssol.owner == ctx.accounts.creator.key(),
        FeelsError::InvalidAuthority
    );

    // Now proceed with state changes
    if mint_fee > 0 {
        // Transfer mint fee from creator to escrow (held until market goes live)
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.creator_feelssol.to_account_info(),
                    to: ctx.accounts.escrow_feelssol_vault.to_account_info(),
                    authority: ctx.accounts.creator.to_account_info(),
                },
            ),
            mint_fee,
        )?;

        msg!("Collected mint fee: {} FeelsSOL (held in escrow)", mint_fee);
    }

    // All tokens go to escrow (100% of supply)
    let escrow_amount = TOTAL_SUPPLY;

    // Initialize pre-launch escrow state
    let escrow = &mut ctx.accounts.escrow;
    let clock = Clock::get()?;
    escrow.token_mint = ctx.accounts.token_mint.key();
    escrow.creator = ctx.accounts.creator.key();
    escrow.feelssol_mint = ctx.accounts.feelssol_mint.key();
    escrow.created_at = clock.unix_timestamp;
    escrow.market = Pubkey::default(); // Will be set when market is initialized
    escrow.escrow_authority_bump = ctx.bumps.escrow_authority;
    escrow._reserved = [0; 128];

    // Create token metadata
    let metadata_data = DataV2 {
        name: params.name.clone(),
        symbol: params.ticker.clone(),
        uri: params.uri.clone(),
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    // Create metadata account using Metaplex
    let create_metadata_ix = CreateMetadataAccountV3 {
        metadata: ctx.accounts.metadata.key(),
        mint: ctx.accounts.token_mint.key(),
        mint_authority: ctx.accounts.creator.key(), // Creator is the mint authority initially
        payer: ctx.accounts.creator.key(),
        update_authority: (ctx.accounts.creator.key(), true), // (authority, is_signer)
        system_program: ctx.accounts.system_program.key(),
        rent: Some(ctx.accounts.rent.key()),
    }
    .instruction(
        mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
            data: metadata_data,
            is_mutable: true,
            collection_details: None,
        },
    );

    // Execute CPI to create metadata
    anchor_lang::solana_program::program::invoke_signed(
        &create_metadata_ix,
        &[
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.creator.to_account_info(), // Creator is mint authority
            ctx.accounts.creator.to_account_info(), // Also payer
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.metadata_program.to_account_info(),
        ],
        &[], // No seeds needed, creator is already a signer
    )?;

    // Mint all tokens to escrow
    token::mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.escrow_token_vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            },
        ),
        escrow_amount,
    )?;

    // NOTE: Mint and freeze authorities are NOT revoked here
    // They will be revoked in initialize_market after creator verification
    // This allows the token creator to be verified as the owner before authorities are removed

    // Initialize protocol token registry entry
    let protocol_token = &mut ctx.accounts.protocol_token;
    protocol_token.mint = ctx.accounts.token_mint.key();
    protocol_token.creator = ctx.accounts.creator.key();
    protocol_token.token_type = crate::state::TokenType::Spl; // Only SPL tokens for now
    protocol_token.created_at = clock.unix_timestamp;
    protocol_token.can_create_markets = true;
    protocol_token._reserved = [0; 32];

    // Emit event
    emit!(TokenMinted {
        token_mint: ctx.accounts.token_mint.key(),
        creator: ctx.accounts.creator.key(),
        ticker: params.ticker,
        name: params.name,
        total_supply: TOTAL_SUPPLY,
        buffer_amount: escrow_amount,
        creator_amount: 0,
        buffer_account: escrow.key(),
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
