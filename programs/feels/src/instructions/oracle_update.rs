/// Secure oracle update instruction with advanced manipulation protection and TWAP computation.
/// Validates price movements against historical data to detect and prevent oracle manipulation.
/// Maintains cumulative price data for time-weighted average calculations and volatility metrics
/// used by the protocol's dynamic fee system and risk management mechanisms.

use anchor_lang::prelude::*;
use crate::state::{Oracle, OracleData};
use crate::state::metrics_price::PriceObservation;
use crate::state::{Pool, FeelsProtocolError};

// ============================================================================
// Oracle Update Handler
// ============================================================================

/// Update oracle with manipulation protection
pub fn handler(
    ctx: Context<UpdateOracle>,
) -> Result<()> {
    let oracle = &mut ctx.accounts.oracle;
    let oracle_data = &mut ctx.accounts.oracle_data.load_mut()?;
    let pool = ctx.accounts.pool.load()?;
    
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;
    let current_slot = clock.slot;
    
    // Get current pool price and tick
    let current_sqrt_price = pool.current_sqrt_rate;
    let current_tick = pool.current_tick;
    
    // Add observation with manipulation protection
    oracle.add_observation(
        current_sqrt_price,
        current_tick,
        current_timestamp,
        oracle_data,
    )?;
    
    msg!(
        "Oracle updated successfully. TWAP 5min: {}, Stale: {}",
        oracle.twap_5min,
        oracle.is_stale(current_timestamp)
    );
    
    Ok(())
}

// ============================================================================
// Account Structures
// ============================================================================

#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    /// Oracle header account
    #[account(
        mut,
        constraint = oracle.pool == pool.key() @ FeelsProtocolError::InvalidOracle,
    )]
    pub oracle: Account<'info, Oracle>,
    
    /// Oracle data account
    #[account(
        mut,
        constraint = oracle.data_account == oracle_data.key() @ FeelsProtocolError::InvalidOracle,
    )]
    pub oracle_data: AccountLoader<'info, OracleData>,
    
    /// Pool account
    pub pool: AccountLoader<'info, Pool>,
    
    /// Anyone can update the oracle (keeper, user, etc)
    pub updater: Signer<'info>,
}