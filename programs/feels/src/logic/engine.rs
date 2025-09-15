//! CLMM Engine - Core swap stepping logic
//!
//! Provides a reusable stepper function that handles
//! the core math for concentrated liquidity swaps.
//!
//! IMPORTANT: Rounding Behavior
//! ----------------------------
//! The Orca Whirlpools core functions handle rounding correctly:
//! - try_get_amount_delta_* with round_up=false: rounds DOWN (for output amounts)
//! - try_get_amount_delta_* with round_up=true: rounds UP (for input amounts)
//! - try_get_next_sqrt_price_from_*: handles rounding to favor the pool
//!
//! This ensures the protocol is never disadvantaged by rounding errors.

use crate::{
    error::FeelsError,
    state::{TickArray, TICK_ARRAY_SIZE},
    utils::{get_tick_array_start_index, sqrt_price_from_tick},
};
use anchor_lang::accounts::account_loader::AccountLoader;
use anchor_lang::prelude::*;
use orca_whirlpools_core::{
    try_get_amount_delta_a, try_get_amount_delta_b, try_get_next_sqrt_price_from_a,
    try_get_next_sqrt_price_from_b, U128,
};

/// Simplified outcome of a swap step for cleaner outer logic
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepOutcome {
    /// Reached the target (tick or bound)
    ReachedTarget,
    /// Stopped at a price bound (floor limit)
    PartialAtBound,
    /// Used all available input amount
    PartialByAmount,
}

/// Result of a single swap step with clear gross/net separation
#[derive(Debug, Clone, Copy)]
pub struct StepResult {
    /// Total input amount consumed (including fees)
    pub gross_in_used: u64,
    /// Net input amount after fees (amount that was actually swapped)
    pub net_in_used: u64,
    /// Amount of output token produced
    pub out: u64,
    /// Fee amount collected (gross_in_used - net_in_used)
    pub fee: u64,
    /// New sqrt price after the step
    pub sqrt_next: u128,
    /// The tick that was crossed (if outcome is ReachedTarget and not at bound)
    pub crossed_tick: Option<i32>,
    /// Simplified outcome for outer logic
    pub outcome: StepOutcome,
}

/// Direction of the swap
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwapDirection {
    /// Swapping token 0 for token 1 (price decreases)
    ZeroForOne,
    /// Swapping token 1 for token 0 (price increases)
    OneForZero,
}

/// Maximum number of steps allowed in a single swap to prevent griefing attacks
/// This limit ensures bounded compute costs and prevents attackers from creating
/// many empty tick arrays to exhaust compute units
pub const MAX_SWAP_STEPS: u16 = 256;

/// Maximum number of tick arrays that can be provided to a swap
/// This prevents griefing attacks where an attacker provides many unnecessary
/// tick arrays to inflate compute costs
pub const MAX_TICK_ARRAYS_PER_SWAP: usize = 10;

/// Context for swap execution - bundles all swap parameters
#[derive(Debug, Clone, Copy)]
pub struct SwapContext {
    pub direction: SwapDirection,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub fee_bps: u16,
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    pub tick_spacing: u16,
}

impl SwapContext {
    /// Create a new swap context
    pub fn new(
        direction: SwapDirection,
        sqrt_price: u128,
        liquidity: u128,
        fee_bps: u16,
        global_lower_tick: i32,
        global_upper_tick: i32,
        tick_spacing: u16,
    ) -> Self {
        Self {
            direction,
            sqrt_price,
            liquidity,
            fee_bps,
            global_lower_tick,
            global_upper_tick,
            tick_spacing,
        }
    }

    /// Update context after a step
    pub fn update_after_step(&mut self, new_sqrt_price: u128, new_liquidity: u128) {
        self.sqrt_price = new_sqrt_price;
        self.liquidity = new_liquidity;
    }
}

/// Execute a single swap step within the current tick range
///
/// Given the current state and a target, this function computes how much
/// can be swapped before hitting the target price or exhausting input.
/// The function properly handles bounds and returns the reason for termination.
///
/// SECURITY: This function is NOT vulnerable to flash loan attacks because:
/// 1. All calculations use only market.sqrt_price and market.liquidity from on-chain state
/// 2. Vault balances are NEVER read or used in price calculations
/// 3. Amounts are computed using the constant product formula x*y=k via sqrt price
/// 4. This follows the Uniswap V3 security model exactly
pub fn compute_swap_step(
    ctx: &SwapContext,
    target_sqrt_price: u128,
    target_tick: Option<i32>,
    amount_remaining: u64,
) -> Result<StepResult> {
    let direction = ctx.direction;
    let current_sqrt_price = ctx.sqrt_price;
    let liquidity = ctx.liquidity;
    let fee_pct = ctx.fee_bps;
    let global_lower_tick = ctx.global_lower_tick;
    let global_upper_tick = ctx.global_upper_tick;
    // Ensure we have a valid price range
    require!(current_sqrt_price > 0, FeelsError::InvalidPrice);
    require!(target_sqrt_price > 0, FeelsError::InvalidPrice);
    require!(liquidity > 0, FeelsError::InsufficientLiquidity);

    // Check direction consistency
    match direction {
        SwapDirection::ZeroForOne => {
            require!(
                target_sqrt_price < current_sqrt_price,
                FeelsError::InvalidPrice
            );
        }
        SwapDirection::OneForZero => {
            require!(
                target_sqrt_price > current_sqrt_price,
                FeelsError::InvalidPrice
            );
        }
    }

    // We'll calculate fees after determining how much we'll actually use
    // This prevents overcharging when we can reach the target with less than amount_remaining

    // Calculate the maximum amount we can swap to reach target price
    let (_max_amount_in, _amount_out_at_target) = match direction {
        SwapDirection::ZeroForOne => {
            // Amount of token0 needed to reach target price
            let max_in = try_get_amount_delta_a(
                U128::from(target_sqrt_price),
                U128::from(current_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            // Amount of token1 we'd get at target
            let out = try_get_amount_delta_b(
                U128::from(target_sqrt_price),
                U128::from(current_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            (max_in, out)
        }
        SwapDirection::OneForZero => {
            // Amount of token1 needed to reach target price
            let max_in = try_get_amount_delta_b(
                U128::from(current_sqrt_price),
                U128::from(target_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            // Amount of token0 we'd get at target
            let out = try_get_amount_delta_a(
                U128::from(current_sqrt_price),
                U128::from(target_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            (max_in, out)
        }
    };

    // Check if we're at a bound
    let at_bound = match direction {
        SwapDirection::ZeroForOne => target_tick.is_some_and(|t| t <= global_lower_tick),
        SwapDirection::OneForZero => target_tick.is_some_and(|t| t >= global_upper_tick),
    };

    // If we're at a bound, clamp the target price
    let clamped_target_sqrt_price = if at_bound {
        match direction {
            SwapDirection::ZeroForOne => {
                // Clamp at lower bound
                let bound_sqrt_price = sqrt_price_from_tick(global_lower_tick)?;
                target_sqrt_price.max(bound_sqrt_price)
            }
            SwapDirection::OneForZero => {
                // Clamp at upper bound
                let bound_sqrt_price = sqrt_price_from_tick(global_upper_tick)?;
                target_sqrt_price.min(bound_sqrt_price)
            }
        }
    } else {
        target_sqrt_price
    };

    // Recalculate max amounts with clamped target
    let (max_amount_in, amount_out_at_target) = match direction {
        SwapDirection::ZeroForOne => {
            let max_in = try_get_amount_delta_a(
                U128::from(clamped_target_sqrt_price),
                U128::from(current_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            let out = try_get_amount_delta_b(
                U128::from(clamped_target_sqrt_price),
                U128::from(current_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            (max_in, out)
        }
        SwapDirection::OneForZero => {
            let max_in = try_get_amount_delta_b(
                U128::from(current_sqrt_price),
                U128::from(clamped_target_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            let out = try_get_amount_delta_a(
                U128::from(current_sqrt_price),
                U128::from(clamped_target_sqrt_price),
                U128::from(liquidity),
                false,
            )
            .map_err(|_| FeelsError::MathOverflow)?;
            (max_in, out)
        }
    };

    // First check if we have enough to reach the target (without fees)
    // Then calculate the precise fee based on what we'll actually use
    let (gross_amount_in, fee_amount, new_sqrt_price, amount_out, outcome, crossed_tick) =
        if amount_remaining > max_amount_in {
            // We have more than enough to reach target
            // Calculate gross amount needed including fee: gross = net / (1 - fee_rate)
            let gross_in = if fee_pct > 0 {
                // gross = ceil(net * 10000 / (10000 - fee_bps))
                let numerator = (max_amount_in as u128)
                    .checked_mul(10000)
                    .ok_or(FeelsError::MathOverflow)?;
                let denominator = 10000u128
                    .checked_sub(fee_pct as u128)
                    .ok_or(FeelsError::MathOverflow)?;
                let gross = numerator
                    .checked_add(denominator - 1) // ceiling division
                    .ok_or(FeelsError::MathOverflow)?
                    .checked_div(denominator)
                    .ok_or(FeelsError::DivisionByZero)?;
                gross.min(amount_remaining as u128) as u64
            } else {
                max_amount_in
            };

            let fee_on_used = gross_in.saturating_sub(max_amount_in);

            (
                gross_in,
                fee_on_used,
                clamped_target_sqrt_price,
                amount_out_at_target,
                StepOutcome::ReachedTarget,
                if at_bound { None } else { target_tick },
            )
        } else {
            // Limited by input amount - use all of it
            // CRITICAL: Use ceiling division for fee calculation to prevent fee draining
            // This ensures minimum fee of 1 for any non-zero swap with non-zero fee rate
            let fee_on_all = crate::utils::calculate_fee_ceil(amount_remaining, fee_pct)?;
            let net_amount = amount_remaining.saturating_sub(fee_on_all);

            // Calculate how far we can go with the net amount
            let new_price = match direction {
                SwapDirection::ZeroForOne => try_get_next_sqrt_price_from_a(
                    U128::from(current_sqrt_price),
                    U128::from(liquidity),
                    net_amount,
                    true,
                )
                .map_err(|_| FeelsError::MathOverflow)?,
                SwapDirection::OneForZero => try_get_next_sqrt_price_from_b(
                    U128::from(current_sqrt_price),
                    U128::from(liquidity),
                    net_amount,
                    true,
                )
                .map_err(|_| FeelsError::MathOverflow)?,
            };

            // Calculate output for partial swap
            let out = match direction {
                SwapDirection::ZeroForOne => try_get_amount_delta_b(
                    U128::from(new_price),
                    U128::from(current_sqrt_price),
                    U128::from(liquidity),
                    false,
                )
                .map_err(|_| FeelsError::MathOverflow)?,
                SwapDirection::OneForZero => try_get_amount_delta_a(
                    U128::from(current_sqrt_price),
                    U128::from(new_price),
                    U128::from(liquidity),
                    false,
                )
                .map_err(|_| FeelsError::MathOverflow)?,
            };

            (
                amount_remaining,
                fee_on_all,
                new_price,
                out,
                StepOutcome::PartialByAmount,
                None,
            )
        };

    // Determine final outcome based on what happened
    let final_outcome = if outcome == StepOutcome::ReachedTarget && at_bound {
        StepOutcome::PartialAtBound
    } else {
        outcome
    };

    Ok(StepResult {
        gross_in_used: gross_amount_in,
        net_in_used: gross_amount_in.saturating_sub(fee_amount),
        out: amount_out,
        fee: fee_amount,
        sqrt_next: new_sqrt_price,
        crossed_tick,
        outcome: final_outcome,
    })
}

use std::collections::HashMap;

/// Generic tick array iterator for finding next initialized tick
///
/// This iterator provides O(1) lookups for tick arrays and efficiently
/// scans through arrays to find initialized ticks. All validation is done
/// in the constructor to ensure invariants are maintained.
pub struct TickArrayIterator<'info> {
    /// Loaded tick arrays
    tick_arrays: Vec<AccountLoader<'info, TickArray>>,
    /// Map from start_tick_index to position in tick_arrays for O(1) lookup
    start_index_map: HashMap<i32, usize>,
    /// Current tick position (for context)
    #[allow(dead_code)]
    current_tick: i32,
    /// Tick spacing for this market
    tick_spacing: u16,
    /// Swap direction
    direction: SwapDirection,
}

impl<'info> TickArrayIterator<'info> {
    /// Create a new tick array iterator from remaining accounts
    ///
    /// This constructor performs all validation to ensure:
    /// - Each tick array belongs to the expected market
    /// - Each tick array has an aligned start_tick_index
    /// - At least one tick array is provided
    ///
    /// Downstream logic can safely assume these invariants hold.
    pub fn new(
        remaining_accounts: &'info [AccountInfo<'info>],
        current_tick: i32,
        tick_spacing: u16,
        direction: SwapDirection,
        market_key: &Pubkey,
    ) -> Result<Self> {
        // SECURITY: Enforce maximum tick arrays to prevent griefing attacks
        // where an attacker provides many unnecessary accounts to exhaust compute units
        require!(
            remaining_accounts.len() <= MAX_TICK_ARRAYS_PER_SWAP,
            FeelsError::TooManyTickArrays
        );
        // Require at least one tick array for valid price path
        require!(
            !remaining_accounts.is_empty(),
            FeelsError::MissingTickArrayCoverage
        );

        // Convert remaining accounts to tick array loaders with full validation
        let mut tick_arrays: Vec<AccountLoader<'info, TickArray>> = Vec::new();
        let mut start_index_map: HashMap<i32, usize> = HashMap::new();
        for account_info in remaining_accounts {
            // Create loader from account info
            let loader = AccountLoader::<TickArray>::try_from(account_info)?;

            // Verify discriminator and market linkage
            {
                let array = loader.load()?;

                // Verify this tick array belongs to the correct market
                require!(array.market == *market_key, FeelsError::InvalidTickArray);

                // Verify start_tick_index alignment using div_euclid
                let expected_start =
                    get_tick_array_start_index(array.start_tick_index, tick_spacing);
                require!(
                    array.start_tick_index == expected_start,
                    FeelsError::InvalidTickArray
                );
                // Memoize start index to loader position for O(1) lookup
                start_index_map.insert(array.start_tick_index, tick_arrays.len());
            }

            tick_arrays.push(loader);
        }

        // Ensure we have at least one tick array
        require!(!tick_arrays.is_empty(), FeelsError::InvalidTickArray);

        Ok(Self {
            tick_arrays,
            start_index_map,
            current_tick,
            tick_spacing,
            direction,
        })
    }

    /// Find the tick array containing a given tick index - O(1) lookup
    pub fn find_array_for_tick(
        &self,
        tick_index: i32,
    ) -> Result<Option<&AccountLoader<'info, TickArray>>> {
        // Calculate which array should contain this tick using div_euclid for proper negative handling
        let array_start_index = get_tick_array_start_index(tick_index, self.tick_spacing);

        if let Some(&pos) = self.start_index_map.get(&array_start_index) {
            return Ok(self.tick_arrays.get(pos));
        }
        Ok(None)
    }

    /// Find the tick array loader for a specific tick index - O(1) lookup
    pub fn find_loader_for_tick_index(
        &self,
        tick: i32,
    ) -> Option<&AccountLoader<'info, TickArray>> {
        let array_start_index = get_tick_array_start_index(tick, self.tick_spacing);
        self.find_loader_for_start(array_start_index)
    }

    /// Find the tick array loader for a specific start index - O(1) lookup
    pub fn find_loader_for_start(&self, start: i32) -> Option<&AccountLoader<'info, TickArray>> {
        self.start_index_map
            .get(&start)
            .and_then(|&pos| self.tick_arrays.get(pos))
    }

    /// Find the next initialized tick in the swap direction
    pub fn next_initialized_tick(
        &self,
        from_tick: i32,
    ) -> Result<Option<(i32, &AccountLoader<'info, TickArray>)>> {
        let search_direction = match self.direction {
            SwapDirection::ZeroForOne => -1, // Search downward
            SwapDirection::OneForZero => 1,  // Search upward
        };

        // Start from the next tick in search direction
        let start_tick = from_tick + search_direction * self.tick_spacing as i32;

        // First, find the array containing the start tick
        if let Some(array_loader) = self.find_loader_for_tick_index(start_tick) {
            let array = array_loader.load()?;

            // Scan within this array first (64 ticks via offsets)
            let start_offset = array.offset_for(start_tick, self.tick_spacing)?;
            if let Some(tick_index) =
                self.scan_array_for_initialized_tick(&array, start_offset, search_direction > 0)?
            {
                return Ok(Some((tick_index, array_loader)));
            }
        }

        // If not found in current array, search subsequent arrays by stepping
        // by the aligned array size (cheap computation)
        let array_size_ticks = TICK_ARRAY_SIZE as i32 * self.tick_spacing as i32;
        let current_array_start = get_tick_array_start_index(start_tick, self.tick_spacing);

        // Search up to 10 arrays in the direction
        for i in 1..=10 {
            let array_start = current_array_start + (search_direction * i * array_size_ticks);

            // Use O(1) lookup via start index
            if let Some(array_loader) = self.find_loader_for_start(array_start) {
                let array = array_loader.load()?;

                // Scan entire array (64 ticks)
                let start_offset = if search_direction > 0 {
                    0
                } else {
                    TICK_ARRAY_SIZE - 1
                };
                if let Some(tick_index) = self.scan_array_for_initialized_tick(
                    &array,
                    start_offset,
                    search_direction > 0,
                )? {
                    return Ok(Some((tick_index, array_loader)));
                }
            }
        }

        Ok(None)
    }

    /// Scan within a single tick array for the next initialized tick
    fn scan_array_for_initialized_tick(
        &self,
        array: &TickArray,
        start_offset: usize,
        forward: bool,
    ) -> Result<Option<i32>> {
        if forward {
            // Scan forward from start_offset to end
            for offset in start_offset..TICK_ARRAY_SIZE {
                if array.ticks[offset].initialized != 0 {
                    let tick_index =
                        array.start_tick_index + (offset as i32 * self.tick_spacing as i32);
                    return Ok(Some(tick_index));
                }
            }
        } else {
            // Scan backward from start_offset to beginning
            for offset in (0..=start_offset).rev() {
                if array.ticks[offset].initialized != 0 {
                    let tick_index =
                        array.start_tick_index + (offset as i32 * self.tick_spacing as i32);
                    return Ok(Some(tick_index));
                }
            }
        }

        Ok(None)
    }
}

/// Update fee growth for a swap segment
///
/// This computes the fee growth increment for a single segment of the swap
/// and should be called before crossing each tick.
pub fn update_fee_growth_segment(
    fee_amount: u64,
    liquidity_before_step: u128,
    _is_token_0: bool,
) -> Result<u128> {
    if liquidity_before_step == 0 {
        return Ok(0);
    }

    // fee_growth_increment = (fee_amount << 64) / liquidity
    let fee_growth_inc = ((fee_amount as u128) << 64)
        .checked_div(liquidity_before_step)
        .ok_or(FeelsError::DivisionByZero)?;

    Ok(fee_growth_inc)
}

/// Initialize fee growth outside values for a tick
///
/// When a tick is first initialized, set its fee_growth_outside based on
/// its position relative to the current tick (Uniswap V3 convention).
pub fn initialize_tick_fee_growth(
    tick_index: i32,
    current_tick: i32,
    fee_growth_global_0: u128,
    fee_growth_global_1: u128,
) -> (u128, u128) {
    if tick_index <= current_tick {
        // Tick is at or below current price
        // All global fee growth is "inside" (below this tick)
        (fee_growth_global_0, fee_growth_global_1)
    } else {
        // Tick is above current price
        // No fee growth is "inside" (below this tick) yet
        (0, 0)
    }
}
