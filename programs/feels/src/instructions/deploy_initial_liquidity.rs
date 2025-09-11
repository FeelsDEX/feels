//! Deploy initial liquidity instruction
//! 
//! Deploys protocol buffer liquidity in an escalating stair pattern (80% of buffer).
//! Optionally allows the deployer to execute an initial buy at the best price
//! by including FeelsSOL with the instruction.

use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount},
};
use crate::{
    constants::{MARKET_AUTHORITY_SEED, VAULT_SEED, BUFFER_SEED, BUFFER_AUTHORITY_SEED},
    error::FeelsError,
    state::{Market, Buffer},
    utils::{transfer_from_user_to_vault_unchecked, sqrt_price_from_tick, liquidity_from_amounts},
};

/// Number of steps in the stair pattern
const STAIR_STEPS: usize = 10;

/// Percentage of buffer tokens to deploy (80%)
const DEPLOYMENT_PERCENTAGE: u8 = 80;

/// Deploy initial liquidity parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DeployInitialLiquidityParams {
    /// Number of ticks between each stair step
    pub tick_step_size: i32,
    /// Optional initial buy amount in FeelsSOL (0 = no initial buy)
    pub initial_buy_feelssol_amount: u64,
}

/// Deploy initial liquidity accounts
#[derive(Accounts)]
#[instruction(params: DeployInitialLiquidityParams)]
pub struct DeployInitialLiquidity<'info> {
    /// Deployer (must be market authority)
    #[account(
        mut,
        constraint = deployer.key() == market.authority @ FeelsError::UnauthorizedSigner,
    )]
    pub deployer: Signer<'info>,
    
    /// Market account
    #[account(
        mut,
        constraint = !market.initial_liquidity_deployed @ FeelsError::InvalidMarket,
    )]
    pub market: Account<'info, Market>,
    
    /// Deployer's FeelsSOL account (for initial buy)
    /// CHECK: Only validated if initial_buy_feelssol_amount > 0
    #[account(mut)]
    pub deployer_feelssol: AccountInfo<'info>,
    
    /// Deployer's token account for receiving initial buy tokens
    /// CHECK: Only validated if initial_buy_feelssol_amount > 0
    #[account(mut)]
    pub deployer_token_out: AccountInfo<'info>,
    
    /// Vault 0
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_0.as_ref()],
        bump = market.vault_0_bump,
    )]
    pub vault_0: Account<'info, TokenAccount>,
    
    /// Vault 1
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_1.as_ref()],
        bump = market.vault_1_bump,
    )]
    pub vault_1: Account<'info, TokenAccount>,
    
    /// Market authority PDA
    /// CHECK: PDA signer
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump = market.market_authority_bump,
    )]
    pub market_authority: AccountInfo<'info>,
    
    /// Buffer account (always required)
    #[account(
        mut,
        seeds = [BUFFER_SEED, market.key().as_ref()],
        bump,
    )]
    pub buffer: Account<'info, Buffer>,
    
    /// Buffer's token vault
    #[account(
        mut,
        constraint = buffer_token_vault.owner == buffer_authority.key() @ FeelsError::InvalidAuthority,
    )]
    pub buffer_token_vault: Account<'info, TokenAccount>,
    
    /// Buffer's FeelsSOL vault
    #[account(
        mut,
        constraint = buffer_feelssol_vault.owner == buffer_authority.key() @ FeelsError::InvalidAuthority,
    )]
    pub buffer_feelssol_vault: Account<'info, TokenAccount>,
    
    /// Buffer authority PDA
    /// CHECK: PDA that controls buffer vaults
    #[account(
        seeds = [BUFFER_AUTHORITY_SEED, buffer.key().as_ref()],
        bump = buffer.buffer_authority_bump,
    )]
    pub buffer_authority: AccountInfo<'info>,
    
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
    let _clock = Clock::get()?;
    
    // Deploy protocol liquidity in stair pattern
    deploy_protocol_stair_pattern(
        ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.vault_0,
        &ctx.accounts.vault_1,
        market,
        &ctx.accounts.buffer,
        &ctx.accounts.buffer_token_vault,
        &ctx.accounts.buffer_feelssol_vault,
        &ctx.accounts.buffer_authority,
        params.tick_step_size,
    )?;
    
    // Update buffer to track deployment
    let buffer = &mut ctx.accounts.buffer;
    
    // Calculate deployment amounts (80% of buffer balances)
    let deploy_token_amount = (ctx.accounts.buffer_token_vault.amount * DEPLOYMENT_PERCENTAGE as u64) / 100;
    let deploy_feelssol_amount = (ctx.accounts.buffer_feelssol_vault.amount * DEPLOYMENT_PERCENTAGE as u64) / 100;
    
    buffer.total_distributed = buffer.total_distributed
        .saturating_add(deploy_token_amount as u128)
        .saturating_add(deploy_feelssol_amount as u128);
    
    // Handle initial buy if requested
    if params.initial_buy_feelssol_amount > 0 {
        msg!("Processing initial buy of {} FeelsSOL", params.initial_buy_feelssol_amount);
        
        // Validate token accounts
        let deployer_feelssol = Account::<TokenAccount>::try_from(&ctx.accounts.deployer_feelssol)?;
        let deployer_token_out = Account::<TokenAccount>::try_from(&ctx.accounts.deployer_token_out)?;
        
        // Determine which token is FeelsSOL
        let feelssol_is_token_0 = market.token_0 == market.feelssol_mint;
        
        // Validate accounts based on which token is FeelsSOL
        if feelssol_is_token_0 {
            require!(
                deployer_feelssol.mint == market.token_0,
                FeelsError::InvalidMint
            );
            require!(
                deployer_token_out.mint == market.token_1,
                FeelsError::InvalidMint
            );
        } else {
            require!(
                deployer_feelssol.mint == market.token_1,
                FeelsError::InvalidMint
            );
            require!(
                deployer_token_out.mint == market.token_0,
                FeelsError::InvalidMint
            );
        }
        
        require!(
            deployer_feelssol.owner == ctx.accounts.deployer.key(),
            FeelsError::InvalidAuthority
        );
        require!(
            deployer_feelssol.amount >= params.initial_buy_feelssol_amount,
            FeelsError::InsufficientBalance
        );
        
        // Transfer FeelsSOL to appropriate vault
        let feelssol_vault = if feelssol_is_token_0 {
            &ctx.accounts.vault_0
        } else {
            &ctx.accounts.vault_1
        };
        
        transfer_from_user_to_vault_unchecked(
            &ctx.accounts.deployer_feelssol.to_account_info(),
            feelssol_vault.to_account_info(),
            &ctx.accounts.deployer,
            &ctx.accounts.token_program,
            params.initial_buy_feelssol_amount,
        )?;
        
        // Calculate output amount based on the current market price
        // The stair pattern starts at the current price, so the initial buy
        // gets tokens at the best available price
        use crate::utils::calculate_token_out_from_sqrt_price;
        
        // Get decimals from market (would be stored in production)
        let token_0_decimals = 9; // Standard for FeelsSOL
        let token_1_decimals = 6; // Standard for protocol tokens
        
        let output_amount = calculate_token_out_from_sqrt_price(
            params.initial_buy_feelssol_amount,
            market.sqrt_price,
            token_0_decimals,
            token_1_decimals,
            feelssol_is_token_0,
        )?;
        
        // Transfer output tokens from vault to deployer
        let token_out_vault = if feelssol_is_token_0 {
            &ctx.accounts.vault_1
        } else {
            &ctx.accounts.vault_0
        };
        
        let market_authority_seeds = &[
            MARKET_AUTHORITY_SEED,
            market.key().as_ref(),
            &[market.market_authority_bump],
        ];
        
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: token_out_vault.to_account_info(),
                    to: ctx.accounts.deployer_token_out.to_account_info(),
                    authority: ctx.accounts.market_authority.to_account_info(),
                },
                &[market_authority_seeds],
            ),
            output_amount,
        )?;
        
        msg!("Initial buy executed:");
        msg!("  FeelsSOL in: {}", params.initial_buy_feelssol_amount);
        msg!("  Tokens out: {}", output_amount);
        msg!("  At sqrt price: {}", market.sqrt_price);
        
        // Note: In a full implementation, this would update market state
        // (sqrt_price, liquidity, fees, etc.) based on the swap execution
    }
    
    Ok(())
}

/// Deploy protocol liquidity in stair pattern
fn deploy_protocol_stair_pattern<'info>(
    token_program: AccountInfo<'info>,
    vault_0: &Account<'info, TokenAccount>,
    vault_1: &Account<'info, TokenAccount>,
    market: &mut Account<'info, Market>,
    buffer: &Account<'info, Buffer>,
    buffer_token_vault: &Account<'info, TokenAccount>,
    buffer_feelssol_vault: &Account<'info, TokenAccount>,
    buffer_authority: &AccountInfo<'info>,
    tick_step_size: i32,
) -> Result<()> {
    // Validate parameters
    require!(
        tick_step_size > 0 && tick_step_size % market.tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    
    // Get current balances
    let buffer_token_balance = buffer_token_vault.amount;
    let buffer_feelssol_balance = buffer_feelssol_vault.amount;
    
    // Calculate 80% of each token to deploy
    let deploy_token_amount = (buffer_token_balance as u128 * DEPLOYMENT_PERCENTAGE as u128 / 100) as u64;
    let deploy_feelssol_amount = (buffer_feelssol_balance as u128 * DEPLOYMENT_PERCENTAGE as u128 / 100) as u64;
    
    msg!("Deploying protocol liquidity in stair pattern:");
    msg!("  Token amount: {} (80% of {})", deploy_token_amount, buffer_token_balance);
    msg!("  FeelsSOL amount: {} (80% of {})", deploy_feelssol_amount, buffer_feelssol_balance);
    
    // Determine if FeelsSOL is token_0 or token_1
    let feelssol_is_token_0 = market.token_0 == buffer.feelssol_mint;
    
    // Transfer tokens from buffer vaults to market vaults
    let buffer_key = buffer.key();
    let buffer_authority_seeds = &[
        BUFFER_AUTHORITY_SEED,
        buffer_key.as_ref(),
        &[buffer.buffer_authority_bump],
    ];
    
    // Transfer non-FeelsSOL token
    let (from_vault, to_vault, transfer_amount) = if feelssol_is_token_0 {
        (buffer_token_vault, vault_1, deploy_token_amount)
    } else {
        (buffer_token_vault, vault_0, deploy_token_amount)
    };
    
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            token_program.clone(),
            anchor_spl::token::Transfer {
                from: from_vault.to_account_info(),
                to: to_vault.to_account_info(),
                authority: buffer_authority.clone(),
            },
            &[buffer_authority_seeds],
        ),
        transfer_amount,
    )?;
    
    // Transfer FeelsSOL
    let (from_vault, to_vault, transfer_amount) = if feelssol_is_token_0 {
        (buffer_feelssol_vault, vault_0, deploy_feelssol_amount)
    } else {
        (buffer_feelssol_vault, vault_1, deploy_feelssol_amount)
    };
    
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            token_program.clone(),
            anchor_spl::token::Transfer {
                from: from_vault.to_account_info(),
                to: to_vault.to_account_info(),
                authority: buffer_authority.clone(),
            },
            &[buffer_authority_seeds],
        ),
        transfer_amount,
    )?;
    
    // Calculate stair pattern positions
    let current_tick = market.current_tick;
    let tick_spacing = market.tick_spacing as i32;
    
    // Create stair steps above current price
    let mut total_liquidity_added = 0u128;
    let mut remaining_token = deploy_token_amount;
    let mut remaining_feelssol = deploy_feelssol_amount;
    
    msg!("Creating {} stair steps starting from tick {}", STAIR_STEPS, current_tick);
    
    for step in 0..STAIR_STEPS {
        // Calculate tick range for this step
        let tick_lower = current_tick + (step as i32 * tick_step_size);
        let tick_upper = tick_lower + tick_step_size;
        
        // Ensure ticks are aligned to spacing
        let tick_lower = (tick_lower / tick_spacing) * tick_spacing;
        let tick_upper = (tick_upper / tick_spacing) * tick_spacing;
        
        // Calculate allocation for this step
        // Use a declining allocation: 20%, 18%, 16%, etc.
        let allocation_factor = if step == STAIR_STEPS - 1 {
            // Last step gets all remaining
            100
        } else {
            // Declining allocation
            20 - (step as u8 * 2).min(18)
        };
        
        let step_token_amount = (remaining_token as u128 * allocation_factor as u128 / 100) as u64;
        let step_feelssol_amount = (remaining_feelssol as u128 * allocation_factor as u128 / 100) as u64;
        
        // Calculate liquidity for this position
        let sqrt_price_lower = sqrt_price_from_tick(tick_lower)?;
        let sqrt_price_upper = sqrt_price_from_tick(tick_upper)?;
        
        // Use current price if it's within this range, otherwise use lower bound
        let sqrt_price_current = if market.sqrt_price >= sqrt_price_lower && market.sqrt_price < sqrt_price_upper {
            market.sqrt_price
        } else {
            sqrt_price_lower
        };
        
        let (amount_0, amount_1) = if feelssol_is_token_0 {
            (step_feelssol_amount, step_token_amount)
        } else {
            (step_token_amount, step_feelssol_amount)
        };
        
        let liquidity = liquidity_from_amounts(
            sqrt_price_current,
            sqrt_price_lower,
            sqrt_price_upper,
            amount_0,
            amount_1,
        )?;
        
        // TODO: In production, update tick arrays and create position NFTs
        // For now, just track the liquidity
        if market.current_tick >= tick_lower && market.current_tick < tick_upper {
            total_liquidity_added = total_liquidity_added.saturating_add(liquidity);
        }
        
        msg!("  Step {}: ticks [{}, {}], liquidity {}, amounts [{}, {}]",
            step, tick_lower, tick_upper, liquidity, amount_0, amount_1);
        
        // Update remaining amounts
        remaining_token = remaining_token.saturating_sub(step_token_amount);
        remaining_feelssol = remaining_feelssol.saturating_sub(step_feelssol_amount);
    }
    
    // Update market liquidity
    market.liquidity = market.liquidity.saturating_add(total_liquidity_added);
    market.initial_liquidity_deployed = true;
    
    // Note: Buffer updates would need to be done outside this function
    // since buffer is passed as immutable reference
    
    msg!("Protocol liquidity deployed successfully:");
    msg!("  Total active liquidity added: {}", total_liquidity_added);
    msg!("  Total market liquidity: {}", market.liquidity);
    
    Ok(())
}