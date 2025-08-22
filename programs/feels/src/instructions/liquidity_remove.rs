/// Removes liquidity from an existing concentrated liquidity position.
/// Calculates the proportional amounts of tokens to return based on the position's
/// share of liquidity within its price range. Updates tick data and burns position
/// tokens accordingly. Includes slippage protection through minimum amount parameters.

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};
use crate::state::{TickArray, PoolError};

// Import TICK_ARRAY_SIZE from constants
const TICK_ARRAY_SIZE: usize = 32;
use crate::utils::{SafeMath, LiquiditySafeMath};
use crate::utils::math_tick;

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
    position.liquidity = position.liquidity.safe_sub_liquidity(liquidity_amount as i128)?;
    
    // Update tick liquidity if position is in range using safe arithmetic
    if pool.current_tick >= position.tick_lower && pool.current_tick < position.tick_upper {
        pool.liquidity = pool.liquidity.safe_sub_liquidity(liquidity_amount as i128)?;
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
    
    // Get the canonical token order to derive proper seeds
    let token_a_key = pool.token_a_mint;
    let token_b_key = pool.token_b_mint;
    let pool_fee_rate = pool.fee_rate;
    
    // Transfer tokens from pool to user
    if amount_0 > 0 {
        #[allow(deprecated)]
        
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_0.to_account_info(),
                    to: ctx.accounts.token_account_0.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&[
                    b"pool",
                    token_a_key.as_ref(),
                    token_b_key.as_ref(),
                    &pool_fee_rate.to_le_bytes(),
                    &[ctx.bumps.pool],
                ]],
            ),
            amount_0,
        )?;
    }
    
    if amount_1 > 0 {
        #[allow(deprecated)]
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_vault_1.to_account_info(),
                    to: ctx.accounts.token_account_1.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[&[
                    b"pool",
                    token_a_key.as_ref(),
                    token_b_key.as_ref(),
                    &pool_fee_rate.to_le_bytes(),
                    &[ctx.bumps.pool],
                ]],
            ),
            amount_1,
        )?;
    }
    
    // Update pool statistics
    pool.last_update_slot = Clock::get()?.slot;
    
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
        timestamp: Clock::get()?.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}

/// Calculate token amounts for given liquidity
fn calculate_token_amounts(
    liquidity: u128,
    current_sqrt_price: u128,
    _tick_lower: i32,
    _tick_upper: i32,
) -> Result<(u64, u64)> {
    let sqrt_price_lower = math_tick::TickMath::get_sqrt_ratio_at_tick(_tick_lower)?;
    let sqrt_price_upper = math_tick::TickMath::get_sqrt_ratio_at_tick(_tick_upper)?;
    
    let (amount_0, amount_1) = if current_sqrt_price < sqrt_price_lower {
        // Current price below range - all in token 0
        let amount_0 = get_amount_0_delta(
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity,
            false
        )?;
        (amount_0 as u64, 0u64)
    } else if current_sqrt_price >= sqrt_price_upper {
        // Current price above range - all in token 1
        let amount_1 = get_amount_1_delta(
            sqrt_price_lower,
            sqrt_price_upper,
            liquidity,
            false
        )?;
        (0u64, amount_1 as u64)
    } else {
        // Current price in range - both tokens
        let amount_0 = get_amount_0_delta(
            current_sqrt_price,
            sqrt_price_upper,
            liquidity,
            false
        )?;
        let amount_1 = get_amount_1_delta(
            sqrt_price_lower,
            current_sqrt_price,
            liquidity,
            false
        )?;
        (amount_0 as u64, amount_1 as u64)
    };
    
    Ok((amount_0, amount_1))
}

/// Calculate amount of token 0 for given liquidity delta
fn get_amount_0_delta(
    sqrt_price_a: u128,
    sqrt_price_b: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u128> {
    let (sqrt_price_lower, sqrt_price_upper) = if sqrt_price_a < sqrt_price_b {
        (sqrt_price_a, sqrt_price_b)
    } else {
        (sqrt_price_b, sqrt_price_a)
    };
    
    let numerator = liquidity.checked_mul(sqrt_price_upper.checked_sub(sqrt_price_lower).ok_or(PoolError::MathOverflow)?).ok_or(PoolError::MathOverflow)?;
    let denominator = sqrt_price_upper.checked_mul(sqrt_price_lower).ok_or(PoolError::MathOverflow)? >> 96;
    
    if round_up {
        Ok(numerator.checked_add(denominator.checked_sub(1).ok_or(PoolError::MathOverflow)?).ok_or(PoolError::MathOverflow)?.checked_div(denominator).ok_or(PoolError::MathOverflow)?)
    } else {
        Ok(numerator.checked_div(denominator).ok_or(PoolError::MathOverflow)?)
    }
}

/// Calculate amount of token 1 for given liquidity delta
fn get_amount_1_delta(
    sqrt_price_a: u128,
    sqrt_price_b: u128,
    liquidity: u128,
    round_up: bool,
) -> Result<u128> {
    let (sqrt_price_lower, sqrt_price_upper) = if sqrt_price_a < sqrt_price_b {
        (sqrt_price_a, sqrt_price_b)
    } else {
        (sqrt_price_b, sqrt_price_a)
    };
    
    let delta = sqrt_price_upper.checked_sub(sqrt_price_lower).ok_or(PoolError::MathOverflow)?;
    if round_up {
        Ok(liquidity.checked_mul(delta).ok_or(PoolError::MathOverflow)?.checked_add((1u128 << 96).checked_sub(1).ok_or(PoolError::MathOverflow)?).ok_or(PoolError::MathOverflow)? >> 96)
    } else {
        Ok(liquidity.checked_mul(delta).ok_or(PoolError::MathOverflow)? >> 96)
    }
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
    tick.liquidity_net = tick.liquidity_net.safe_add(liquidity_delta)?;
    tick.liquidity_gross = if liquidity_delta > 0 {
        tick.liquidity_gross.safe_add_liquidity(liquidity_delta)?
    } else {
        tick.liquidity_gross.safe_sub_liquidity(-liquidity_delta)?
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

/// Liquidity event types
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum LiquidityEventType {
    Add,
    Remove,
}

/// Event emitted when liquidity changes
#[event]
pub struct LiquidityEvent {
    #[index]
    pub pool: Pubkey,
    pub position: Pubkey,
    pub liquidity_delta: i128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub event_type: LiquidityEventType,
    pub timestamp: i64,
}

impl crate::logic::EventBase for LiquidityEvent {
    fn pool(&self) -> Pubkey {
        self.pool
    }
    
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    
    fn actor(&self) -> Pubkey {
        self.position
    }
}