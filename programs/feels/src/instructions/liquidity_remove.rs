/// Removes liquidity from an existing concentrated liquidity position.
/// Calculates the proportional amounts of tokens to return based on the position's
/// share of liquidity within its price range. Updates tick data and burns position
/// tokens accordingly. Includes slippage protection through minimum amount parameters.

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};
use crate::state::{TickArray, PoolError};
use crate::constant::TICK_ARRAY_SIZE;
use crate::utils::{safe_add_i128, sub_liquidity_delta, add_liquidity_delta, get_amount_0_delta, get_amount_1_delta};
use crate::logic::event::{LiquidityEvent, LiquidityEventType};

// ============================================================================
// Handler Functions
// ============================================================================

/// Remove liquidity from a concentrated liquidity position
pub fn handler(
    ctx: Context<crate::RemoveLiquidity>,
    liquidity_amount: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<(u64, u64)> {
    let pool = &mut ctx.accounts.pool.load_mut()?;
    let tick_position_key = ctx.accounts.position.key();
    let position = &mut ctx.accounts.position;
    
    // Validate liquidity amount
    require!(liquidity_amount > 0, PoolError::InvalidLiquidityAmount);
    require!(liquidity_amount <= position.liquidity, PoolError::InsufficientLiquidity);
    
    // Load tick arrays for position bounds
    let tick_array_lower = &mut ctx.accounts.tick_array_lower.load_mut()?;
    let tick_array_upper = &mut ctx.accounts.tick_array_upper.load_mut()?;
    
    // Calculate token amounts based on current price
    let (amount_0, amount_1) = calculate_token_amounts(
        liquidity_amount,
        pool.current_sqrt_price,
        position.tick_lower,
        position.tick_upper,
    )?;
    
    // Check slippage protection
    require!(amount_0 >= amount_0_min, PoolError::SlippageExceeded);
    require!(amount_1 >= amount_1_min, PoolError::SlippageExceeded);
    
    // Update position liquidity using safe arithmetic
    position.liquidity = sub_liquidity_delta(position.liquidity, liquidity_amount as i128)?;
    
    // Update tick liquidity if position is in range using safe arithmetic
    if pool.current_tick >= position.tick_lower && pool.current_tick < position.tick_upper {
        let current_liquidity = pool.liquidity;
        pool.liquidity = sub_liquidity_delta(current_liquidity, liquidity_amount as i128)?;
    }
    
    // Update tick states
    update_tick(
        tick_array_lower,
        position.tick_lower,
        -(liquidity_amount as i128),
        pool.fee_growth_global_0,
        pool.fee_growth_global_1,
        pool.tick_spacing,
    )?;
    
    update_tick(
        tick_array_upper,
        position.tick_upper,
        liquidity_amount as i128,
        pool.fee_growth_global_0,
        pool.fee_growth_global_1,
        pool.tick_spacing,
    )?;
    
    // Clear tick initialization if no liquidity remains
    let lower_tick_index = ((position.tick_lower - tick_array_lower.start_tick_index) / pool.tick_spacing as i32) as usize;
    let upper_tick_index = ((position.tick_upper - tick_array_upper.start_tick_index) / pool.tick_spacing as i32) as usize;
    
    if lower_tick_index < TICK_ARRAY_SIZE && tick_array_lower.ticks[lower_tick_index].liquidity_gross == 0 {
        // Copy bitmap to avoid packed field reference issues
        let mut bitmap = pool.tick_array_bitmap;
        clear_tick_bit(&mut bitmap, position.tick_lower)?;
        pool.tick_array_bitmap = bitmap;
    }
    if upper_tick_index < TICK_ARRAY_SIZE && tick_array_upper.ticks[upper_tick_index].liquidity_gross == 0 {
        // Copy bitmap to avoid packed field reference issues
        let mut bitmap = pool.tick_array_bitmap;
        clear_tick_bit(&mut bitmap, position.tick_upper)?;
        pool.tick_array_bitmap = bitmap;
    }
    
    // Get pool authority seeds for CPI signing
    let pool_seeds = crate::utils::CanonicalSeeds::get_pool_seeds(
        &pool.token_a_mint,
        &pool.token_b_mint,
        pool.fee_rate,
        ctx.bumps.pool,
    );
    
    // Transfer tokens from pool to user
    // Note: Transfer logic kept inline for Phase 2 Valence hook integration.
    // Liquidity operations may need complex multi-step atomic guarantees.
    if amount_0 > 0 {
        
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_0.to_account_info(),
                    to: ctx.accounts.token_account_0.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&pool_seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>()],
            ),
            amount_0,
        )?;
    }
    
    if amount_1 > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_1.to_account_info(),
                    to: ctx.accounts.token_account_1.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&pool_seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>()],
            ),
            amount_1,
        )?;
    }
    
    // Update pool statistics with consistent clock
    let clock = Clock::get()?;
    pool.last_update_slot = clock.slot;
    
    // Emit event
    emit!(LiquidityEvent {
        pool: ctx.accounts.pool.key(),
        position: tick_position_key,
        liquidity_delta: -(liquidity_amount as i128),
        amount_0,
        amount_1,
        tick_lower: position.tick_lower,
        tick_upper: position.tick_upper,
        event_type: LiquidityEventType::Remove,
        timestamp: clock.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}

/// Calculate token amounts for given liquidity
fn calculate_token_amounts(
    liquidity: u128,
    current_sqrt_price: u128,
    tick_lower: i32,
    tick_upper: i32,
) -> Result<(u64, u64)> {
    use crate::utils::TickMath;
    
    let sqrt_price_lower = TickMath::get_sqrt_ratio_at_tick(tick_lower)?;
    let sqrt_price_upper = TickMath::get_sqrt_ratio_at_tick(tick_upper)?;
    
    // Calculate amounts using round_down (false) for removing liquidity
    let (amount_0_u128, amount_1_u128) = if current_sqrt_price < sqrt_price_lower {
        // Current price below range - all in token 0
        (get_amount_0_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false)?, 0u128)
    } else if current_sqrt_price >= sqrt_price_upper {
        // Current price above range - all in token 1
        (0u128, get_amount_1_delta(sqrt_price_lower, sqrt_price_upper, liquidity, false)?)
    } else {
        // Current price in range - both tokens
        (
            get_amount_0_delta(current_sqrt_price, sqrt_price_upper, liquidity, false)?,
            get_amount_1_delta(sqrt_price_lower, current_sqrt_price, liquidity, false)?
        )
    };
    
    // Convert with overflow checking
    let amount_0 = amount_0_u128.try_into()
        .map_err(|_| PoolError::ArithmeticOverflow)?;
    let amount_1 = amount_1_u128.try_into()
        .map_err(|_| PoolError::ArithmeticOverflow)?;
    
    Ok((amount_0, amount_1))
}

/// Update tick liquidity and fee growth
fn update_tick(
    tick_array: &mut TickArray,
    tick_index: i32,
    liquidity_delta: i128,
    fee_growth_global_0: [u64; 4],
    fee_growth_global_1: [u64; 4],
    tick_spacing: i16,
) -> Result<()> {
    let array_index = ((tick_index - tick_array.start_tick_index) / tick_spacing as i32) as usize;
    require!(array_index < TICK_ARRAY_SIZE, PoolError::InvalidTickIndex);
    
    let tick = &mut tick_array.ticks[array_index];
    
    // Update liquidity using safe arithmetic
    let current_liquidity_net = tick.liquidity_net;
    tick.liquidity_net = safe_add_i128(current_liquidity_net, liquidity_delta)?;
    let current_liquidity_gross = tick.liquidity_gross;
    tick.liquidity_gross = if liquidity_delta > 0 {
        add_liquidity_delta(current_liquidity_gross, liquidity_delta)?
    } else {
        sub_liquidity_delta(current_liquidity_gross, -liquidity_delta)?
    };
    
    // Update fee growth if tick is initialized
    if tick.initialized == 1 {
        tick.fee_growth_outside_0 = fee_growth_global_0;
        tick.fee_growth_outside_1 = fee_growth_global_1;
    }
    
    Ok(())
}

/// Clear tick bit in bitmap
fn clear_tick_bit(bitmap: &mut [u64; 16], tick: i32) -> Result<()> {
    let word_pos = (tick / 64) as usize;
    let bit_pos = (tick % 64) as u8;
    
    require!(word_pos < 16, PoolError::InvalidTickIndex);
    
    bitmap[word_pos] &= !(1u64 << bit_pos);
    
    Ok(())
}
