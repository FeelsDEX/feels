//! Close position with metadata instruction
//! 
//! Wrapper instruction that closes a position and handles NFT metadata cleanup.
//! This removes the position and ensures proper cleanup of Metaplex metadata.

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, TokenAccount, Burn};
use mpl_token_metadata::{
    instructions as mpl_instruction,
    ID as METADATA_PROGRAM_ID,
};
use crate::{
    constants::{POSITION_SEED, MARKET_AUTHORITY_SEED, VAULT_SEED},
    error::FeelsError,
    events::{PositionUpdated, PositionOperation},
    state::{Market, Position, TickArray},
    utils::{
        validate_slippage, validate_market_active,
        transfer_from_vault_to_user_unchecked, subtract_liquidity,
    },
    logic::{calculate_position_fee_accrual, amounts_from_liquidity},
};

#[derive(Accounts)]
pub struct ClosePositionWithMetadata<'info> {
    /// Position owner
    /// SECURITY: Must be a system account to prevent PDA identity confusion
    #[account(
        mut,
        constraint = owner.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub owner: Signer<'info>,
    
    /// Market state
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    /// Position mint
    /// CHECK: Validated in handler
    #[account(mut)]
    pub position_mint: UncheckedAccount<'info>,
    
    /// Position token account
    /// CHECK: Validated in handler
    #[account(mut)]
    pub position_token_account: UncheckedAccount<'info>,
    
    /// Position account (PDA)
    /// SECURITY: Removed `close = owner` to prevent fee theft vulnerability.
    /// Position must be closed in a separate instruction after verification.
    #[account(
        mut,
        seeds = [POSITION_SEED, position.nft_mint.as_ref()],
        bump,
        constraint = position.market == market.key() @ FeelsError::InvalidMarket,
        constraint = position.owner == owner.key() @ FeelsError::InvalidAuthority,
    )]
    pub position: Account<'info, Position>,
    
    /// Metadata account (will be closed)
    /// CHECK: Validated by Metaplex program
    #[account(
        mut,
        seeds = [
            b"metadata",
            METADATA_PROGRAM_ID.as_ref(),
            position.nft_mint.as_ref(),
        ],
        bump,
        seeds::program = METADATA_PROGRAM_ID,
    )]
    pub metadata: AccountInfo<'info>,
    
    /// Owner's token account for token 0
    /// CHECK: Validated in handler
    #[account(mut)]
    pub owner_token_0: UncheckedAccount<'info>,
    
    /// Owner's token account for token 1
    /// CHECK: Validated in handler
    #[account(mut)]
    pub owner_token_1: UncheckedAccount<'info>,
    
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
    
    /// Unified market authority PDA
    /// CHECK: PDA signer for vault operations
    #[account(
        seeds = [MARKET_AUTHORITY_SEED, market.key().as_ref()],
        bump,
    )]
    pub market_authority: AccountInfo<'info>,
    
    /// Tick array containing the lower tick
    #[account(
        mut,
        constraint = lower_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray,
    )]
    pub lower_tick_array: AccountLoader<'info, TickArray>,
    
    /// Tick array containing the upper tick
    #[account(
        mut,
        constraint = upper_tick_array.load()?.market == market.key() @ FeelsError::InvalidTickArray,
    )]
    pub upper_tick_array: AccountLoader<'info, TickArray>,
    
    /// Metaplex Token Metadata program
    /// CHECK: Address verified in constraint
    #[account(address = METADATA_PROGRAM_ID)]
    pub metadata_program: AccountInfo<'info>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
}

/// Helper to validate tick arrays (moved out to reduce stack usage)
#[inline(never)]
fn validate_tick_arrays(
    lower_tick_array: &AccountLoader<TickArray>,
    upper_tick_array: &AccountLoader<TickArray>,
    tick_lower: i32,
    tick_upper: i32,
    tick_spacing: u16,
) -> Result<()> {
    let lower_array = lower_tick_array.load()?;
    let upper_array = upper_tick_array.load()?;
    crate::utils::validate_tick_array_for_tick(&lower_array, tick_lower, tick_spacing)?;
    crate::utils::validate_tick_array_for_tick(&upper_array, tick_upper, tick_spacing)?;
    Ok(())
}

pub fn close_position_with_metadata(
    ctx: Context<ClosePositionWithMetadata>,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let position = &ctx.accounts.position;
    let clock = Clock::get()?;
    
    // Validate market is active
    validate_market_active(market)?;
    
    // Manually deserialize and validate position mint
    let position_mint = Mint::try_deserialize(&mut &ctx.accounts.position_mint.data.borrow()[..])?;
    require!(
        position_mint.supply == 1,
        FeelsError::InvalidPosition
    );
    require!(
        position.nft_mint == ctx.accounts.position_mint.key(),
        FeelsError::InvalidPosition
    );
    
    // Manually deserialize and validate position token account
    let position_token_account = TokenAccount::try_deserialize(&mut &ctx.accounts.position_token_account.data.borrow()[..])?;
    require!(
        position_token_account.mint == ctx.accounts.position_mint.key(),
        FeelsError::InvalidMint
    );
    require!(
        position_token_account.owner == ctx.accounts.owner.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        position_token_account.amount == 1,
        FeelsError::InvalidPosition
    );
    
    // Get position details
    let tick_lower = position.tick_lower;
    let tick_upper = position.tick_upper;
    let liquidity = position.liquidity;
    
    require!(liquidity > 0, FeelsError::ZeroLiquidity);
    
    // Validate that tick arrays match the expected ticks
    validate_tick_arrays(
        &ctx.accounts.lower_tick_array,
        &ctx.accounts.upper_tick_array,
        tick_lower,
        tick_upper,
        market.tick_spacing,
    )?;
    
    // Calculate fee accrual using the reusable function
    let (fees_0, fees_1) = {
        let lower_array = ctx.accounts.lower_tick_array.load()?;
        let upper_array = ctx.accounts.upper_tick_array.load()?;
        let lower_tick = lower_array.get_tick(tick_lower, market.tick_spacing)?;
        let upper_tick = upper_array.get_tick(tick_upper, market.tick_spacing)?;
        
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
        
        (fee_accrual.tokens_owed_0_increment, fee_accrual.tokens_owed_1_increment)
    };
    
    // Calculate amounts to return based on current price
    let sqrt_price_lower = crate::logic::sqrt_price_from_tick(tick_lower)?;
    let sqrt_price_upper = crate::logic::sqrt_price_from_tick(tick_upper)?;
    let sqrt_price_current = market.sqrt_price;
    
    // Use unified amount calculation function (same as swap logic)
    let (amount_0, amount_1) = amounts_from_liquidity(
        sqrt_price_current,
        sqrt_price_lower,
        sqrt_price_upper,
        liquidity,
    )?;
    
    // Add fees to amounts
    let total_amount_0 = amount_0.saturating_add(fees_0).saturating_add(position.tokens_owed_0);
    let total_amount_1 = amount_1.saturating_add(fees_1).saturating_add(position.tokens_owed_1);
    
    // Validate slippage
    validate_slippage(total_amount_0, amount_0_min)?;
    validate_slippage(total_amount_1, amount_1_min)?;
    
    // Manually deserialize and validate vault accounts
    let _vault_0 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_0.data.borrow()[..])?;
    let _vault_1 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_1.data.borrow()[..])?;
    
    // Manually deserialize and validate owner token accounts
    let owner_token_0 = TokenAccount::try_deserialize(&mut &ctx.accounts.owner_token_0.data.borrow()[..])?;
    let owner_token_1 = TokenAccount::try_deserialize(&mut &ctx.accounts.owner_token_1.data.borrow()[..])?;
    
    // Validate owner token accounts
    require!(
        owner_token_0.owner == ctx.accounts.owner.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        owner_token_1.owner == ctx.accounts.owner.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        owner_token_0.mint == market.token_0,
        FeelsError::InvalidMint
    );
    require!(
        owner_token_1.mint == market.token_1,
        FeelsError::InvalidMint
    );
    
    // Update tick arrays - remove liquidity
    {
        let mut lower_array = ctx.accounts.lower_tick_array.load_mut()?;
        lower_array.update_liquidity(tick_lower, market.tick_spacing, -(liquidity as i128), false)?;
    }
    {
        let mut upper_array = ctx.accounts.upper_tick_array.load_mut()?;
        upper_array.update_liquidity(tick_upper, market.tick_spacing, -(liquidity as i128), true)?;
    }
    
    // Update market liquidity if position was in range
    if market.current_tick >= tick_lower && market.current_tick < tick_upper {
        market.liquidity = subtract_liquidity(market.liquidity, liquidity)?;
    }
    
    // Transfer tokens to owner
    // Use stored bump for performance (avoids PDA derivation)
    let market_authority_bump = market.market_authority_bump;
    let market_key = market.key();
    let seeds = &[
        MARKET_AUTHORITY_SEED,
        market_key.as_ref(),
        &[market_authority_bump],
    ];
    let signer_seeds = &[&seeds[..]];
    
    if total_amount_0 > 0 {
        transfer_from_vault_to_user_unchecked(
            &ctx.accounts.vault_0.to_account_info(),
            &ctx.accounts.owner_token_0.to_account_info(),
            &ctx.accounts.market_authority,
            &ctx.accounts.token_program,
            signer_seeds,
            total_amount_0,
        )?;
    }
    
    if total_amount_1 > 0 {
        transfer_from_vault_to_user_unchecked(
            &ctx.accounts.vault_1.to_account_info(),
            &ctx.accounts.owner_token_1.to_account_info(),
            &ctx.accounts.market_authority,
            &ctx.accounts.token_program,
            signer_seeds,
            total_amount_1,
        )?;
    }
    
    // First handle metadata cleanup - burn the metadata account
    // This needs to happen before the position token is burned
    // Burn metadata account via CPI to Metaplex
    // Note: We're not using master edition, so we skip that account
    let burn_nft = mpl_instruction::BurnNft {
        metadata: ctx.accounts.metadata.key(),
        owner: ctx.accounts.owner.key(),
        mint: ctx.accounts.position_mint.key(),
        token_account: ctx.accounts.position_token_account.key(),
        master_edition_account: ctx.accounts.metadata.key(), // Using metadata as placeholder
        spl_token_program: ctx.accounts.token_program.key(),
        collection_metadata: None,
    };
    
    let burn_metadata_ix = burn_nft.instruction();
    
    anchor_lang::solana_program::program::invoke(
        &burn_metadata_ix,
        &[
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.position_mint.to_account_info(),
            ctx.accounts.position_token_account.to_account_info(),
            ctx.accounts.owner.to_account_info(),
        ],
    )?;
    
    // Burn position token
    let cpi_accounts = Burn {
        mint: ctx.accounts.position_mint.to_account_info(),
        from: ctx.accounts.position_token_account.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::burn(cpi_ctx, 1)?;
    
    // Emit unified event
    emit!(PositionUpdated {
        position: position.key(),
        position_mint: ctx.accounts.position_mint.key(),
        market: market.key(),
        owner: ctx.accounts.owner.key(),
        tick_lower,
        tick_upper,
        liquidity,
        amount_0,
        amount_1,
        fees_collected_0: fees_0.saturating_add(position.tokens_owed_0),
        fees_collected_1: fees_1.saturating_add(position.tokens_owed_1),
        operation: PositionOperation::Close,
        timestamp: clock.unix_timestamp,
    });
    
    // Position account will be closed automatically due to close = owner constraint
    
    #[cfg(feature = "telemetry")]
    msg!("Position NFT closed and metadata burned");
    
    Ok(())
}