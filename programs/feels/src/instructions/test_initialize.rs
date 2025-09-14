//! Test instruction to debug initialize_market
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TestInitializeParams {
    pub dummy: u64,
}

#[derive(Accounts)]
pub struct TestInitialize<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(mut)]
    pub token_0: Account<'info, Mint>,
    
    #[account(mut)]  
    pub token_1: Account<'info, Mint>,
    
    pub system_program: Program<'info, System>,
}

pub fn test_initialize(
    ctx: Context<TestInitialize>,
    params: TestInitializeParams,
) -> Result<()> {
    msg!("test_initialize: Handler called!");
    msg!("  creator: {}", ctx.accounts.creator.key());
    msg!("  token_0: {}", ctx.accounts.token_0.key());
    msg!("  token_1: {}", ctx.accounts.token_1.key());
    msg!("  params.dummy: {}", params.dummy);
    
    Ok(())
}