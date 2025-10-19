//! Common validation utilities
//!
//! Comprehensive validation logic for security and data integrity

use crate::{
    constants::*,
    error::FeelsError,
    state::{Market, Position, ProtocolConfig, TickArray, TICK_ARRAY_SIZE},
};
use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

/// Validate that an amount is non-zero and within reasonable bounds
pub fn validate_amount(amount: u64) -> Result<()> {
    require!(amount > 0, FeelsError::ZeroAmount);
    require!(amount <= u64::MAX / 2, FeelsError::AmountOverflow);
    Ok(())
}

/// Validate that amounts for liquidity operations are non-zero
pub fn validate_liquidity_amounts(amount_0: u64, amount_1: u64) -> Result<()> {
    require!(amount_0 > 0 || amount_1 > 0, FeelsError::ZeroAmount);
    require!(
        amount_0 <= u64::MAX / 2 && amount_1 <= u64::MAX / 2,
        FeelsError::AmountOverflow
    );
    Ok(())
}

/// Validate slippage constraints
pub fn validate_slippage(actual: u64, minimum: u64) -> Result<()> {
    require!(actual >= minimum, FeelsError::SlippageExceeded);
    Ok(())
}

/// Validate that a market is operational
pub fn validate_market_active(market: &Market) -> Result<()> {
    require!(market.is_initialized, FeelsError::MarketNotInitialized);
    require!(!market.is_paused, FeelsError::MarketPaused);
    Ok(())
}

/// Validate fee bounds
pub fn validate_fee(fee_bps: u16, max_fee_bps: u16) -> Result<()> {
    require!(
        fee_bps > 0 && fee_bps <= max_fee_bps,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validate tick spacing
pub fn validate_tick_spacing(tick_spacing: u16, max_tick_spacing: u16) -> Result<()> {
    require!(
        tick_spacing > 0 && tick_spacing <= max_tick_spacing,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validate that ticks are properly ordered and aligned
pub fn validate_tick_range(tick_lower: i32, tick_upper: i32, tick_spacing: u16) -> Result<()> {
    require!(tick_lower < tick_upper, FeelsError::InvalidTickRange);
    require!(
        tick_lower % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    require!(
        tick_upper % tick_spacing as i32 == 0,
        FeelsError::TickNotSpaced
    );
    require!(
        tick_lower >= crate::constants::MIN_TICK,
        FeelsError::InvalidTick
    );
    require!(
        tick_upper <= crate::constants::MAX_TICK,
        FeelsError::InvalidTick
    );
    Ok(())
}

/// Calculate expected tick array start index for a given tick
pub fn get_tick_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
    let ticks_per_array = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    let array_index = tick_index.div_euclid(ticks_per_array);
    array_index * ticks_per_array
}

/// Validate that a tick array matches the expected tick
pub fn validate_tick_array_for_tick(
    tick_array: &TickArray,
    tick_index: i32,
    tick_spacing: u16,
) -> Result<()> {
    let expected_start = get_tick_array_start_index(tick_index, tick_spacing);
    require!(
        tick_array.start_tick_index == expected_start,
        FeelsError::InvalidTickArray
    );
    Ok(())
}

/// Validate distribution for token minting
pub fn validate_distribution(
    distribution_total: u64,
    total_supply: u64,
    min_reserve: u64,
) -> Result<()> {
    require!(
        distribution_total <= total_supply - min_reserve,
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validates that a pool includes FeelsSOL as one side
pub fn validate_pool_includes_feelssol(
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
    feelssol_mint: &Pubkey,
) -> Result<()> {
    require!(
        token_0_mint == feelssol_mint || token_1_mint == feelssol_mint,
        FeelsError::InvalidRoute
    );
    Ok(())
}

// ===== NEW COMPREHENSIVE VALIDATIONS =====

/// Validate account ownership
pub fn validate_account_owner(account_info: &AccountInfo, expected_owner: &Pubkey) -> Result<()> {
    require!(
        account_info.owner == expected_owner,
        FeelsError::InvalidAccountOwner
    );
    Ok(())
}

/// Validate that an account is a signer
pub fn validate_signer(account_info: &AccountInfo) -> Result<()> {
    require!(account_info.is_signer, FeelsError::MissingSignature);
    Ok(())
}

/// Validate PDA derivation
pub fn validate_pda(
    account_info: &AccountInfo,
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<()> {
    let (expected_key, _bump) = Pubkey::find_program_address(seeds, program_id);
    require!(account_info.key() == expected_key, FeelsError::InvalidPDA);
    Ok(())
}

/// Validate PDA with specific bump
pub fn validate_pda_with_bump(
    account_info: &AccountInfo,
    seeds: &[&[u8]],
    bump: u8,
    program_id: &Pubkey,
) -> Result<()> {
    let expected_key = Pubkey::create_program_address(&[seeds, &[&[bump]]].concat(), program_id)
        .map_err(|_| FeelsError::InvalidPDA)?;

    require!(account_info.key() == expected_key, FeelsError::InvalidPDA);
    Ok(())
}

/// Validate PDA with known bump - optimized version
/// Use this when you already have the bump stored in state
pub fn validate_pda_with_known_bump(
    account_key: &Pubkey,
    seeds: &[&[u8]],
    bump: u8,
    program_id: &Pubkey,
) -> Result<()> {
    let mut seeds_with_bump = seeds.to_vec();
    let bump_array = [bump];
    seeds_with_bump.push(&bump_array);

    let expected_key = Pubkey::create_program_address(&seeds_with_bump, program_id)
        .map_err(|_| FeelsError::InvalidPDA)?;

    require!(account_key == &expected_key, FeelsError::InvalidPDA);
    Ok(())
}

/// Validate time-based constraints
pub fn validate_time_constraint(
    current_timestamp: i64,
    last_action_timestamp: i64,
    minimum_interval: i64,
) -> Result<()> {
    require!(
        current_timestamp >= last_action_timestamp + minimum_interval,
        FeelsError::CooldownActive
    );
    Ok(())
}

/// Validate slot-based constraints
pub fn validate_slot_constraint(
    current_slot: u64,
    last_action_slot: u64,
    minimum_slots: u64,
) -> Result<()> {
    require!(
        current_slot >= last_action_slot + minimum_slots,
        FeelsError::TooEarly
    );
    Ok(())
}

/// Validate position ownership
pub fn validate_position_ownership(position: &Position, owner: &Pubkey) -> Result<()> {
    require!(position.owner == *owner, FeelsError::InvalidPositionOwner);
    Ok(())
}

/// Validate position is for the correct market
pub fn validate_position_market(position: &Position, market: &Pubkey) -> Result<()> {
    require!(position.market == *market, FeelsError::InvalidMarket);
    Ok(())
}

/// Validate sqrt price bounds
pub fn validate_sqrt_price(sqrt_price: u128) -> Result<()> {
    require!(
        (MIN_SQRT_PRICE..=MAX_SQRT_PRICE).contains(&sqrt_price),
        FeelsError::InvalidPrice
    );
    Ok(())
}

/// Validate liquidity bounds
pub fn validate_liquidity(liquidity: u128) -> Result<()> {
    require!(
        liquidity >= MIN_LIQUIDITY,
        FeelsError::LiquidityBelowMinimum
    );
    require!(liquidity <= MAX_LIQUIDITY, FeelsError::LiquidityOverflow);
    Ok(())
}

/// Validate authority matches protocol config
pub fn validate_protocol_authority(
    authority: &Pubkey,
    protocol_config: &ProtocolConfig,
) -> Result<()> {
    require!(
        *authority == protocol_config.authority,
        FeelsError::InvalidAuthority
    );
    Ok(())
}

/// Validate token mint matches expected
pub fn validate_token_mint(token_account: &AccountInfo, expected_mint: &Pubkey) -> Result<()> {
    let data = token_account.try_borrow_data()?;
    let mint_offset = 0; // mint is first field in TokenAccount
    let mint_bytes = &data[mint_offset..mint_offset + 32];
    let actual_mint =
        Pubkey::new_from_array(mint_bytes.try_into().map_err(|_| FeelsError::InvalidMint)?);

    require!(actual_mint == *expected_mint, FeelsError::InvalidMint);
    Ok(())
}

/// Validate that a value hasn't decreased (monotonicity check)
pub fn validate_monotonic_increase(new_value: u64, old_value: u64) -> Result<()> {
    require!(new_value >= old_value, FeelsError::InvalidUpdate);
    Ok(())
}

/// Validate buffer threshold bounds
pub fn validate_buffer_threshold(threshold: u64) -> Result<()> {
    require!(
        (MIN_BUFFER_THRESHOLD..=MAX_BUFFER_THRESHOLD).contains(&threshold),
        FeelsError::InvalidThreshold
    );
    Ok(())
}

/// Validate oracle staleness
pub fn validate_oracle_freshness(
    current_timestamp: i64,
    oracle_timestamp: i64,
    max_age: i64,
) -> Result<()> {
    require!(
        current_timestamp - oracle_timestamp <= max_age,
        FeelsError::OracleStale
    );
    Ok(())
}

/// Validate fee cap
pub fn validate_fee_cap(fee_amount: u64, amount_in: u64, max_fee_bps: u16) -> Result<()> {
    let max_fee = (amount_in as u128)
        .saturating_mul(max_fee_bps as u128)
        .saturating_div(10000) as u64;

    require!(fee_amount <= max_fee, FeelsError::FeeCapExceeded);
    Ok(())
}

/// Validate rate limit
pub fn validate_rate_limit(current_amount: u64, new_amount: u64, cap: u64) -> Result<()> {
    let total = current_amount
        .checked_add(new_amount)
        .ok_or(FeelsError::MathOverflow)?;

    require!(total <= cap, FeelsError::RateLimitExceeded);
    Ok(())
}

/// Validate account is not closed
pub fn validate_account_not_closed(account_info: &AccountInfo) -> Result<()> {
    require!(account_info.lamports() > 0, FeelsError::AccountClosed);
    Ok(())
}

/// Validate account has sufficient lamports for rent
pub fn validate_rent_exempt(account_info: &AccountInfo, rent: &Rent) -> Result<()> {
    let data_len = account_info.data_len();
    let required_lamports = rent.minimum_balance(data_len);

    require!(
        account_info.lamports() >= required_lamports,
        FeelsError::NotRentExempt
    );
    Ok(())
}

/// Validate token amounts don't overflow when combined
pub fn validate_token_amounts_safe(amount_0: u64, amount_1: u64) -> Result<()> {
    amount_0
        .checked_add(amount_1)
        .ok_or(FeelsError::MathOverflow)?;
    Ok(())
}

/// Validate sqrt price movement bounds
pub fn validate_sqrt_price_movement(
    old_price: u128,
    new_price: u128,
    max_movement_bps: u16,
) -> Result<()> {
    let movement = if new_price > old_price {
        (new_price - old_price)
            .saturating_mul(10000)
            .saturating_div(old_price)
    } else {
        (old_price - new_price)
            .saturating_mul(10000)
            .saturating_div(old_price)
    };

    require!(
        movement <= max_movement_bps as u128,
        FeelsError::PriceMovementTooLarge
    );
    Ok(())
}

// ===== CONSTANTS =====
pub const MIN_SQRT_PRICE: u128 = 4295048016; // Approximately 1.0001^(-443636)
pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055; // Approximately 1.0001^443636
pub const MIN_BUFFER_THRESHOLD: u64 = 100_000; // 0.1 tokens minimum
pub const MAX_BUFFER_THRESHOLD: u64 = 100_000_000_000; // 100k tokens maximum
