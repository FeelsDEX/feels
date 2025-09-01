/// Update rebase accumulator indices and set new rates for yield and funding.
/// This instruction should be called periodically to accrue yield and distribute funding.
use anchor_lang::prelude::*;
use crate::state::{Pool, RebaseAccumulator, Oracle, LendingMetrics};
use crate::state::rebase::{calculate_supply_rate, calculate_borrow_rate, calculate_funding_rate};

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct RebaseUpdateParams {
    /// New supply rate for token A (basis points per year, 0 to use calculated)
    pub supply_rate_a: Option<u64>,
    
    /// New supply rate for token B (basis points per year, 0 to use calculated)
    pub supply_rate_b: Option<u64>,
    
    /// Maximum funding rate allowed (basis points per year)
    pub max_funding_rate: u64,
}

// ============================================================================
// Instruction Handler
// ============================================================================

pub fn handler(ctx: Context<UpdateRebase>, params: RebaseUpdateParams) -> Result<()> {
    let pool = &ctx.accounts.pool.load()?;
    let rebase = &mut ctx.accounts.rebase_accumulator.load_mut()?;
    let clock = Clock::get()?;
    
    // Update indices based on elapsed time
    rebase.update_indices(clock.unix_timestamp)?;
    
    // Calculate new rates based on market conditions
    let (new_rate_a, new_rate_b) = if let Some(lending_metrics) = ctx.accounts.lending_metrics.as_ref() {
        let metrics = lending_metrics.load()?;
        
        // Calculate rates based on utilization
        let utilization_a = metrics.utilization_rate_a();
        let utilization_b = metrics.utilization_rate_b();
        
        let supply_a = params.supply_rate_a.unwrap_or_else(|| {
            calculate_supply_rate(utilization_a, 300, 1000) // 3% base, 10% reserve
        });
        
        let supply_b = params.supply_rate_b.unwrap_or_else(|| {
            calculate_supply_rate(utilization_b, 300, 1000) // 3% base, 10% reserve
        });
        
        (supply_a, supply_b)
    } else {
        // Use provided rates or defaults
        (
            params.supply_rate_a.unwrap_or(0),
            params.supply_rate_b.unwrap_or(0),
        )
    };
    
    // Calculate funding rate based on long/short imbalance
    let funding_rate = if let Some(oracle) = ctx.accounts.oracle.as_ref() {
        let oracle_data = oracle.load()?;
        
        // Estimate long/short values based on pool metrics
        // In production, would track actual leveraged positions
        let total_liquidity = pool.liquidity;
        let long_value = total_liquidity / 3; // Placeholder: 1/3 long
        let short_value = total_liquidity / 3; // Placeholder: 1/3 short
        
        calculate_funding_rate(long_value, short_value, params.max_funding_rate)
    } else {
        0
    };
    
    // Update rates in accumulator
    rebase.set_supply_rates(new_rate_a, new_rate_b);
    rebase.set_funding_rate(funding_rate);
    
    msg!("Rebase rates updated:");
    msg!("  Supply A: {} bps/year", new_rate_a);
    msg!("  Supply B: {} bps/year", new_rate_b);
    msg!("  Funding: {} bps/year", funding_rate);
    msg!("  Indices updated to timestamp {}", clock.unix_timestamp);
    
    Ok(())
}

// ============================================================================
// Account Structures
// ============================================================================

#[derive(Accounts)]
pub struct UpdateRebase<'info> {
    pub pool: AccountLoader<'info, Pool>,
    
    #[account(
        mut,
        constraint = pool.load()?.rebase_accumulator == rebase_accumulator.key()
    )]
    pub rebase_accumulator: AccountLoader<'info, RebaseAccumulator>,
    
    /// Optional: Oracle for calculating funding rates
    pub oracle: Option<AccountLoader<'info, Oracle>>,
    
    /// Optional: Lending metrics for calculating supply rates
    pub lending_metrics: Option<AccountLoader<'info, LendingMetrics>>,
    
    /// Anyone can call this instruction to update rates
    pub updater: Signer<'info>,
}