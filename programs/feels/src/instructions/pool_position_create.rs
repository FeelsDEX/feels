use crate::PoolPositionCreate;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self as token_2022};

/// Create a Position NFT mint (represents liquidity positions in pools)
pub fn handler(
    ctx: Context<PoolPositionCreate>,
    position_id: String,
    pool_id: String,
) -> Result<()> {
    // Initialize Position NFT mint with 0 decimals (NFT standard)
    token_2022::initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        0, // Position NFTs have 0 decimals
        &ctx.accounts.mint_authority.key(),
        Some(&ctx.accounts.mint_authority.key()),
    )?;

    msg!(
        "Created Position NFT mint: {} | Position ID: {} | Pool ID: {}",
        ctx.accounts.mint.key(),
        position_id,
        pool_id
    );

    msg!("Position NFT ready to represent liquidity position in protocol");

    Ok(())
}
