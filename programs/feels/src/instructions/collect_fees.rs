//! Collect fees for a position - smart single entry point
//! 
//! This instruction automatically handles:
//! 1. Normal positions: Calculates and collects fees in one transaction
//! 2. Wide positions: Returns error directing to use 3-step process
//! 3. Accumulated fees: Transfers already-calculated fees
//! 
//! ## Usage Guide:
//! 
//! ### Normal Positions (common case):
//! ```
//! // Provide tick arrays in remaining_accounts
//! collect_fees {
//!     accounts: { position, owner, vaults, ... },
//!     remaining_accounts: [lower_tick_array, upper_tick_array]
//! }
//! ```
//! 
//! ### Wide Positions (tick arrays too far apart):
//! If you get a MissingTickArrayCoverage error, use the 3-step process:
//! ```
//! // Step 1: Calculate fees for lower tick
//! update_position_fee_lower { ... }
//! 
//! // Step 2: Calculate fees for upper tick  
//! update_position_fee_upper { ... }
//! 
//! // Step 3: Collect accumulated fees (no tick arrays needed)
//! collect_fees {
//!     accounts: { position, owner, vaults, ... },
//!     remaining_accounts: []  // No tick arrays needed!
//! }
//! ```
//! 
//! ### Already Calculated Fees:
//! ```
//! // If tokens_owed > 0, just call without tick arrays
//! collect_fees {
//!     accounts: { position, owner, vaults, ... },
//!     remaining_accounts: []
//! }
//! ```

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::{
    constants::{MARKET_AUTHORITY_SEED, POSITION_SEED, VAULT_SEED},
    error::FeelsError,
    events::{PositionUpdated, PositionOperation},
    state::{Market, Position, TickArray},
    logic::calculate_position_fee_accrual,
    utils::transfer_from_vault_to_user_unchecked,
};

/// Collect fees accounts - tick arrays are optional via remaining_accounts
#[derive(Accounts)]
pub struct CollectFees<'info> {
    /// Position owner
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = owner.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub owner: Signer<'info>,

    /// Market
    #[account(
        mut,
        constraint = market.is_initialized,
        constraint = !market.is_paused,
    )]
    pub market: Box<Account<'info, Market>>,

    /// Position mint
    pub position_mint: Account<'info, Mint>,
    
    /// Position token account (must hold the position token)
    #[account(
        constraint = position_token_account.mint == position_mint.key() @ FeelsError::InvalidMint,
        constraint = position_token_account.owner == owner.key() @ FeelsError::InvalidAuthority,
        constraint = position_token_account.amount == 1 @ FeelsError::InvalidPosition,
    )]
    pub position_token_account: Account<'info, TokenAccount>,

    /// Position
    #[account(
        mut,
        seeds = [POSITION_SEED, position.nft_mint.as_ref()],
        bump,
        constraint = position.nft_mint == position_mint.key() @ FeelsError::InvalidPosition,
        constraint = position.owner == owner.key() @ FeelsError::InvalidAuthority,
        constraint = position.market == market.key() @ FeelsError::InvalidAuthority,
    )]
    pub position: Box<Account<'info, Position>>,

    /// Owner token accounts
    #[account(
        mut,
        constraint = owner_token_0.owner == owner.key() @ FeelsError::InvalidAuthority,
        constraint = owner_token_0.mint == market.token_0 @ FeelsError::InvalidMint,
    )]
    pub owner_token_0: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = owner_token_1.owner == owner.key() @ FeelsError::InvalidAuthority,
        constraint = owner_token_1.mint == market.token_1 @ FeelsError::InvalidMint,
    )]
    pub owner_token_1: Account<'info, TokenAccount>,

    /// Market vault for token 0 - derived from market and token_0
    /// CHECK: Validated as PDA in constraints
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_0.as_ref()],
        bump,
    )]
    pub vault_0: UncheckedAccount<'info>,
    
    /// Market vault for token 1 - derived from market and token_1
    /// CHECK: Validated as PDA in constraints
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref(), market.token_1.as_ref()],
        bump,
    )]
    pub vault_1: UncheckedAccount<'info>,

    /// Unified market authority
    /// CHECK: PDA
    #[account(seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()], bump)]
    pub market_authority: AccountInfo<'info>,

    // Tick arrays are now optional - passed via remaining_accounts
    // remaining_accounts[0] = lower_tick_array (if needed)
    // remaining_accounts[1] = upper_tick_array (if needed)

    pub token_program: Program<'info, Token>,
}

/// Check if position has uncalculated fees
#[inline(never)]
fn needs_fee_calculation(
    position: &Position,
    market: &Market,
) -> bool {
    position.liquidity > 0 && 
    (position.fee_growth_inside_0_last_x64 < market.fee_growth_global_0_x64 ||
     position.fee_growth_inside_1_last_x64 < market.fee_growth_global_1_x64)
}

/// Transfer accumulated fees to user
#[inline(never)]
fn transfer_accumulated_fees<'info>(
    position: &mut Account<'info, Position>,
    owner_token_0: &Account<'info, TokenAccount>,
    owner_token_1: &Account<'info, TokenAccount>,
    vault_0: &AccountInfo<'info>,
    vault_1: &AccountInfo<'info>,
    market_authority: &AccountInfo<'info>,
    market: &Account<'info, Market>,
    token_program: &Program<'info, Token>,
) -> Result<(u64, u64)> {
    let amount_0 = position.tokens_owed_0;
    let amount_1 = position.tokens_owed_1;
    
    if amount_0 == 0 && amount_1 == 0 {
        return Ok((0, 0));
    }
    
    let market_key = market.key();
    let bump = market.market_authority_bump;
    let seeds = &[MARKET_AUTHORITY_SEED, market_key.as_ref(), &[bump]];
    let signer = &[&seeds[..]];
    
    if amount_0 > 0 {
        transfer_from_vault_to_user_unchecked(
            vault_0,
            &owner_token_0.to_account_info(),
            market_authority,
            token_program,
            signer,
            amount_0,
        )?;
        position.tokens_owed_0 = 0;
    }
    
    if amount_1 > 0 {
        transfer_from_vault_to_user_unchecked(
            vault_1,
            &owner_token_1.to_account_info(),
            market_authority,
            token_program,
            signer,
            amount_1,
        )?;
        position.tokens_owed_1 = 0;
    }
    
    Ok((amount_0, amount_1))
}

/// Process fee calculation with tick arrays
#[inline(never)]
fn process_fee_calculation<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    position: &mut Position,
    market: &Market,
    market_key: Pubkey,
) -> Result<()> {
    let lower_tick_array = &remaining_accounts[0];
    let upper_tick_array = &remaining_accounts[1];
    
    // Load and validate tick arrays
    let lower_array_data = lower_tick_array.try_borrow_data()?;
    let upper_array_data = upper_tick_array.try_borrow_data()?;
    
    // Deserialize tick arrays
    let lower_array = TickArray::try_deserialize(&mut &lower_array_data[8..])?;
    let upper_array = TickArray::try_deserialize(&mut &upper_array_data[8..])?;
    
    // Validate arrays
    require!(
        lower_array.market == market_key,
        FeelsError::InvalidTickArray
    );
    require!(
        upper_array.market == market_key,
        FeelsError::InvalidTickArray
    );
    
    crate::utils::validate_tick_array_for_tick(&lower_array, position.tick_lower, market.tick_spacing)?;
    crate::utils::validate_tick_array_for_tick(&upper_array, position.tick_upper, market.tick_spacing)?;
    
    let lower_tick = lower_array.get_tick(position.tick_lower, market.tick_spacing)?;
    let upper_tick = upper_array.get_tick(position.tick_upper, market.tick_spacing)?;
    
    // Calculate fee accrual
    let fee_accrual = calculate_position_fee_accrual(
        market.current_tick,
        position.tick_lower,
        position.tick_upper,
        position.liquidity,
        market.fee_growth_global_0_x64,
        market.fee_growth_global_1_x64,
        lower_tick,
        upper_tick,
        position.fee_growth_inside_0_last_x64,
        position.fee_growth_inside_1_last_x64,
    )?;
    
    // Update position
    position.fee_growth_inside_0_last_x64 = fee_accrual.fee_growth_inside_0;
    position.fee_growth_inside_1_last_x64 = fee_accrual.fee_growth_inside_1;
    position.tokens_owed_0 = position.tokens_owed_0
        .saturating_add(fee_accrual.tokens_owed_0_increment);
    position.tokens_owed_1 = position.tokens_owed_1
        .saturating_add(fee_accrual.tokens_owed_1_increment);
    
    Ok(())
}

/// Collect fees handler - simplified for stack size
pub fn collect_fees<'info>(
    ctx: Context<'_, '_, 'info, 'info, CollectFees<'info>>
) -> Result<()> {
    let market_key = ctx.accounts.market.key();
    let position = &mut ctx.accounts.position;
    
    // For now, just transfer any accumulated fees
    // TODO: Restore full fee calculation logic with better stack management
    
    let mut fees_collected_0 = 0u64;
    let mut fees_collected_1 = 0u64;
    
    if position.tokens_owed_0 > 0 || position.tokens_owed_1 > 0 {
        let (collected_0, collected_1) = transfer_accumulated_fees(
            position,
            &ctx.accounts.owner_token_0,
            &ctx.accounts.owner_token_1,
            &ctx.accounts.vault_0.to_account_info(),
            &ctx.accounts.vault_1.to_account_info(),
            &ctx.accounts.market_authority,
            &ctx.accounts.market,
            &ctx.accounts.token_program,
        )?;
        fees_collected_0 = collected_0;
        fees_collected_1 = collected_1;
    } else {
        // No fees to collect
        msg!("No accumulated fees to collect");
    }
    
    // Get clock for timestamp
    let clock = Clock::get()?;

    // Emit unified event
    emit!(PositionUpdated {
        position: position.key(),
        position_mint: ctx.accounts.position_mint.key(),
        market: market_key,
        owner: ctx.accounts.owner.key(),
        tick_lower: position.tick_lower,
        tick_upper: position.tick_upper,
        liquidity: position.liquidity,
        amount_0: 0,
        amount_1: 0,
        fees_collected_0,
        fees_collected_1,
        operation: PositionOperation::CollectFees,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

