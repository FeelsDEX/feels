use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_2022::Token2022};

pub mod instructions;
pub mod state;

pub use state::*;

declare_id!("Fee1sProtoco11111111111111111111111111111111");

#[program]
pub mod feels {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Create a new Token-2022 mint with metadata support
    pub fn create(
        ctx: Context<Create>,
        name: String,
        symbol: String,
        uri: String,
        decimals: u8,
    ) -> Result<()> {
        instructions::create::handler(ctx, name, symbol, uri, decimals)
    }

    /// Mint tokens to a specific account
    pub fn mint(ctx: Context<Mint>, amount: u64) -> Result<()> {
        instructions::mint::handler(ctx, amount)
    }

    /// Burn tokens from an account
    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        instructions::burn::handler(ctx, amount)
    }

    /// Update token metadata (logs the request for now)
    pub fn update(
        ctx: Context<Update>,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
    ) -> Result<()> {
        instructions::update::handler(ctx, name, symbol, uri)
    }

    /// Create a new NFT with Token-2022 metadata extension
    pub fn create_nft(
        ctx: Context<CreateNft>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::create_nft::handler(ctx, name, symbol, uri)
    }

    /// Mint an NFT (exactly 1 token)
    pub fn mint_nft(ctx: Context<MintNft>) -> Result<()> {
        instructions::mint_nft::handler(ctx)
    }

    /// Update NFT metadata field
    pub fn update_nft(ctx: Context<UpdateNft>, field: String, value: String) -> Result<()> {
        instructions::update_nft::handler(ctx, field, value)
    }

    /// Burn an NFT (exactly 1 token)
    pub fn burn_nft(ctx: Context<BurnNft>) -> Result<()> {
        instructions::burn_nft::handler(ctx)
    }
}

// Account structs must be at crate root for Anchor macros
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String, decimals: u8)]
pub struct Create<'info> {
    #[account(
        init,
        payer = payer,
        space = 82, // Token-2022 mint account size
    )]
    /// CHECK: This account will be initialized as a Token-2022 mint
    pub mint: AccountInfo<'info>,

    /// CHECK: This will be set as the mint authority
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Mint<'info> {
    #[account(mut)]
    /// CHECK: Token-2022 mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Token-2022
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    /// CHECK: Token recipient
    pub recipient: AccountInfo<'info>,

    pub mint_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(mut)]
    /// CHECK: Token-2022 mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Token-2022
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    pub update_authority: Signer<'info>,

    /// CHECK: Token-2022 mint account for metadata updates
    pub mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token2022>,
}

// NFT Account Structs
#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String)]
pub struct CreateNft<'info> {
    #[account(
        init,
        payer = payer,
        space = 82, // Standard Token-2022 mint account size
    )]
    /// CHECK: This account will be initialized as a Token-2022 NFT mint with metadata
    pub mint: AccountInfo<'info>,

    /// CHECK: This will be set as the mint authority and metadata update authority
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    /// CHECK: Token-2022 NFT mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Token-2022 NFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    /// CHECK: NFT recipient
    pub recipient: AccountInfo<'info>,

    pub mint_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateNft<'info> {
    /// CHECK: Token-2022 NFT mint account with metadata
    pub mint: AccountInfo<'info>,

    pub update_authority: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct BurnNft<'info> {
    #[account(mut)]
    /// CHECK: Token-2022 NFT mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Token-2022 NFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}
