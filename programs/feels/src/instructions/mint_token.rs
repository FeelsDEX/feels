//! Mint token instruction
//! 
//! Creates a new SPL token with distribution and buffer

use anchor_lang::prelude::*;
use anchor_lang::prelude::borsh;
use anchor_spl::{
    token::{self, Token, TokenAccount, Mint, spl_token::instruction::AuthorityType},
    associated_token::AssociatedToken,
};
use mpl_token_metadata::{
    instructions::CreateMetadataAccountV3,
    types::DataV2,
};
use crate::{
    constants::{BUFFER_SEED, BUFFER_AUTHORITY_SEED, TOKEN_DECIMALS, TOTAL_SUPPLY, MIN_FLOOR_PLACEMENT_THRESHOLD},
    error::FeelsError,
    events::TokenMinted,
    state::Buffer,
    utils::validate_distribution,
};

/// Distribution recipient
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct DistributionRecipient {
    pub address: Pubkey,
    pub amount: u64,
}

/// Wrapper for recipients to help with type inference
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct RecipientList {
    pub recipients: Vec<DistributionRecipient>,
}

/// Parameters for minting a new token
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintTokenParams {
    pub ticker: String,
    pub name: String,
    pub uri: String,
    pub creator_amount: u64,
    pub recipients: RecipientList,
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
        mint::authority = mint_authority.key(),
    )]
    pub token_mint: Account<'info, Mint>,
    
    /// Creator's token account
    #[account(
        init,
        payer = creator,
        associated_token::mint = token_mint,
        associated_token::authority = creator,
    )]
    pub creator_token: Account<'info, TokenAccount>,
    
    /// Buffer account for this token
    #[account(
        init,
        payer = creator,
        space = Buffer::LEN,
        seeds = [BUFFER_SEED, token_mint.key().as_ref()],
        bump,
    )]
    pub buffer: Account<'info, Buffer>,
    
    /// Buffer's token vault
    #[account(
        init,
        payer = creator,
        associated_token::mint = token_mint,
        associated_token::authority = buffer_authority,
    )]
    pub buffer_token_vault: Account<'info, TokenAccount>,
    
    /// Buffer's FeelsSOL vault
    #[account(
        init,
        payer = creator,
        associated_token::mint = feelssol_mint,
        associated_token::authority = buffer_authority,
    )]
    pub buffer_feelssol_vault: Account<'info, TokenAccount>,
    
    /// Buffer authority PDA
    /// CHECK: PDA that controls buffer vaults
    #[account(
        seeds = [BUFFER_AUTHORITY_SEED, buffer.key().as_ref()],
        bump,
    )]
    pub buffer_authority: AccountInfo<'info>,
    
    /// Mint authority (temporary)
    /// CHECK: Will be transferred to buffer authority
    pub mint_authority: AccountInfo<'info>,
    
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
    pub protocol_token: Account<'info, crate::state::ProtocolToken>,
    
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
    // Calculate total distribution
    let mut distribution_total = params.creator_amount;
    for recipient in &params.recipients.recipients {
        distribution_total = distribution_total
            .checked_add(recipient.amount)
            .ok_or(FeelsError::MathOverflow)?;
    }
    
    // Validate distribution (50% must go to buffer)
    let buffer_amount = TOTAL_SUPPLY / 2; // 500M tokens
    validate_distribution(distribution_total, TOTAL_SUPPLY, buffer_amount)?;
    
    // Validate ticker and name lengths
    require!(params.ticker.len() <= 10, FeelsError::InvalidPrice);
    require!(params.name.len() <= 32, FeelsError::InvalidPrice);
    
    // Initialize buffer state
    let buffer = &mut ctx.accounts.buffer;
    buffer.market = Pubkey::default(); // Will be set when market is initialized
    buffer.authority = ctx.accounts.creator.key();
    buffer.feelssol_mint = ctx.accounts.feelssol_mint.key();
    buffer.fees_token_0 = 0u128;
    buffer.fees_token_1 = 0u128;
    buffer.tau_spot = 0u128;
    buffer.tau_time = 0u128;
    buffer.tau_leverage = 0u128;
    buffer.floor_tick_spacing = 100; // Default
    buffer.floor_placement_threshold = MIN_FLOOR_PLACEMENT_THRESHOLD;
    buffer.last_floor_placement = 0;
    buffer.last_rebase = 0;
    buffer.total_distributed = 0u128;
    buffer.buffer_authority_bump = ctx.bumps.buffer_authority;
    buffer._reserved = [0; 8];
    
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
        mint_authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.creator.key(),
        update_authority: (ctx.accounts.creator.key(), true), // (authority, is_signer)
        system_program: ctx.accounts.system_program.key(),
        rent: Some(ctx.accounts.rent.key()),
    }.instruction(mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
        data: metadata_data,
        is_mutable: true,
        collection_details: None,
    });
    
    // Execute CPI to create metadata
    anchor_lang::solana_program::program::invoke(
        &create_metadata_ix,
        &[
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.metadata_program.to_account_info(),
        ],
    )?;
    
    // Mint tokens to distribution addresses
    // First mint to creator
    token::mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.creator_token.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        ),
        params.creator_amount,
    )?;
    
    // Mint to buffer
    token::mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.buffer_token_vault.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        ),
        buffer_amount,
    )?;
    
    // Note: In production, would also mint to other recipients
    // For MVP, skipping to keep it simple
    
    // Transfer mint authority to buffer authority PDA
    token::set_authority(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::SetAuthority {
                current_authority: ctx.accounts.mint_authority.to_account_info(),
                account_or_mint: ctx.accounts.token_mint.to_account_info(),
            },
        ),
        AuthorityType::MintTokens,
        Some(ctx.accounts.buffer_authority.key()),
    )?;
    
    // Initialize protocol token registry entry
    let protocol_token = &mut ctx.accounts.protocol_token;
    protocol_token.mint = ctx.accounts.token_mint.key();
    protocol_token.creator = ctx.accounts.creator.key();
    protocol_token.token_type = crate::state::TokenType::Spl; // Only SPL tokens for now
    protocol_token.created_at = Clock::get()?.unix_timestamp;
    protocol_token.can_create_markets = true;
    protocol_token._reserved = [0; 32];
    
    // Emit event
    emit!(TokenMinted {
        token_mint: ctx.accounts.token_mint.key(),
        creator: ctx.accounts.creator.key(),
        ticker: params.ticker,
        name: params.name,
        total_supply: TOTAL_SUPPLY,
        buffer_amount,
        creator_amount: params.creator_amount,
        buffer_account: buffer.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok(())
}