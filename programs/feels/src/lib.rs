use anchor_lang::prelude::*;
use anchor_lang::accounts::interface_account::InterfaceAccount;
use anchor_spl::token_interface::Mint;
use anchor_spl::token_2022::Token2022;

pub mod instructions;
pub mod state;

declare_id!("Fee1sProtoco11111111111111111111111111111111");

#[derive(Accounts)]
pub struct InitializeFeels<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(underlying_mint: Pubkey)]
pub struct InitializeFeelsSOL<'info> {
    /// FeelsSOL wrapper account
    #[account(
        init,
        payer = authority,
        space = state::FeelsSOL::SIZE,
        seeds = [b"feelssol"],
        bump
    )]
    pub feelssol: Account<'info, state::FeelsSOL>,
    
    /// FeelsSOL Token-2022 mint
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = feelssol,
        mint::freeze_authority = feelssol,
    )]
    pub feels_mint: InterfaceAccount<'info, Mint>,
    
    /// Protocol authority
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[program]
pub mod feels {
    use super::*;
    
    pub fn initialize_feels(ctx: Context<InitializeFeels>) -> Result<()> {
        instructions::initialize_protocol::handler(ctx)
    }
    
    // TODO: Re-enable after fixing Bumps trait issue
    // pub fn initialize_feelssol(ctx: Context<InitializeFeelsSOL>, underlying_mint: Pubkey) -> Result<()> {
    //     instructions::initialize_feelssol::handler(ctx, underlying_mint)
    // }
}