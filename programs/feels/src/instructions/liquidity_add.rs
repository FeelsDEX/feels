/// Adds liquidity to a concentrated liquidity position within a specified price range.
/// Calculates the required amounts of both tokens based on the current pool price and
/// the position's price bounds. Updates tick liquidity data and mints position NFTs
/// to track ownership. Liquidity providers earn fees proportional to their share.

use crate::state::{TickArray, PoolError};
use crate::constant::TICK_ARRAY_SIZE;
// SafeMath trait removed - using native checked arithmetic
use crate::logic::tick::TickArrayManager;
use crate::logic::event::{LiquidityEvent, LiquidityEventType};
use anchor_lang::prelude::*;
use crate::utils::cpi_helpers::transfer_tokens;

// ============================================================================
// Handler Functions
// ============================================================================

/// Add liquidity to a concentrated liquidity position
pub fn handler(
    ctx: Context<crate::AddLiquidity>,
    liquidity_amount: u128,
    amount_0_max: u64,
    amount_1_max: u64,
) -> Result<(u64, u64)> {
    require!(liquidity_amount > 0, PoolError::InvalidAmount);
    require!(amount_0_max > 0 && amount_1_max > 0, PoolError::InvalidAmount);
    
    let mut pool = ctx.accounts.pool.load_mut()?;
    
    // Get position data before mutable borrow
    let position_key = ctx.accounts.tick_position_metadata.key();
    let position = &mut ctx.accounts.tick_position_metadata;
    
    // Store tick values for later use
    let tick_lower = position.tick_lower;
    let tick_upper = position.tick_upper;
    
    // Validate tick range
    require!(
        tick_lower < tick_upper,
        PoolError::InvalidTickRange
    );
    
    // Calculate required token amounts based on current price and liquidity
    let (amount_0, amount_1) = calculate_token_amounts(
        liquidity_amount,
        tick_lower,
        tick_upper,
        pool.current_sqrt_price,
    )?;
    
    // Validate against maximum amounts
    require!(amount_0 <= amount_0_max, PoolError::SlippageExceeded);
    require!(amount_1 <= amount_1_max, PoolError::SlippageExceeded);
    
    // Ensure tick arrays exist for both ticks, creating them if necessary
    TickArrayManager::ensure_tick_array_exists(
        &mut pool,
        &ctx.accounts.pool.key(),
        tick_lower,
        &ctx.accounts.tick_array_lower.to_account_info(),
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        ctx.program_id,
    )?;
    
    TickArrayManager::ensure_tick_array_exists(
        &mut pool,
        &ctx.accounts.pool.key(),
        tick_upper,
        &ctx.accounts.tick_array_upper.to_account_info(),
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        ctx.program_id,
    )?;
    
    // Initialize or update ticks if needed
    update_tick_liquidity(
        &mut ctx.accounts.tick_array_lower,
        tick_lower,
        liquidity_amount as i128,
        false, // not removing
    )?;
    
    update_tick_liquidity(
        &mut ctx.accounts.tick_array_upper,
        tick_upper,
        -(liquidity_amount as i128), // negative for upper tick
        false,
    )?;
    
    // Transfer tokens from user to pool vaults
    // Note: Transfer logic kept inline for Phase 2 Valence hook integration.
    // Different operations will transition to atomic position vault adjustments at different times.
    transfer_tokens(
        ctx.accounts.user_token_0.to_account_info(),
        ctx.accounts.pool_token_0.to_account_info(),
        ctx.accounts.user.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        amount_0,
        &[],
    )?;
    
    transfer_tokens(
        ctx.accounts.user_token_1.to_account_info(),
        ctx.accounts.pool_token_1.to_account_info(),
        ctx.accounts.user.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        amount_1,
        &[],
    )?;
    
    // Update position metadata using checked arithmetic
    position.liquidity = position.liquidity.checked_add(liquidity_amount)
        .ok_or(PoolError::LiquidityOverflow)?;
    
    // Update global liquidity if position is in range using checked arithmetic
    if pool.current_tick >= tick_lower && pool.current_tick < tick_upper {
        pool.liquidity = pool.liquidity.checked_add(liquidity_amount)
            .ok_or(PoolError::LiquidityOverflow)?;
    }
    
    // Update volume statistics using checked arithmetic
    pool.total_volume_0 = pool.total_volume_0.checked_add(amount_0 as u128)
        .ok_or(PoolError::MathOverflow)?;
    pool.total_volume_1 = pool.total_volume_1.checked_add(amount_1 as u128)
        .ok_or(PoolError::MathOverflow)?;
    
    // Position data already stored at the beginning
    
    // Update last update slot
    let clock = Clock::get()?;
    pool.last_update_slot = clock.slot;
    
    emit!(LiquidityEvent {
        pool: ctx.accounts.pool.key(),
        position: position_key,
        liquidity_delta: liquidity_amount as i128,
        amount_0,
        amount_1,
        tick_lower,
        tick_upper,
        event_type: LiquidityEventType::Add,
        timestamp: clock.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}

/// Calculate required token amounts for given liquidity
fn calculate_token_amounts(
    liquidity: u128,
    tick_lower: i32,
    tick_upper: i32,
    current_sqrt_price: u128,
) -> Result<(u64, u64)> {
    use crate::logic::concentrated_liquidity::ConcentratedLiquidityMath;
    use crate::utils::TickMath;
    
    let sqrt_price_lower = TickMath::get_sqrt_ratio_at_tick(tick_lower)?;
    let sqrt_price_upper = TickMath::get_sqrt_ratio_at_tick(tick_upper)?;
    
    // Delegate to the standardized concentrated liquidity math implementation
    ConcentratedLiquidityMath::get_amounts_for_concentrated_liquidity(
        current_sqrt_price,
        sqrt_price_lower,
        sqrt_price_upper,
        liquidity,
    )
}

/// Update tick liquidity in tick array
fn update_tick_liquidity(
    tick_array: &mut AccountLoader<TickArray>,
    tick_index: i32,
    liquidity_delta: i128,
    _is_remove: bool,
) -> Result<()> {
    let mut array = tick_array.load_mut()?;
    
    // Find the tick in the array
    let array_start = array.start_tick_index;
    let relative_index = (tick_index - array_start) as usize;
    
    require!(
        relative_index < TICK_ARRAY_SIZE,
        PoolError::InvalidTickRange
    );
    
    // Update liquidity initialization first
    if array.ticks[relative_index].initialized == 0 {
        array.ticks[relative_index].initialized = 1;
        array.initialized_tick_count += 1;
    }
    
    let tick = &mut array.ticks[relative_index];
    
    tick.liquidity_net = tick.liquidity_net.checked_add(liquidity_delta)
        .ok_or(PoolError::MathOverflow)?;
    
    tick.liquidity_gross = if liquidity_delta > 0 {
        tick.liquidity_gross.checked_add(liquidity_delta as u128)
            .ok_or(PoolError::MathOverflow)?
    } else {
        tick.liquidity_gross.checked_sub((-liquidity_delta) as u128)
            .ok_or(PoolError::MathOverflow)?
    };
    
    Ok(())
}
