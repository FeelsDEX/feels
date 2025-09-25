//! Deploy initial liquidity instruction
//!
//! Deploys protocol escrow liquidity in an escalating stair pattern (100% of escrow).
//! Optionally allows the deployer to execute an initial buy at the best price
//! by including FeelsSOL with the instruction.

use crate::{
    constants::{ESCROW_AUTHORITY_SEED, MARKET_AUTHORITY_SEED, VAULT_SEED},
    error::FeelsError,
    state::{Market, PreLaunchEscrow, TrancheEntry, TranchePlan},
    utils::{liquidity_from_amounts, sqrt_price_from_tick, transfer_from_user_to_vault_unchecked},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccountState;

/// Number of steps in the stair pattern
const STAIR_STEPS: usize = 10;

/// Percentage of escrow tokens to deploy (100%)
const DEPLOYMENT_PERCENTAGE: u8 = 100;

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
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub deployer: Signer<'info>,

    /// Market account
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,

    /// Token mints to read decimals (production-grade)
    pub token_0_mint: Account<'info, anchor_spl::token::Mint>,
    pub token_1_mint: Account<'info, anchor_spl::token::Mint>,

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
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"0"],
        bump = market.vault_0_bump,
    )]
    pub vault_0: Account<'info, TokenAccount>,

    /// Vault 1
    #[account(
        mut,
        seeds = [VAULT_SEED, market.token_0.as_ref(), market.token_1.as_ref(), b"1"],
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

    /// Market buffer account (for fee collection, not token escrow)
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub buffer: Box<Account<'info, crate::state::Buffer>>,

    /// Oracle account for price updates
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub oracle: Box<Account<'info, crate::state::OracleState>>,

    /// Pre-launch escrow for the protocol token
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub escrow: Box<Account<'info, PreLaunchEscrow>>,

    /// Escrow's token vault
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub escrow_token_vault: Account<'info, TokenAccount>,

    /// Escrow's FeelsSOL vault
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub escrow_feelssol_vault: Account<'info, TokenAccount>,

    /// Escrow authority PDA
    /// CHECK: PDA that controls escrow vaults
    #[account(
        seeds = [ESCROW_AUTHORITY_SEED, escrow.key().as_ref()],
        bump = escrow.escrow_authority_bump,
    )]
    pub escrow_authority: AccountInfo<'info>,

    /// Protocol config account
    #[account(
        seeds = [crate::state::ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Box<Account<'info, crate::state::ProtocolConfig>>,

    /// Treasury to receive mint fee
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,

    // Remaining accounts: expected TickArray PDAs for any tick ranges touched.
    // The handler will initialize any uninitialized arrays it needs from these.
    /// Tranche plan PDA (initialized here)
    #[account(
        init,
        payer = deployer,
        space = TranchePlan::space_for(STAIR_STEPS),
        seeds = [b"tranche_plan".as_ref(), market.key().as_ref()],
        bump
    )]
    pub tranche_plan: Account<'info, TranchePlan>,
}

/// Deploy initial liquidity handler
pub fn deploy_initial_liquidity<'info>(
    ctx: Context<'_, '_, 'info, 'info, DeployInitialLiquidity<'info>>,
    params: DeployInitialLiquidityParams,
) -> Result<()> {
    // Early validation - fail fast before any state changes

    // Validate constraints (moved from struct to save stack space)
    require!(
        ctx.accounts.deployer.key() == ctx.accounts.market.authority,
        FeelsError::UnauthorizedSigner
    );
    require!(
        ctx.accounts.buffer.market == ctx.accounts.market.key(),
        FeelsError::InvalidBuffer
    );
    require!(
        ctx.accounts.oracle.key() == ctx.accounts.market.oracle,
        FeelsError::InvalidOracle
    );
    require!(
        ctx.accounts.escrow_token_vault.owner == ctx.accounts.escrow_authority.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        ctx.accounts.escrow_feelssol_vault.owner == ctx.accounts.escrow_authority.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        ctx.accounts.treasury.key() == ctx.accounts.protocol_config.treasury,
        FeelsError::InvalidAuthority
    );
    require!(
        ctx.accounts.treasury.mint == ctx.accounts.buffer.feelssol_mint,
        FeelsError::InvalidMint
    );

    // 1. Validate tick parameters first
    require!(params.tick_step_size > 0, FeelsError::TickNotSpaced);
    require!(
        params.tick_step_size % ctx.accounts.market.tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );

    // Validate initial buy amount if specified
    if params.initial_buy_feelssol_amount > 0 {
        crate::utils::validate_swap_amount(params.initial_buy_feelssol_amount, false)?;
    }

    // 2. Validate deployment hasn't already happened
    require!(
        !ctx.accounts.market.initial_liquidity_deployed,
        FeelsError::InvalidMarket
    );

    // 3. Validate escrow is linked to this market
    require!(
        ctx.accounts.escrow.market == ctx.accounts.market.key(),
        FeelsError::InvalidBuffer
    );

    // 4. Validate sufficient tokens in escrow for deployment
    let deploy_token_amount =
        (ctx.accounts.escrow_token_vault.amount * DEPLOYMENT_PERCENTAGE as u64) / 100;
    let deploy_feelssol_amount =
        (ctx.accounts.escrow_feelssol_vault.amount * DEPLOYMENT_PERCENTAGE as u64) / 100;

    require!(deploy_token_amount > 0, FeelsError::InsufficientBalance);
    require!(deploy_feelssol_amount > 0, FeelsError::InsufficientBalance);

    // 5. Early validation: If initial buy requested, validate deployer has sufficient balance
    if params.initial_buy_feelssol_amount > 0 {
        let deployer_feelssol_data = &ctx.accounts.deployer_feelssol.try_borrow_data()?;
        let deployer_feelssol = TokenAccountState::unpack(deployer_feelssol_data)?;

        require!(
            deployer_feelssol.owner == ctx.accounts.deployer.key(),
            FeelsError::InvalidAuthority
        );
        require!(
            deployer_feelssol.amount >= params.initial_buy_feelssol_amount,
            FeelsError::InsufficientBalance
        );

        msg!(
            "Initial buy validation passed: {} FeelsSOL available",
            deployer_feelssol.amount
        );
    }

    // Get initial balances (deployment amounts already calculated in validation)
    let initial_feelssol_balance = ctx.accounts.escrow_feelssol_vault.amount;

    // Deploy protocol liquidity in stair pattern
    let market_key = ctx.accounts.market.key();
    let mut tranche_entries: Vec<TrancheEntry> = Vec::with_capacity(STAIR_STEPS);
    deploy_protocol_stair_pattern(
        ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.vault_0,
        &ctx.accounts.vault_1,
        ctx.accounts.market.as_mut(),
        ctx.accounts.escrow.as_ref(),
        ctx.accounts.escrow.key(),
        &ctx.accounts.escrow_token_vault,
        &ctx.accounts.escrow_feelssol_vault,
        &ctx.accounts.escrow_authority,
        params.tick_step_size,
        &ctx.accounts.deployer,
        &ctx.accounts.system_program,
        ctx.remaining_accounts,
        market_key,
        &mut tranche_entries,
    )?;

    // Update market buffer to track deployment
    let buffer = &mut ctx.accounts.buffer;

    buffer.total_distributed = buffer
        .total_distributed
        .saturating_add(deploy_token_amount as u128)
        .saturating_add(deploy_feelssol_amount as u128);

    // Handle initial buy if requested
    if params.initial_buy_feelssol_amount > 0 {
        msg!(
            "Processing initial buy of {} FeelsSOL",
            params.initial_buy_feelssol_amount
        );

        // Validate token accounts
        let deployer_feelssol_data = &ctx.accounts.deployer_feelssol.try_borrow_data()?;
        let deployer_feelssol = TokenAccountState::unpack(deployer_feelssol_data)?;

        let deployer_token_out_data = &ctx.accounts.deployer_token_out.try_borrow_data()?;
        let deployer_token_out = TokenAccountState::unpack(deployer_token_out_data)?;

        // Determine which token is FeelsSOL
        let feelssol_is_token_0 = ctx.accounts.market.token_0 == ctx.accounts.escrow.feelssol_mint;

        // Validate accounts based on which token is FeelsSOL
        if feelssol_is_token_0 {
            require!(
                deployer_feelssol.mint == ctx.accounts.market.token_0,
                FeelsError::InvalidMint
            );
            require!(
                deployer_token_out.mint == ctx.accounts.market.token_1,
                FeelsError::InvalidMint
            );
        } else {
            require!(
                deployer_feelssol.mint == ctx.accounts.market.token_1,
                FeelsError::InvalidMint
            );
            require!(
                deployer_token_out.mint == ctx.accounts.market.token_0,
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
            &feelssol_vault.to_account_info(),
            &ctx.accounts.deployer,
            &ctx.accounts.token_program,
            params.initial_buy_feelssol_amount,
        )?;

        // Calculate output amount based on the current market price
        // The stair pattern starts at the current price, so the initial buy
        // gets tokens at the best available price
        use crate::utils::calculate_token_out_from_sqrt_price;

        // Read decimals from mint accounts
        let token_0_decimals = ctx.accounts.token_0_mint.decimals;
        let token_1_decimals = ctx.accounts.token_1_mint.decimals;

        let output_amount = calculate_token_out_from_sqrt_price(
            params.initial_buy_feelssol_amount,
            ctx.accounts.market.sqrt_price,
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

        let market_key = ctx.accounts.market.key();
        let market_authority_seeds = &[
            MARKET_AUTHORITY_SEED,
            market_key.as_ref(),
            &[ctx.accounts.market.market_authority_bump],
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
        msg!("  At sqrt price: {}", ctx.accounts.market.sqrt_price);

        // Update market state to reflect the initial buy
        // This ensures the market remains in a consistent state

        // 1. Update cumulative volume
        if feelssol_is_token_0 {
            ctx.accounts.market.total_volume_token_0 = ctx
                .accounts
                .market
                .total_volume_token_0
                .saturating_add(params.initial_buy_feelssol_amount);
        } else {
            ctx.accounts.market.total_volume_token_1 = ctx
                .accounts
                .market
                .total_volume_token_1
                .saturating_add(params.initial_buy_feelssol_amount);
        }

        // 2. Calculate and apply base fee to buffer
        let base_fee_bps = ctx.accounts.market.base_fee_bps;
        let base_fee_amount = if base_fee_bps > 0 {
            (params.initial_buy_feelssol_amount as u128 * base_fee_bps as u128 / 10_000) as u64
        } else {
            0
        };

        if base_fee_amount > 0 {
            // Add fees to buffer accounting
            if feelssol_is_token_0 {
                ctx.accounts.buffer.fees_token_0 = ctx
                    .accounts
                    .buffer
                    .fees_token_0
                    .saturating_add(base_fee_amount as u128);
            } else {
                ctx.accounts.buffer.fees_token_1 = ctx
                    .accounts
                    .buffer
                    .fees_token_1
                    .saturating_add(base_fee_amount as u128);
            }

            msg!(
                "  Base fee collected: {} ({}bps)",
                base_fee_amount,
                base_fee_bps
            );
        }

        // 3. Update oracle with current price data
        let clock = Clock::get()?;
        ctx.accounts
            .oracle
            .update(ctx.accounts.market.current_tick, clock.unix_timestamp)?;

        // 4. Apply any price impact (simplified for initial buy)
        // Since this is the first trade and liquidity is just deployed,
        // price impact is minimal, but we should still account for it
        let price_impact_bps = 1; // Minimal impact for initial buy

        // 5. Emit swap event for the initial buy
        emit!(crate::events::SwapExecuted {
            market: ctx.accounts.market.key(),
            user: ctx.accounts.deployer.key(),
            token_in: if feelssol_is_token_0 {
                ctx.accounts.market.token_0
            } else {
                ctx.accounts.market.token_1
            },
            token_out: if feelssol_is_token_0 {
                ctx.accounts.market.token_1
            } else {
                ctx.accounts.market.token_0
            },
            amount_in: params.initial_buy_feelssol_amount,
            amount_out: output_amount,
            fee_paid: base_fee_amount,
            base_fee_paid: base_fee_amount,
            impact_bps: price_impact_bps,
            sqrt_price_after: ctx.accounts.market.sqrt_price,
            timestamp: clock.unix_timestamp,
            version: 2,
        });

        msg!("Market state updated after initial buy");
    }

    // Update market to reflect deployment
    let market = &mut ctx.accounts.market;
    market.initial_liquidity_deployed = true;

    // Transfer mint fee from escrow to treasury now that market is live
    // With 100% deployment, there is no remaining mint fee
    let mint_fee_amount = initial_feelssol_balance - deploy_feelssol_amount;

    if mint_fee_amount > 0 {
        msg!(
            "Transferring mint fee of {} FeelsSOL to treasury",
            mint_fee_amount
        );

        let escrow_key = ctx.accounts.escrow.key();
        let escrow_authority_seeds = &[
            ESCROW_AUTHORITY_SEED,
            escrow_key.as_ref(),
            &[ctx.accounts.escrow.escrow_authority_bump],
        ];

        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.escrow_feelssol_vault.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                    authority: ctx.accounts.escrow_authority.to_account_info(),
                },
                &[escrow_authority_seeds],
            ),
            mint_fee_amount,
        )?;

        msg!("Mint fee transferred to treasury successfully");
    }

    // Initialize TranchePlan PDA with computed ranges + liquidity for crank usage
    // Account creation handled by Anchor's init constraint
    let tranche_plan = &mut ctx.accounts.tranche_plan;
    tranche_plan.market = ctx.accounts.market.key();
    tranche_plan.applied = false;
    tranche_plan.count = STAIR_STEPS as u8;
    tranche_plan.entries = tranche_entries;

    Ok(())
}

/// Deploy protocol liquidity in stair pattern
#[allow(clippy::too_many_arguments)]
fn deploy_protocol_stair_pattern<'info>(
    token_program: AccountInfo<'info>,
    vault_0: &Account<'info, TokenAccount>,
    vault_1: &Account<'info, TokenAccount>,
    market: &mut Market,
    escrow: &PreLaunchEscrow,
    escrow_key: Pubkey,
    escrow_token_vault: &Account<'info, TokenAccount>,
    escrow_feelssol_vault: &Account<'info, TokenAccount>,
    escrow_authority: &AccountInfo<'info>,
    tick_step_size: i32,
    _payer: &Signer<'info>,
    _system_program: &Program<'info, System>,
    _remaining_accounts: &'info [AccountInfo<'info>],
    _market_key: Pubkey,
    entries_out: &mut Vec<TrancheEntry>,
) -> Result<()> {
    // Parameters already validated in main handler

    // Get current balances
    let escrow_token_balance = escrow_token_vault.amount;
    let escrow_feelssol_balance = escrow_feelssol_vault.amount;

    // Calculate 100% of each token to deploy
    let deploy_token_amount =
        (escrow_token_balance as u128 * DEPLOYMENT_PERCENTAGE as u128 / 100) as u64;
    let deploy_feelssol_amount =
        (escrow_feelssol_balance as u128 * DEPLOYMENT_PERCENTAGE as u128 / 100) as u64;

    msg!("Deploying protocol liquidity in stair pattern:");
    msg!(
        "  Token amount: {} (100% of {})",
        deploy_token_amount,
        escrow_token_balance
    );
    msg!(
        "  FeelsSOL amount: {} (100% of {})",
        deploy_feelssol_amount,
        escrow_feelssol_balance
    );

    // Determine if FeelsSOL is token_0 or token_1
    let feelssol_is_token_0 = market.token_0 == escrow.feelssol_mint;

    // Transfer tokens from escrow vaults to market vaults
    let escrow_authority_seeds = &[
        ESCROW_AUTHORITY_SEED,
        escrow_key.as_ref(),
        &[escrow.escrow_authority_bump],
    ];

    // Transfer non-FeelsSOL token
    let (from_vault, to_vault, transfer_amount) = if feelssol_is_token_0 {
        (escrow_token_vault, vault_1, deploy_token_amount)
    } else {
        (escrow_token_vault, vault_0, deploy_token_amount)
    };

    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            token_program.clone(),
            anchor_spl::token::Transfer {
                from: from_vault.to_account_info(),
                to: to_vault.to_account_info(),
                authority: escrow_authority.clone(),
            },
            &[escrow_authority_seeds],
        ),
        transfer_amount,
    )?;

    // Transfer FeelsSOL
    let (from_vault, to_vault, transfer_amount) = if feelssol_is_token_0 {
        (escrow_feelssol_vault, vault_0, deploy_feelssol_amount)
    } else {
        (escrow_feelssol_vault, vault_1, deploy_feelssol_amount)
    };

    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            token_program.clone(),
            anchor_spl::token::Transfer {
                from: from_vault.to_account_info(),
                to: to_vault.to_account_info(),
                authority: escrow_authority.clone(),
            },
            &[escrow_authority_seeds],
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

    msg!(
        "Creating {} stair steps starting from tick {}",
        STAIR_STEPS,
        current_tick
    );

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
        let step_feelssol_amount =
            (remaining_feelssol as u128 * allocation_factor as u128 / 100) as u64;

        // Calculate liquidity for this position
        let sqrt_price_lower = sqrt_price_from_tick(tick_lower)?;
        let sqrt_price_upper = sqrt_price_from_tick(tick_upper)?;

        // Use current price if it's within this range, otherwise use lower bound
        let sqrt_price_current =
            if market.sqrt_price >= sqrt_price_lower && market.sqrt_price < sqrt_price_upper {
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
        // Record tranche entry for crank usage
        entries_out.push(TrancheEntry {
            tick_lower,
            tick_upper,
            liquidity,
        });

        // Track active liquidity contribution for the tranche that includes current price
        if market.current_tick >= tick_lower && market.current_tick < tick_upper {
            total_liquidity_added = total_liquidity_added.saturating_add(liquidity);
        }

        msg!(
            "  Step {}: ticks [{}, {}], liquidity {}, amounts [{}, {}]",
            step,
            tick_lower,
            tick_upper,
            liquidity,
            amount_0,
            amount_1
        );

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

// Bonding-curve tranche tick array initialization can be handled by a follow-up crank.
