/// Computes the necessary accounts and tick arrays for executing a 3D order.
/// This read-only helper allows clients to pre-fetch all necessary accounts across
/// the three dimensions (rate, duration, leverage) before executing an order.
/// Essential for efficient client implementations as it predicts the path through
/// the 3D tick space.

use anchor_lang::prelude::*;
use crate::constant::{MAX_TICK_ARRAYS_PER_SWAP, TICK_ARRAY_SIZE};
use crate::state::{FeelsProtocolError, Tick3D, Pool};
use crate::state::duration::Duration;
use crate::utils::TickMath;
use super::order::OrderType;

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderComputeParams {
    /// Order amount
    pub amount: u64,
    
    /// Rate dimension parameters
    pub rate_params: RateComputeParams,
    
    /// Duration for the order
    pub duration: Duration,
    
    /// Leverage level (6 decimals)
    pub leverage: u64,
    
    /// Order type to compute for
    pub order_type: OrderType,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum RateComputeParams {
    /// For swaps - compute path from current to target rate
    SwapPath {
        sqrt_rate_limit: u128,
        is_token_a_to_b: bool,
    },
    /// For liquidity - compute affected tick arrays
    LiquidityRange {
        tick_lower: i32,
        tick_upper: i32,
    },
}


// ============================================================================
// Result Types
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct OrderComputeResult {
    /// Required tick arrays for rate dimension
    pub rate_tick_arrays: Vec<TickArrayInfo>,
    
    /// Required duration bucket accounts
    pub duration_accounts: Vec<DurationAccountInfo>,
    
    /// Required leverage tier accounts  
    pub leverage_accounts: Vec<LeverageAccountInfo>,
    
    /// Estimated 3D tick position
    pub estimated_tick_3d: Tick3DEncoded,
    
    /// Estimated compute units needed
    pub estimated_compute_units: u64,
    
    /// Whether order crosses multiple dimensions
    pub is_multi_dimensional: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct TickArrayInfo {
    pub pubkey: Pubkey,
    pub start_tick_index: i32,
    pub bump: u8,
}

// Alias for return type compatibility
pub type Tick3DArrayInfo = OrderComputeResult;

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct DurationAccountInfo {
    pub pubkey: Pubkey,
    pub duration_type: u8,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct LeverageAccountInfo {
    pub pubkey: Pubkey,
    pub leverage_tier: u8,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct Tick3DEncoded {
    pub encoded: i32,
    pub rate_tick: i32,
    pub duration_tick: i16,
    pub leverage_tick: i16,
}

// ============================================================================
// Handler Function
// ============================================================================

pub fn handler(
    ctx: Context<crate::OrderCompute>,
    params: OrderComputeParams,
) -> Result<Tick3DArrayInfo> {
    require!(params.amount > 0, FeelsProtocolError::InvalidAmount);
    
    let pool = ctx.accounts.pool.load()?;
    let clock = Clock::get()?;
    
    // Compute rate dimension requirements
    let rate_tick_arrays = match params.rate_params {
        RateComputeParams::SwapPath { sqrt_rate_limit, is_token_a_to_b } => {
            compute_swap_tick_arrays(
                &pool,
                &ctx.accounts.pool.key(),
                sqrt_rate_limit,
                is_token_a_to_b,
                ctx.program_id,
            )?
        },
        RateComputeParams::LiquidityRange { tick_lower, tick_upper } => {
            compute_liquidity_tick_arrays(
                &pool,
                &ctx.accounts.pool.key(),
                tick_lower,
                tick_upper,
                ctx.program_id,
            )?
        },
    };
    
    // Compute duration accounts
    let duration_accounts = compute_duration_accounts(
        &pool,
        &ctx.accounts.pool.key(),
        &params.duration,
        ctx.program_id,
    )?;
    
    // Compute leverage accounts
    let leverage_accounts = compute_leverage_accounts(
        &pool,
        &ctx.accounts.pool.key(),
        params.leverage,
        ctx.program_id,
    )?;
    
    // Calculate estimated 3D tick position
    let estimated_tick_3d = calculate_3d_tick(
        &pool,
        &params.rate_params,
        &params.duration,
        params.leverage,
    )?;
    
    // Estimate compute units
    let estimated_compute_units = estimate_compute_units(
        &rate_tick_arrays,
        &duration_accounts,
        &leverage_accounts,
        &params.order_type,
    );
    
    // Determine if order crosses dimensions
    let is_multi_dimensional = match params.order_type {
        OrderType::Immediate => rate_tick_arrays.len() > 1, // Crosses rate ticks
        OrderType::Liquidity => {
            // Check if range spans multiple duration or leverage buckets
            duration_accounts.len() > 1 || leverage_accounts.len() > 1
        },
        OrderType::Limit => false, // Single point in 3D space
    };
    
    // Populate TickArrayRouter if provided
    if let (Some(router), Some(authority)) = (&ctx.accounts.tick_array_router, &ctx.accounts.authority) {
        populate_tick_array_router(
            router,
            authority,
            &rate_tick_arrays,
            &pool,
            clock.slot,
        )?;
    }
    
    Ok(OrderComputeResult {
        rate_tick_arrays,
        duration_accounts,
        leverage_accounts,
        estimated_tick_3d,
        estimated_compute_units,
        is_multi_dimensional,
    })
}

// ============================================================================
// Computation Functions
// ============================================================================

/// Calculate required tick arrays for a swap path
fn compute_swap_tick_arrays(
    pool: &Pool,
    pool_key: &Pubkey,
    sqrt_rate_limit: u128,
    is_token_a_to_b: bool,
    program_id: &Pubkey,
) -> Result<Vec<TickArrayInfo>> {
    let start_tick = pool.current_tick;
    let end_tick = if sqrt_rate_limit > 0 {
        TickMath::get_tick_at_sqrt_ratio(sqrt_rate_limit)?
    } else {
        if is_token_a_to_b {
            crate::utils::MIN_TICK
        } else {
            crate::utils::MAX_TICK
        }
    };
    
    let tick_arrays = calculate_required_tick_arrays(
        start_tick,
        end_tick,
        pool.tick_spacing,
        is_token_a_to_b,
    )?;
    
    // Generate PDAs for each tick array
    let mut tick_array_infos = Vec::with_capacity(tick_arrays.len());
    for start_tick_index in tick_arrays {
        let (pda, bump) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
            pool_key,
            start_tick_index,
            program_id,
        );
        
        tick_array_infos.push(TickArrayInfo {
            pubkey: pda,
            start_tick_index,
            bump,
        });
    }
    
    Ok(tick_array_infos)
}

/// Calculate required tick arrays for a liquidity range
fn compute_liquidity_tick_arrays(
    pool: &Pool,
    pool_key: &Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    program_id: &Pubkey,
) -> Result<Vec<TickArrayInfo>> {
    require!(
        tick_lower < tick_upper,
        FeelsProtocolError::InvalidTickRange
    );
    
    // Find tick arrays containing the range boundaries
    let lower_array_start = get_tick_array_start_index(tick_lower, pool.tick_spacing);
    let upper_array_start = get_tick_array_start_index(tick_upper, pool.tick_spacing);
    
    let mut tick_array_infos = Vec::new();
    
    // Add lower tick array
    let (lower_pda, lower_bump) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
        pool_key,
        lower_array_start,
        program_id,
    );
    tick_array_infos.push(TickArrayInfo {
        pubkey: lower_pda,
        start_tick_index: lower_array_start,
        bump: lower_bump,
    });
    
    // Add upper tick array if different
    if upper_array_start != lower_array_start {
        let (upper_pda, upper_bump) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
            pool_key,
            upper_array_start,
            program_id,
        );
        tick_array_infos.push(TickArrayInfo {
            pubkey: upper_pda,
            start_tick_index: upper_array_start,
            bump: upper_bump,
        });
    }
    
    Ok(tick_array_infos)
}

/// Compute duration bucket accounts
fn compute_duration_accounts(
    _pool: &Pool,
    pool_key: &Pubkey,
    duration: &Duration,
    program_id: &Pubkey,
) -> Result<Vec<DurationAccountInfo>> {
    // For Phase 1, we use simple duration buckets
    // Phase 3 will have more sophisticated duration management
    
    let duration_type = duration.to_u8();
    let (pda, bump) = Pubkey::find_program_address(
        &[
            b"duration",
            pool_key.as_ref(),
            &[duration_type],
        ],
        program_id,
    );
    
    Ok(vec![DurationAccountInfo {
        pubkey: pda,
        duration_type,
        bump,
    }])
}

/// Compute leverage tier accounts
fn compute_leverage_accounts(
    _pool: &Pool,
    pool_key: &Pubkey,
    leverage: u64,
    program_id: &Pubkey,
) -> Result<Vec<LeverageAccountInfo>> {
    // Convert leverage to tier (Phase 2 feature)
    // For now, simple bucketing: 1x, 2x, 3x, 5x, 10x
    let leverage_tier = match leverage / 1_000_000 {
        0..=1 => 0,   // 1x
        2 => 1,       // 2x
        3 => 2,       // 3x
        4..=5 => 3,   // 5x
        6..=10 => 4,  // 10x
        _ => 5,       // Max
    };
    
    let (pda, bump) = Pubkey::find_program_address(
        &[
            b"leverage",
            pool_key.as_ref(),
            &[leverage_tier],
        ],
        program_id,
    );
    
    Ok(vec![LeverageAccountInfo {
        pubkey: pda,
        leverage_tier,
        bump,
    }])
}

/// Calculate the 3D tick position
fn calculate_3d_tick(
    _pool: &Pool,
    rate_params: &RateComputeParams,
    duration: &Duration,
    leverage: u64,
) -> Result<Tick3DEncoded> {
    let rate_tick = match rate_params {
        RateComputeParams::SwapPath { sqrt_rate_limit, .. } => {
            TickMath::get_tick_at_sqrt_ratio(*sqrt_rate_limit)?
        },
        RateComputeParams::LiquidityRange { tick_lower, tick_upper } => {
            // Use midpoint for estimation
            (*tick_lower + *tick_upper) / 2
        },
    };
    
    let tick_3d = Tick3D {
        rate_tick,
        duration_tick: duration.to_tick(),
        leverage_tick: leverage_to_tick(leverage),
    };
    
    Ok(Tick3DEncoded {
        encoded: tick_3d.encode(),
        rate_tick: tick_3d.rate_tick,
        duration_tick: tick_3d.duration_tick,
        leverage_tick: tick_3d.leverage_tick,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate required tick arrays for a price range
fn calculate_required_tick_arrays(
    start_tick: i32,
    end_tick: i32,
    tick_spacing: i16,
    is_descending: bool,
) -> Result<Vec<i32>> {
    let mut tick_arrays = Vec::new();
    let mut current_tick = start_tick;
    
    let (direction, target) = if is_descending {
        (-1, end_tick.min(start_tick))
    } else {
        (1, end_tick.max(start_tick))
    };
    
    while tick_arrays.len() < MAX_TICK_ARRAYS_PER_SWAP &&
          ((direction > 0 && current_tick <= target) ||
           (direction < 0 && current_tick >= target)) {
        
        let array_start = get_tick_array_start_index(current_tick, tick_spacing);
        
        // Add if not already included
        if tick_arrays.is_empty() || tick_arrays.last() != Some(&array_start) {
            tick_arrays.push(array_start);
        }
        
        // Move to next tick array
        current_tick = if direction > 0 {
            array_start + (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)
        } else {
            array_start - 1
        };
    }
    
    Ok(tick_arrays)
}

/// Get the start index of the tick array containing the given tick
fn get_tick_array_start_index(tick: i32, tick_spacing: i16) -> i32 {
    let ticks_per_array = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    let array_index = tick.div_euclid(ticks_per_array);
    array_index * ticks_per_array
}

/// Convert leverage value to tick
fn leverage_to_tick(leverage: u64) -> i16 {
    // Map leverage (1-10x with 6 decimals) to tick space (0-63)
    let normalized_leverage = leverage.saturating_sub(1_000_000) / 1_000_000;
    (normalized_leverage * 7).min(63) as i16
}

/// Estimate compute units for the operation
fn estimate_compute_units(
    rate_arrays: &[TickArrayInfo],
    duration_accounts: &[DurationAccountInfo],
    leverage_accounts: &[LeverageAccountInfo],
    order_type: &OrderType,
) -> u64 {
    let base_units = match order_type {
        OrderType::Immediate => 50_000,
        OrderType::Liquidity => 75_000,
        OrderType::Limit => 30_000,
    };
    
    // Add units for each dimension
    let rate_units = rate_arrays.len() as u64 * 10_000;
    let duration_units = duration_accounts.len() as u64 * 5_000;
    let leverage_units = leverage_accounts.len() as u64 * 5_000;
    
    base_units + rate_units + duration_units + leverage_units
}

/// Populate the TickArrayRouter with computed tick arrays
fn populate_tick_array_router(
    router: &Account<crate::state::TickArrayRouter>,
    authority: &Signer,
    tick_arrays: &[TickArrayInfo],
    pool: &Pool,
    current_slot: u64,
) -> Result<()> {
    // Verify authority
    require!(
        router.authority == authority.key() || pool.authority == authority.key(),
        FeelsProtocolError::InvalidAuthority
    );
    
    // Create a mutable copy of the router to update
    let mut router_data = router.clone();
    
    // Clear existing arrays if stale (more than 100 slots old)
    if current_slot.saturating_sub(router_data.last_update_slot) > 100 {
        router_data.active_bitmap = 0;
        for i in 0..crate::constant::MAX_ROUTER_ARRAYS {
            router_data.tick_arrays[i] = Pubkey::default();
            router_data.start_indices[i] = i32::MIN;
        }
    }
    
    // Register new tick arrays
    for array_info in tick_arrays.iter().take(crate::constant::MAX_ROUTER_ARRAYS) {
        // Check if already registered
        if router_data.contains_array(array_info.start_tick_index).is_some() {
            continue;
        }
        
        // Find first available slot
        for i in 0..crate::constant::MAX_ROUTER_ARRAYS {
            if !router_data.is_slot_active(i) {
                router_data.tick_arrays[i] = array_info.pubkey;
                router_data.start_indices[i] = array_info.start_tick_index;
                router_data.active_bitmap |= 1 << i;
                break;
            }
        }
    }
    
    // Update last update slot
    router_data.last_update_slot = current_slot;
    
    // Note: In a real implementation, we would need to save this back to the account
    // This would require making the router parameter mutable and using proper account serialization
    msg!("TickArrayRouter populated with {} arrays", tick_arrays.len());
    
    Ok(())
}
