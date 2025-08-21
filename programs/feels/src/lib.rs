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

    // FeelsSOL Token Operations (Protocol Synthetic Token)
    pub fn feelssol_create(ctx: Context<FeelsSOLCreate>) -> Result<()> {
        instructions::feelssol_create::handler(ctx)
    }

    pub fn feelssol_mint(ctx: Context<FeelsSOLMint>, amount: u64) -> Result<()> {
        instructions::feelssol_mint::handler(ctx, amount)
    }

    pub fn feelssol_burn(ctx: Context<FeelsSOLBurn>, amount: u64) -> Result<()> {
        instructions::feelssol_burn::handler(ctx, amount)
    }

    // Feels Token Operations (User-Created Tokens)
    pub fn feels_token_create(
        ctx: Context<FeelsTokenCreate>,
        name: String,
        symbol: String,
        uri: String,
        decimals: u8,
    ) -> Result<()> {
        instructions::feels_token_create::handler(ctx, name, symbol, uri, decimals)
    }

    pub fn feels_token_mint(ctx: Context<FeelsTokenMint>, amount: u64) -> Result<()> {
        instructions::feels_token_mint::handler(ctx, amount)
    }

    pub fn feels_token_burn(ctx: Context<FeelsTokenBurn>, amount: u64) -> Result<()> {
        instructions::feels_token_burn::handler(ctx, amount)
    }

    // Pool Position NFT Operations (Liquidity Positions)
    pub fn pool_position_create(
        ctx: Context<PoolPositionCreate>,
        position_id: String,
        pool_id: String,
    ) -> Result<()> {
        instructions::pool_position_create::handler(ctx, position_id, pool_id)
    }

    pub fn pool_position_mint(ctx: Context<PoolPositionMint>) -> Result<()> {
        instructions::pool_position_mint::handler(ctx)
    }

    pub fn pool_position_burn(ctx: Context<PoolPositionBurn>) -> Result<()> {
        instructions::pool_position_burn::handler(ctx)
    }
}

// Account Structs (must be at crate root for Anchor macros)
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// FeelsSOL Token Account Structs
#[derive(Accounts)]
pub struct FeelsSOLCreate<'info> {
    #[account(
        init,
        payer = payer,
        space = 82, // Token-2022 mint account size
    )]
    /// CHECK: This account will be initialized as the FeelsSOL mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Protocol authority that controls FeelsSOL minting
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct FeelsSOLMint<'info> {
    #[account(mut)]
    /// CHECK: FeelsSOL mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for FeelsSOL
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    /// CHECK: FeelsSOL recipient
    pub recipient: AccountInfo<'info>,

    pub mint_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FeelsSOLBurn<'info> {
    #[account(mut)]
    /// CHECK: FeelsSOL mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for FeelsSOL
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}

// Feels Token Account Structs
#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String, decimals: u8)]
pub struct FeelsTokenCreate<'info> {
    #[account(
        init,
        payer = payer,
        space = 82, // Token-2022 mint account size
    )]
    /// CHECK: This account will be initialized as a user's Feels token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: User authority that controls their Feels token
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct FeelsTokenMint<'info> {
    #[account(mut)]
    /// CHECK: User's Feels token mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for user's Feels token
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    /// CHECK: Feels token recipient
    pub recipient: AccountInfo<'info>,

    pub mint_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FeelsTokenBurn<'info> {
    #[account(mut)]
    /// CHECK: User's Feels token mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for user's Feels token
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}

// Pool Position NFT Account Structs
#[derive(Accounts)]
#[instruction(position_id: String, pool_id: String)]
pub struct PoolPositionCreate<'info> {
    #[account(
        init,
        payer = payer,
        space = 82, // Token-2022 mint account size
    )]
    /// CHECK: This account will be initialized as a Pool Position NFT mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Protocol authority that controls Pool Position NFT creation
    pub mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PoolPositionMint<'info> {
    #[account(mut)]
    /// CHECK: Pool Position NFT mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Pool Position NFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    /// CHECK: Pool Position NFT recipient
    pub recipient: AccountInfo<'info>,

    pub mint_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PoolPositionBurn<'info> {
    #[account(mut)]
    /// CHECK: Pool Position NFT mint account
    pub mint: AccountInfo<'info>,

    /// CHECK: Associated token account for Pool Position NFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}
