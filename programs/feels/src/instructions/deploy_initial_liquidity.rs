//! Deploy initial liquidity instruction
//! 
//! Deploys the committed initial liquidity to a market. This must match
//! the commitment made during market initialization exactly.

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::{
    constants::{MARKET_AUTHORITY_SEED, VAULT_SEED},
    error::FeelsError,
    state::{Market, InitialLiquidityCommitment},
    utils::transfer_from_user_to_vault_unchecked,
};

/// Deploy initial liquidity parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DeployInitialLiquidityParams {
    /// The commitment that was stored during market initialization
    pub commitment: InitialLiquidityCommitment,
}

/// Deploy initial liquidity accounts
#[derive(Accounts)]
#[instruction(params: DeployInitialLiquidityParams)]
pub struct DeployInitialLiquidity<'info> {
    /// Deployer (must match commitment.deployer)
    #[account(
        mut,
        constraint = deployer.key() == params.commitment.deployer @ FeelsError::InvalidAuthority
    )]
    pub deployer: Signer<'info>,
    
    /// Market account
    #[account(
        mut,
        constraint = !market.initial_liquidity_deployed @ FeelsError::InvalidMarket,
    )]
    pub market: Account<'info, Market>,
    
    /// Deployer's token 0 account
    #[account(
        mut,
        constraint = deployer_token_0.owner == deployer.key() @ FeelsError::InvalidAuthority,
        constraint = deployer_token_0.mint == market.token_0 @ FeelsError::InvalidMint,
        constraint = deployer_token_0.amount >= params.commitment.token_0_amount @ FeelsError::InsufficientBalance,
    )]
    pub deployer_token_0: Account<'info, TokenAccount>,
    
    /// Deployer's token 1 account
    #[account(
        mut,
        constraint = deployer_token_1.owner == deployer.key() @ FeelsError::InvalidAuthority,
        constraint = deployer_token_1.mint == market.token_1 @ FeelsError::InvalidMint,
        constraint = deployer_token_1.amount >= params.commitment.token_1_amount @ FeelsError::InsufficientBalance,
    )]
    pub deployer_token_1: Account<'info, TokenAccount>,
    
    /// Vault 0
    /// CHECK: Validated as PDA
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_0.as_ref()],
        bump = market.vault_0_bump,
    )]
    pub vault_0: UncheckedAccount<'info>,
    
    /// Vault 1
    /// CHECK: Validated as PDA
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_1.as_ref()],
        bump = market.vault_1_bump,
    )]
    pub vault_1: UncheckedAccount<'info>,
    
    /// Market authority PDA
    /// CHECK: PDA signer
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump = market.market_authority_bump,
    )]
    pub market_authority: AccountInfo<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Deploy initial liquidity handler
pub fn deploy_initial_liquidity<'info>(
    ctx: Context<'_, '_, 'info, 'info, DeployInitialLiquidity<'info>>,
    params: DeployInitialLiquidityParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let commitment = &params.commitment;
    let clock = Clock::get()?;
    
    // Verify deployment deadline
    require!(
        clock.unix_timestamp <= commitment.deploy_by,
        FeelsError::InvalidTimestamp
    );
    
    // Transfer committed token amounts to vaults
    transfer_from_user_to_vault_unchecked(
        &ctx.accounts.deployer_token_0.to_account_info(),
        &ctx.accounts.vault_0.to_account_info(),
        &ctx.accounts.deployer,
        &ctx.accounts.token_program,
        commitment.token_0_amount,
    )?;
    
    transfer_from_user_to_vault_unchecked(
        &ctx.accounts.deployer_token_1.to_account_info(),
        &ctx.accounts.vault_1.to_account_info(),
        &ctx.accounts.deployer,
        &ctx.accounts.token_program,
        commitment.token_1_amount,
    )?;
    
    // Process each position commitment
    // Note: In a real implementation, this would create position NFTs and update tick arrays
    // For now, we'll track total liquidity
    let mut total_liquidity = 0u128;
    for position_commitment in &commitment.position_commitments {
        // Validate position is within global bounds
        require!(
            position_commitment.tick_lower >= market.global_lower_tick,
            FeelsError::InvalidTickRange
        );
        require!(
            position_commitment.tick_upper <= market.global_upper_tick,
            FeelsError::InvalidTickRange
        );
        
        // If position is in range, add to active liquidity
        if market.current_tick >= position_commitment.tick_lower &&
           market.current_tick < position_commitment.tick_upper {
            total_liquidity = total_liquidity
                .checked_add(position_commitment.liquidity)
                .ok_or(FeelsError::MathOverflow)?;
        }
        
        // TODO: Create actual position NFTs and update tick arrays
        // This would require the position mints and tick arrays to be passed in remaining_accounts
    }
    
    // Update market state
    market.liquidity = total_liquidity;
    market.initial_liquidity_deployed = true;
    
    msg!("Initial liquidity deployed successfully");
    msg!("Total active liquidity: {}", total_liquidity);
    msg!("Token 0 deployed: {}", commitment.token_0_amount);
    msg!("Token 1 deployed: {}", commitment.token_1_amount);
    
    Ok(())
}