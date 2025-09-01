/// Position management logic with virtual rebasing integration.
/// Handles position creation, updates, and yield accrual through lazy evaluation.
use anchor_lang::prelude::*;
use crate::state::{TickPositionMetadata, RebaseAccumulator, Pool};
use crate::state::rebase::{create_checkpoint, apply_position_rebase};
use crate::state::duration::Duration;

// ============================================================================
// Position Creation with Rebasing
// ============================================================================

/// Create a new position with rebase checkpoint
pub fn create_position_with_rebase(
    position: &mut TickPositionMetadata,
    pool: &Pool,
    rebase_accumulator: &RebaseAccumulator,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    leverage: u64,
    duration: Duration,
    owner: Pubkey,
    is_long: bool,
) -> Result<()> {
    // Set basic position parameters
    position.pool = pool.key();
    position.owner = owner;
    position.tick_lower = tick_lower;
    position.tick_upper = tick_upper;
    position.liquidity = liquidity;
    position.leverage = leverage;
    position.duration = duration;
    
    // Initialize fee tracking
    position.fee_growth_inside_last_a = pool.fee_growth_global_a;
    position.fee_growth_inside_last_b = pool.fee_growth_global_b;
    position.tokens_owed_a = 0;
    position.tokens_owed_b = 0;
    
    // Set creation and maturity slots
    let current_slot = Clock::get()?.slot;
    position.creation_slot = current_slot;
    position.maturity_slot = calculate_maturity_slot(current_slot, duration);
    
    // Create risk profile hash
    position.risk_profile_hash = TickPositionMetadata::calculate_risk_profile_hash(
        leverage,
        10000, // Default protection factor
    );
    
    // Initialize rebase checkpoint
    position.rebase_checkpoint = create_checkpoint(rebase_accumulator, is_long);
    
    Ok(())
}

// ============================================================================
// Position Value Calculation with Rebasing
// ============================================================================

/// Calculate current position value including accrued yield and funding
pub fn calculate_position_value_with_yield(
    position: &TickPositionMetadata,
    pool: &Pool,
    rebase_accumulator: &RebaseAccumulator,
) -> Result<(u64, u64)> {
    // Calculate base position value from liquidity
    let sqrt_price_lower = sqrt_price_from_tick(position.tick_lower)?;
    let sqrt_price_upper = sqrt_price_from_tick(position.tick_upper)?;
    let current_sqrt_price = pool.current_sqrt_rate;
    
    let (base_value_a, base_value_b) = if current_sqrt_price <= sqrt_price_lower {
        // Position is entirely in token B
        let value_b = calculate_token_b_amount(
            position.liquidity,
            sqrt_price_lower,
            sqrt_price_upper,
        )?;
        (0, value_b)
    } else if current_sqrt_price >= sqrt_price_upper {
        // Position is entirely in token A
        let value_a = calculate_token_a_amount(
            position.liquidity,
            sqrt_price_lower,
            sqrt_price_upper,
        )?;
        (value_a, 0)
    } else {
        // Position is in range
        let value_a = calculate_token_a_amount(
            position.liquidity,
            current_sqrt_price,
            sqrt_price_upper,
        )?;
        let value_b = calculate_token_b_amount(
            position.liquidity,
            sqrt_price_lower,
            current_sqrt_price,
        )?;
        (value_a, value_b)
    };
    
    // Apply virtual rebasing to include yield and funding
    apply_position_rebase(
        base_value_a,
        base_value_b,
        &position.rebase_checkpoint,
        rebase_accumulator,
        position.is_leveraged(),
        true, // Assume long for now
    )
}

// ============================================================================
// Yield Claiming
// ============================================================================

/// Claim accrued yield for a position
pub fn claim_position_yield(
    position: &mut TickPositionMetadata,
    rebase_accumulator: &RebaseAccumulator,
) -> Result<(u64, u64)> {
    // Get current values with yield
    let (current_a, current_b) = apply_position_rebase(
        position.tokens_owed_a,
        position.tokens_owed_b,
        &position.rebase_checkpoint,
        rebase_accumulator,
        position.is_leveraged(),
        true,
    )?;
    
    // Calculate yield portions
    let yield_a = current_a.saturating_sub(position.tokens_owed_a);
    let yield_b = current_b.saturating_sub(position.tokens_owed_b);
    
    // Update checkpoint to current indices
    position.rebase_checkpoint = create_checkpoint(rebase_accumulator, true);
    
    Ok((yield_a, yield_b))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_maturity_slot(current_slot: u64, duration: Duration) -> u64 {
    match duration {
        Duration::Flash => current_slot + 1,
        Duration::Swap => 0, // No maturity
        Duration::Weekly => current_slot + 7 * 24 * 60 * 60 / 2, // ~7 days at 2s/slot
        Duration::Monthly => current_slot + 30 * 24 * 60 * 60 / 2, // ~30 days
        Duration::Quarterly => current_slot + 90 * 24 * 60 * 60 / 2, // ~90 days
        Duration::Annual => current_slot + 365 * 24 * 60 * 60 / 2, // ~365 days
    }
}

fn sqrt_price_from_tick(tick: i32) -> Result<u128> {
    // Simplified calculation - in production use proper tick math
    let tick_abs = tick.abs() as u32;
    let base = 1_000_100u128; // ~1.0001
    let mut sqrt_price = 1u128 << 64; // Q64.64 format
    
    for _ in 0..tick_abs {
        sqrt_price = sqrt_price.saturating_mul(base) / 1_000_000;
    }
    
    if tick < 0 {
        sqrt_price = (1u128 << 128) / sqrt_price;
    }
    
    Ok(sqrt_price)
}

fn calculate_token_a_amount(
    liquidity: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
) -> Result<u64> {
    // amount_a = liquidity * (1/sqrt_lower - 1/sqrt_upper)
    let inv_lower = (1u128 << 128) / sqrt_price_lower;
    let inv_upper = (1u128 << 128) / sqrt_price_upper;
    let delta = inv_lower.saturating_sub(inv_upper);
    
    Ok((liquidity.saturating_mul(delta) >> 128) as u64)
}

fn calculate_token_b_amount(
    liquidity: u128,
    sqrt_price_lower: u128,
    sqrt_price_upper: u128,
) -> Result<u64> {
    // amount_b = liquidity * (sqrt_upper - sqrt_lower)
    let delta = sqrt_price_upper.saturating_sub(sqrt_price_lower);
    
    Ok((liquidity.saturating_mul(delta) >> 64) as u64)
}