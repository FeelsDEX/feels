/// Computes and returns the tick array PDAs required for a swap transaction.
/// This read-only helper allows clients to pre-fetch all necessary tick arrays before
/// executing a swap, avoiding transaction failures due to missing accounts. Essential for
/// efficient client implementations as it predicts the price path through tick ranges.

use crate::state::PoolError;
use crate::utils::TickMath;
use crate::constant::{TICK_ARRAY_SIZE, MAX_TICK_ARRAYS_PER_SWAP};
use anchor_lang::prelude::*;

// ============================================================================
// Handler Functions
// ============================================================================

pub fn handler(
    ctx: Context<crate::GetSwapTickArrays>,
    amount_in: u64,
    sqrt_price_limit: u128,
    zero_for_one: bool,
) -> Result<SwapTickArrayInfo> {
    require!(amount_in > 0, PoolError::InvalidAmount);
    
    let pool = ctx.accounts.pool.load()?;
    
    // Calculate the price range the swap will traverse
    let _current_sqrt_price = pool.current_sqrt_price;
    let start_tick = pool.current_tick;
    
    // Determine end tick based on price limit
    let end_tick = if sqrt_price_limit > 0 {
        TickMath::get_tick_at_sqrt_ratio(sqrt_price_limit)?
    } else {
        // Use maximum price movement if no limit specified
        if zero_for_one {
            crate::utils::MIN_TICK
        } else {
            crate::utils::MAX_TICK
        }
    };
    
    // Calculate tick arrays needed for this range
    let tick_arrays = calculate_required_tick_arrays(
        start_tick,
        end_tick,
        pool.tick_spacing,
        zero_for_one,
    )?;
    
    // Generate PDAs for each required tick array
    let mut tick_array_pdas = Vec::new();
    for start_tick_index in tick_arrays {
        let (pda, bump) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
            &ctx.accounts.pool.key(),
            start_tick_index,
            ctx.program_id,
        );
        tick_array_pdas.push(TickArrayPda {
            pubkey: pda,
            start_tick_index,
            bump,
        });
    }
    
    let estimated_max_arrays = tick_array_pdas.len() as u8;
    
    Ok(SwapTickArrayInfo {
        pool: ctx.accounts.pool.key(),
        tick_arrays: tick_array_pdas,
        estimated_max_arrays,
        start_tick,
        end_tick,
        zero_for_one,
    })
}

// ============================================================================
// Data Structures
// ============================================================================

/// Information about required tick arrays for a swap
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SwapTickArrayInfo {
    pub pool: Pubkey,
    pub tick_arrays: Vec<TickArrayPda>,
    pub estimated_max_arrays: u8,
    pub start_tick: i32,
    pub end_tick: i32,
    pub zero_for_one: bool,
}

/// Tick array PDA information
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TickArrayPda {
    pub pubkey: Pubkey,
    pub start_tick_index: i32,
    pub bump: u8,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate which tick arrays are needed for a swap
fn calculate_required_tick_arrays(
    start_tick: i32,
    end_tick: i32,
    tick_spacing: i16,
    zero_for_one: bool,
) -> Result<Vec<i32>> {
    let tick_array_start_tick_index = |tick: i32| {
        let ticks_per_array = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        (tick / ticks_per_array) * ticks_per_array
    };
    
    let start_array_index = tick_array_start_tick_index(start_tick);
    let end_array_index = tick_array_start_tick_index(end_tick);
    
    let mut arrays = Vec::new();
    let ticks_per_array = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    
    if zero_for_one {
        // Price decreasing, traverse downward
        let mut current = start_array_index;
        while current >= end_array_index {
            arrays.push(current);
            current -= ticks_per_array;
            
            // Calculate reasonable limit based on tick range
            // Max tick range is 887272 * 2 = 1,774,544 ticks
            // With TICK_ARRAY_SIZE=32 and min tick_spacing=1, max arrays = 55,454
            // But realistically, with typical tick spacings:
            // - spacing=1: max ~55k arrays (unrealistic for one swap)
            // - spacing=10: max ~5.5k arrays
            // - spacing=60: max ~925 arrays
            // - spacing=200: max ~277 arrays
            // A limit of 100 arrays is conservative but allows large swaps
            if arrays.len() >= MAX_TICK_ARRAYS_PER_SWAP {
                break;
            }
        }
    } else {
        // Price increasing, traverse upward
        let mut current = start_array_index;
        while current <= end_array_index {
            arrays.push(current);
            current += ticks_per_array;
            
            // Calculate reasonable limit based on tick range
            // Max tick range is 887272 * 2 = 1,774,544 ticks
            // With TICK_ARRAY_SIZE=32 and min tick_spacing=1, max arrays = 55,454
            // But realistically, with typical tick spacings:
            // - spacing=1: max ~55k arrays (unrealistic for one swap)
            // - spacing=10: max ~5.5k arrays
            // - spacing=60: max ~925 arrays
            // - spacing=200: max ~277 arrays
            // A limit of 100 arrays is conservative but allows large swaps
            if arrays.len() >= MAX_TICK_ARRAYS_PER_SWAP {
                break;
            }
        }
    }
    
    Ok(arrays)
}

// ============================================================================
// SDK Documentation
// ============================================================================

/// Client-side helper function information (for SDK documentation)
/// 
/// Usage example for TypeScript SDK:
/// ```typescript
/// const swapTickArrays = await program.methods
///   .getSwapTickArrays(amountIn, sqrtPriceLimit, zeroForOne)
///   .accounts({ pool: poolPubkey })
///   .view();
/// 
/// // Use the returned tick array PDAs in your swap transaction
/// const swapTx = await program.methods
///   .swap(amountIn, amountOutMin, sqrtPriceLimit, zeroForOne)
///   .accounts({
///     pool: poolPubkey,
///     // ... other accounts
///   })
///   .remainingAccounts(
///     swapTickArrays.tickArrays.map(ta => ({
///       pubkey: ta.pubkey,
///       isSigner: false,
///       isWritable: true,
///     }))
///   )
///   .rpc();
/// ```
pub struct ClientSdkHelper;