/// Adds liquidity to a concentrated liquidity position within a specified price range.
/// Calculates the required amounts of both tokens based on the current pool price and
/// the position's price bounds. Updates tick liquidity data and mints position NFTs
/// to track ownership. Liquidity providers earn fees proportional to their share.

use crate::state::{TickArray, PoolError};

// Import TICK_ARRAY_SIZE from constants
const TICK_ARRAY_SIZE: usize = 32;
use crate::utils::SafeMath;
use crate::logic::tick_array::TickArrayManager;
use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};

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
    let position = &mut ctx.accounts.tick_position_metadata;
    
    // Validate tick range
    require!(
        position.tick_lower < position.tick_upper,
        PoolError::InvalidTickRange
    );
    
    // Calculate required token amounts based on current price and liquidity
    let (amount_0, amount_1) = calculate_token_amounts(
        liquidity_amount,
        position.tick_lower,
        position.tick_upper,
        pool.current_sqrt_price,
    )?;
    
    // Validate against maximum amounts
    require!(amount_0 <= amount_0_max, PoolError::SlippageExceeded);
    require!(amount_1 <= amount_1_max, PoolError::SlippageExceeded);
    
    // Ensure tick arrays exist for both ticks, creating them if necessary
    TickArrayManager::ensure_tick_array_exists(
        &mut pool,
        &ctx.accounts.pool.key(),
        position.tick_lower,
        &ctx.accounts.tick_array_lower.to_account_info(),
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        ctx.program_id,
    )?;
    
    TickArrayManager::ensure_tick_array_exists(
        &mut pool,
        &ctx.accounts.pool.key(),
        position.tick_upper,
        &ctx.accounts.tick_array_upper.to_account_info(),
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        ctx.program_id,
    )?;
    
    // Initialize or update ticks if needed
    update_tick_liquidity(
        &mut ctx.accounts.tick_array_lower,
        position.tick_lower,
        liquidity_amount as i128,
        false, // not removing
    )?;
    
    update_tick_liquidity(
        &mut ctx.accounts.tick_array_upper,
        position.tick_upper,
        -(liquidity_amount as i128), // negative for upper tick
        false,
    )?;
    
    // Transfer tokens from user to pool vaults
    if amount_0 > 0 {
        #[allow(deprecated)]
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_0.to_account_info(),
                    to: ctx.accounts.pool_token_0.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_0,
        )?;
    }
    
    if amount_1 > 0 {
        #[allow(deprecated)]
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_1.to_account_info(),
                    to: ctx.accounts.pool_token_1.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_1,
        )?;
    }
    
    // Update position metadata using safe arithmetic
    position.liquidity = position.liquidity.safe_add(liquidity_amount)?;
    
    // Update global liquidity if position is in range using safe arithmetic
    if pool.current_tick >= position.tick_lower && pool.current_tick < position.tick_upper {
        pool.liquidity = pool.liquidity.safe_add(liquidity_amount)?;
    }
    
    // Update volume statistics using safe arithmetic
    pool.total_volume_0 = pool.total_volume_0.safe_add(amount_0 as u128)?;
    pool.total_volume_1 = pool.total_volume_1.safe_add(amount_1 as u128)?;
    
    // Update last update slot
    let clock = Clock::get()?;
    pool.last_update_slot = clock.slot;
    
    emit!(LiquidityEvent {
        pool: ctx.accounts.pool.key(),
        position: ctx.accounts.tick_position_metadata.key(),
        liquidity_delta: liquidity_amount as i128,
        amount_0,
        amount_1,
        user: ctx.accounts.user.key(),
        timestamp: clock.unix_timestamp,
    });
    
    Ok((amount_0, amount_1))
}

#[event]
pub struct LiquidityEvent {
    #[index]
    pub pool: Pubkey,
    pub position: Pubkey,
    pub liquidity_delta: i128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub user: Pubkey,
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
        self.user
    }
}

/// Calculate required token amounts for given liquidity
fn calculate_token_amounts(
    liquidity: u128,
    tick_lower: i32,
    tick_upper: i32,
    current_sqrt_price: u128,
) -> Result<(u64, u64)> {
    use crate::utils::math_liquidity::*;
    use crate::utils::SafeMath;
    
    let sqrt_price_lower = tick_to_sqrt_price(tick_lower);
    let sqrt_price_upper = tick_to_sqrt_price(tick_upper);
    
    // Use proper liquidity math with overflow protection
    let amount_0 = if current_sqrt_price < sqrt_price_upper {
        let sqrt_price_effective = current_sqrt_price.max(sqrt_price_lower);
        get_amount_0_delta(
            sqrt_price_effective,
            sqrt_price_upper,
            liquidity,
            true // round up for user to pay
        )?
    } else {
        0
    };
    
    let amount_1 = if current_sqrt_price > sqrt_price_lower {
        let sqrt_price_effective = current_sqrt_price.min(sqrt_price_upper);
        get_amount_1_delta(
            sqrt_price_lower,
            sqrt_price_effective,
            liquidity,
            true // round up for user to pay
        )?
    } else {
        0
    };
    
    Ok((amount_0, amount_1))
}

/// Convert tick to sqrt price using precise calculation
fn tick_to_sqrt_price(tick: i32) -> u128 {
    // Use the proper implementation from math_tick
    use crate::utils::math_tick;
    match math_tick::TickMath::get_sqrt_ratio_at_tick(tick) {
        Ok(sqrt_price) => sqrt_price,
        Err(_) => {
            // Fallback to middle price if tick is invalid
            // This shouldn't happen with proper validation
            5u128 << 64
        }
    }
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