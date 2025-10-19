//! Open position with metadata instruction
//!
//! Wrapper instruction that creates a position and adds
//! Metaplex metadata to make it a proper NFT.

use crate::{
    constants::{POSITION_SEED, VAULT_SEED},
    error::FeelsError,
    events::{PositionOperation, PositionUpdated},
    state::{Market, Position, TickArray},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use mpl_token_metadata::{
    instructions as mpl_instruction, types::DataV2, ID as METADATA_PROGRAM_ID,
};

#[derive(Accounts)]
pub struct OpenPositionWithMetadata<'info> {
    /// Liquidity provider
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub provider: Signer<'info>,

    /// Market state
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub market: Account<'info, Market>,

    /// Position mint - will become an NFT with metadata
    #[account(
        init,
        payer = provider,
        mint::decimals = 0,
        mint::authority = position,
        mint::freeze_authority = position,
    )]
    pub position_mint: Account<'info, Mint>,

    /// Position token account
    #[account(
        init,
        payer = provider,
        token::mint = position_mint,
        token::authority = provider,
    )]
    pub position_token_account: Account<'info, TokenAccount>,

    /// Position account (PDA)
    #[account(
        init,
        payer = provider,
        space = Position::LEN,
        seeds = [POSITION_SEED, position_mint.key().as_ref()],
        bump,
    )]
    pub position: Account<'info, Position>,

    /// Metadata account (PDA of Metaplex Token Metadata program)
    /// CHECK: Created by Metaplex program
    #[account(
        mut,
        seeds = [
            b"metadata",
            METADATA_PROGRAM_ID.as_ref(),
            position_mint.key().as_ref(),
        ],
        bump,
        seeds::program = METADATA_PROGRAM_ID,
    )]
    pub metadata: AccountInfo<'info>,

    /// Provider's token account for token 0
    /// CHECK: Validated in handler
    #[account(mut)]
    pub provider_token_0: UncheckedAccount<'info>,

    /// Provider's token account for token 1
    /// CHECK: Validated in handler
    #[account(mut)]
    pub provider_token_1: UncheckedAccount<'info>,

    /// Market vault for token 0 - derived from market and token_0
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// Market vault for token 1 - derived from market and token_1
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// Tick array containing the lower tick
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub lower_tick_array: AccountLoader<'info, TickArray>,

    /// Tick array containing the upper tick
    /// CHECK: Validated in handler to reduce stack usage
    #[account(mut)]
    pub upper_tick_array: AccountLoader<'info, TickArray>,

    /// Metaplex Token Metadata program
    /// CHECK: Validated in handler to reduce stack usage
    pub metadata_program: AccountInfo<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn open_position_with_metadata(
    ctx: Context<OpenPositionWithMetadata>,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_amount: u128,
) -> Result<()> {
    // Validate constraints (moved from struct to save stack space)
    require!(
        ctx.accounts.provider.owner == &System::id(),
        FeelsError::InvalidAuthority
    );
    require!(
        ctx.accounts.market.is_initialized,
        FeelsError::MarketNotInitialized
    );
    require!(!ctx.accounts.market.is_paused, FeelsError::MarketPaused);
    require!(
        ctx.accounts.metadata_program.key() == METADATA_PROGRAM_ID,
        FeelsError::InvalidAccount
    );

    // Validate vault PDAs
    let (expected_vault_0, _) = Pubkey::find_program_address(
        &[
            VAULT_SEED,
            ctx.accounts.market.key().as_ref(),
            ctx.accounts.market.token_0.as_ref(),
        ],
        &crate::ID,
    );
    let (expected_vault_1, _) = Pubkey::find_program_address(
        &[
            VAULT_SEED,
            ctx.accounts.market.key().as_ref(),
            ctx.accounts.market.token_1.as_ref(),
        ],
        &crate::ID,
    );
    require!(
        ctx.accounts.vault_0.key() == expected_vault_0,
        FeelsError::InvalidVault
    );
    require!(
        ctx.accounts.vault_1.key() == expected_vault_1,
        FeelsError::InvalidVault
    );

    // Tick array validation is done later with more detailed checks

    // First, execute the core open position logic inline
    // This duplicates the logic but avoids complex cross-handler calls
    let market = &mut ctx.accounts.market;
    let position = &mut ctx.accounts.position;
    let position_key = position.key();
    let clock = Clock::get()?;

    // Validate tick range and alignment
    crate::utils::validate_tick_range(tick_lower, tick_upper, market.tick_spacing)?;
    require!(liquidity_amount > 0, FeelsError::ZeroLiquidity);

    // Check against minimum liquidity to prevent dust positions
    require!(
        liquidity_amount >= crate::constants::MIN_LIQUIDITY,
        FeelsError::LiquidityBelowMinimum
    );

    // Validate that tick arrays match the expected ticks
    {
        let lower_array = ctx.accounts.lower_tick_array.load()?;
        let upper_array = ctx.accounts.upper_tick_array.load()?;
        crate::utils::validate_tick_array_for_tick(&lower_array, tick_lower, market.tick_spacing)?;
        crate::utils::validate_tick_array_for_tick(&upper_array, tick_upper, market.tick_spacing)?;
    }

    // Manually deserialize and validate provider token accounts
    let provider_token_0 =
        TokenAccount::try_deserialize(&mut &ctx.accounts.provider_token_0.data.borrow()[..])?;
    let provider_token_1 =
        TokenAccount::try_deserialize(&mut &ctx.accounts.provider_token_1.data.borrow()[..])?;

    // Validate provider token accounts
    require!(
        provider_token_0.owner == ctx.accounts.provider.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        provider_token_1.owner == ctx.accounts.provider.key(),
        FeelsError::InvalidAuthority
    );
    require!(
        provider_token_0.mint == market.token_0,
        FeelsError::InvalidMint
    );
    require!(
        provider_token_1.mint == market.token_1,
        FeelsError::InvalidMint
    );

    // Manually deserialize and validate vault accounts
    let _vault_0 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_0.data.borrow()[..])?;
    let _vault_1 = TokenAccount::try_deserialize(&mut &ctx.accounts.vault_1.data.borrow()[..])?;

    // Calculate token amounts using unified function
    let sqrt_price_lower = crate::logic::sqrt_price_from_tick(tick_lower)?;
    let sqrt_price_upper = crate::logic::sqrt_price_from_tick(tick_upper)?;
    let sqrt_price_current = market.sqrt_price;

    // Use unified amount calculation function (same as swap logic)
    let (amount_0, amount_1) = crate::logic::amounts_from_liquidity(
        sqrt_price_current,
        sqrt_price_lower,
        sqrt_price_upper,
        liquidity_amount,
    )?;

    // Initialize position state
    position.nft_mint = ctx.accounts.position_mint.key();
    position.market = market.key();
    position.owner = ctx.accounts.provider.key();
    position.tick_lower = tick_lower;
    position.tick_upper = tick_upper;
    position.liquidity = liquidity_amount;

    // CRITICAL: Initialize ticks FIRST before reading fee growth values
    // This ensures fee_growth_outside is properly set based on tick position
    {
        let mut lower_array = ctx.accounts.lower_tick_array.load_mut()?;
        lower_array.init_tick(
            tick_lower,
            market.tick_spacing,
            market.current_tick,
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
        )?;
        lower_array.update_liquidity(
            tick_lower,
            market.tick_spacing,
            liquidity_amount as i128,
            false,
        )?;
    }
    {
        let mut upper_array = ctx.accounts.upper_tick_array.load_mut()?;
        upper_array.init_tick(
            tick_upper,
            market.tick_spacing,
            market.current_tick,
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
        )?;
        upper_array.update_liquidity(
            tick_upper,
            market.tick_spacing,
            liquidity_amount as i128,
            true,
        )?;
    }

    // NOW take fee growth snapshot after ticks are initialized
    let (fee_growth_inside_0, fee_growth_inside_1) = {
        let lower_array = ctx.accounts.lower_tick_array.load()?;
        let upper_array = ctx.accounts.upper_tick_array.load()?;
        let lower_tick = lower_array.get_tick(tick_lower, market.tick_spacing)?;
        let upper_tick = upper_array.get_tick(tick_upper, market.tick_spacing)?;

        // Calculate initial fee growth using the function
        let fee_accrual = crate::logic::calculate_position_fee_accrual(
            market.current_tick,
            tick_lower,
            tick_upper,
            0, // No liquidity yet for fee calculation
            market.fee_growth_global_0_x64,
            market.fee_growth_global_1_x64,
            lower_tick,
            upper_tick,
            0, // No previous fee growth
            0, // No previous fee growth
        )?;

        (
            fee_accrual.fee_growth_inside_0,
            fee_accrual.fee_growth_inside_1,
        )
    };

    position.fee_growth_inside_0_last_x64 = fee_growth_inside_0;
    position.fee_growth_inside_1_last_x64 = fee_growth_inside_1;
    position.tokens_owed_0 = 0;
    position.tokens_owed_1 = 0;
    position.position_bump = ctx.bumps.position;

    // Update market liquidity if position is in range
    if market.current_tick >= tick_lower && market.current_tick < tick_upper {
        market.liquidity = crate::utils::add_liquidity(market.liquidity, liquidity_amount)?;
    }

    // Transfer tokens
    if amount_0 > 0 {
        crate::utils::transfer_from_user_to_vault_unchecked(
            &ctx.accounts.provider_token_0.to_account_info(),
            &ctx.accounts.vault_0.to_account_info(),
            &ctx.accounts.provider,
            &ctx.accounts.token_program,
            amount_0,
        )?;
    }

    if amount_1 > 0 {
        crate::utils::transfer_from_user_to_vault_unchecked(
            &ctx.accounts.provider_token_1.to_account_info(),
            &ctx.accounts.vault_1.to_account_info(),
            &ctx.accounts.provider,
            &ctx.accounts.token_program,
            amount_1,
        )?;
    }

    // Mint position token
    let position_bump = ctx.bumps.position;
    let position_mint_key = ctx.accounts.position_mint.key();
    let seeds = &[POSITION_SEED, position_mint_key.as_ref(), &[position_bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = anchor_spl::token::MintTo {
        mint: ctx.accounts.position_mint.to_account_info(),
        to: ctx.accounts.position_token_account.to_account_info(),
        authority: ctx.accounts.position.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    anchor_spl::token::mint_to(cpi_ctx, 1)?;

    // Emit unified position event
    emit!(PositionUpdated {
        position: position_key,
        position_mint: ctx.accounts.position_mint.key(),
        market: market.key(),
        owner: ctx.accounts.provider.key(),
        tick_lower,
        tick_upper,
        liquidity: liquidity_amount,
        amount_0,
        amount_1,
        fees_collected_0: 0,
        fees_collected_1: 0,
        operation: PositionOperation::Open,
        timestamp: clock.unix_timestamp,
    });

    // Then add Metaplex metadata to make it a proper NFT
    let position_mint_key = ctx.accounts.position_mint.key();
    let _market = &ctx.accounts.market;

    // Generate metadata for the NFT
    let name = format!("Feels Position #{}", &position_mint_key.to_string()[0..8]);
    let symbol = "FEELS-POS".to_string();
    let uri = format!("https://api.feels.market/position/{}", position_mint_key);

    // Create metadata account via CPI to Metaplex
    let position_bump = ctx.bumps.position;
    let seeds = &[POSITION_SEED, position_mint_key.as_ref(), &[position_bump]];
    let signer_seeds = &[&seeds[..]];

    // Build metadata instruction using the new API
    let create_metadata_accounts_v3 = mpl_instruction::CreateMetadataAccountV3 {
        metadata: ctx.accounts.metadata.key(),
        mint: ctx.accounts.position_mint.key(),
        mint_authority: position_key,
        payer: ctx.accounts.provider.key(),
        update_authority: (position_key, true), // true = is_signer
        system_program: ctx.accounts.system_program.key(),
        rent: Some(ctx.accounts.rent.key()),
    };

    let args = mpl_instruction::CreateMetadataAccountV3InstructionArgs {
        data: DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        is_mutable: true,
        collection_details: None,
    };

    let create_metadata_accounts_v3_ix = create_metadata_accounts_v3.instruction(args);

    anchor_lang::solana_program::program::invoke_signed(
        &create_metadata_accounts_v3_ix,
        &[
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.position_mint.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.provider.to_account_info(),
            ctx.accounts.position.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        signer_seeds,
    )?;

    // The position is now a proper NFT that will appear in wallets
    msg!(
        "Position NFT created with metadata at {}",
        ctx.accounts.metadata.key()
    );

    Ok(())
}
