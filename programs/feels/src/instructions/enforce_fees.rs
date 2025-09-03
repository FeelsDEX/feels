/// Fee enforcement instruction that validates orders against the fees policy.
/// Ensures minimum fees are collected and pools can be disabled under stress.

use anchor_lang::prelude::*;
use crate::state::{
    FeelsProtocolError, FeesPolicy, PoolStatus,
    MarketField, BufferAccount, TwapOracle, FieldCommitment,
};
use crate::logic::instantaneous_fee::{
    calculate_order_fees_instantaneous, calculate_swap_work_simplified,
};
use crate::constant::Q64;

// ============================================================================
// Accounts
// ============================================================================

#[derive(Accounts)]
pub struct EnforceFees<'info> {
    /// Market field being traded on
    #[account(mut)]
    pub market_field: Account<'info, MarketField>,
    
    /// Pool status tracking
    #[account(
        mut,
        seeds = [b"pool_status", market_field.key().as_ref()],
        bump,
    )]
    pub pool_status: AccountLoader<'info, PoolStatus>,
    
    /// Field commitment for the market
    #[account(
        seeds = [b"field_commitment", market_field.key().as_ref()],
        bump,
    )]
    pub field_commitment: AccountLoader<'info, FieldCommitment>,
    
    /// Buffer account for fees
    #[account(
        mut,
        seeds = [b"buffer", market_field.key().as_ref()],
        bump,
    )]
    pub buffer: Account<'info, BufferAccount>,
    
    /// TWAP oracle
    #[account(
        seeds = [b"twap", market_field.key().as_ref()],
        bump,
    )]
    pub twap_oracle: AccountLoader<'info, TwapOracle>,
    
    /// Protocol-wide fees policy
    #[account(
        seeds = [b"fees_policy"],
        bump,
    )]
    pub fees_policy: AccountLoader<'info, FeesPolicy>,
    
    /// Clock for time checks
    pub clock: Sysvar<'info, Clock>,
}

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct EnforceFeesParams {
    /// Order amount in
    pub amount_in: u64,
    
    /// Expected amount out
    pub amount_out: u64,
    
    /// Direction of swap
    pub zero_for_one: bool,
    
    /// Current sqrt price
    pub sqrt_price_current: u128,
    
    /// Target sqrt price
    pub sqrt_price_target: u128,
    
    /// Liquidity in range
    pub liquidity: u128,
}

// ============================================================================
// Result
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct EnforceFeesResult {
    /// Actual fee amount to collect
    pub fee_amount: u64,
    
    /// Rebate amount (if any)
    pub rebate_amount: u64,
    
    /// Effective fee rate in basis points
    pub effective_fee_bps: u64,
    
    /// Whether pool is operational
    pub pool_operational: bool,
    
    /// Pool status
    pub pool_status: u8,
}

// ============================================================================
// Handler
// ============================================================================

pub fn handler(
    ctx: Context<EnforceFees>,
    params: EnforceFeesParams,
) -> Result<EnforceFeesResult> {
    let current_time = ctx.accounts.clock.unix_timestamp;
    
    // Load accounts
    let market_field = &ctx.accounts.market_field;
    let mut pool_status = ctx.accounts.pool_status.load_mut()?;
    let field_commitment = ctx.accounts.field_commitment.load()?;
    let buffer = &ctx.accounts.buffer;
    let twap_oracle = ctx.accounts.twap_oracle.load()?;
    let fees_policy = ctx.accounts.fees_policy.load()?;
    
    // 1. Check field commitment freshness
    let commitment_age = current_time - field_commitment.snapshot_ts;
    let is_stale = commitment_age > fees_policy.max_commitment_staleness;
    
    // 2. Get base fee from field commitment or use fallback
    let base_fee_bps = if is_stale {
        msg!("Field commitment stale, using fallback fee");
        fees_policy.fallback_fee_bps
    } else {
        field_commitment.base_fee_bps
    };
    
    // 3. Validate against minimum fee policy
    require!(
        base_fee_bps >= fees_policy.min_base_fee_bps,
        FeelsProtocolError::FeeBelowMinimum
    );
    
    // 4. Calculate work for the order
    let work = calculate_swap_work_simplified(
        params.sqrt_price_current,
        params.sqrt_price_target,
        params.liquidity,
        params.zero_for_one,
        &market_field,
    )?;
    
    // 5. Calculate instantaneous fees
    let fee_result = calculate_order_fees_instantaneous(
        params.amount_in,
        params.amount_out,
        params.zero_for_one,
        work,
        &twap_oracle,
        &buffer,
    )?;
    
    // 6. Apply status-based fee multiplier
    let fee_multiplier = pool_status.get_fee_multiplier();
    let adjusted_fee = (fee_result.fee_amount as u128)
        .saturating_mul(fee_multiplier as u128)
        .saturating_div(10000) // BPS_DENOMINATOR
        .min(u64::MAX as u128) as u64;
    
    // 7. Check stress levels if not already disabled
    if pool_status.status != 2 { // Disabled
        // Get stress components from field commitment and market conditions
        let spot_stress = calculate_spot_stress(&field_commitment, &twap_oracle)?;
        let time_stress = calculate_time_stress(&market_field)?;
        let leverage_stress = calculate_leverage_stress(&market_field)?;
        
        // Check if pool should be disabled
        let should_disable = fees_policy.should_disable_pool(
            spot_stress,
            time_stress,
            leverage_stress,
        );
        
        // Update stress tracking
        pool_status.update_stress_tracking(
            &fees_policy,
            should_disable,
            current_time,
        )?;
    }
    
    // 8. Check if pool can accept orders
    let pool_operational = pool_status.can_accept_orders();
    
    // 9. Ensure minimum fee is met even with rebates
    let net_fee = adjusted_fee.saturating_sub(fee_result.rebate_amount);
    let min_required = (params.amount_in as u128)
        .saturating_mul(fees_policy.min_base_fee_bps as u128)
        .saturating_div(10000)
        .min(u64::MAX as u128) as u64;
    
    require!(
        net_fee >= min_required,
        FeelsProtocolError::FeeBelowMinimum
    );
    
    // 10. Update pool status fee tracking
    pool_status.current_base_fee_bps = base_fee_bps;
    pool_status.last_fee_update_ts = current_time;
    
    // Calculate effective fee rate
    let effective_fee_bps = if params.amount_in > 0 {
        (adjusted_fee as u128)
            .saturating_mul(10000)
            .saturating_div(params.amount_in as u128)
            .min(10000) as u64
    } else {
        0
    };
    
    msg!(
        "Fee enforcement: base_fee={} bps, adjusted_fee={}, rebate={}, effective={} bps, status={}",
        base_fee_bps,
        adjusted_fee,
        fee_result.rebate_amount,
        effective_fee_bps,
        pool_status.status as u8
    );
    
    Ok(EnforceFeesResult {
        fee_amount: adjusted_fee,
        rebate_amount: fee_result.rebate_amount,
        effective_fee_bps,
        pool_operational,
        pool_status: pool_status.status as u8,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate spot stress from price deviation
fn calculate_spot_stress(
    field_commitment: &FieldCommitment,
    twap_oracle: &TwapOracle,
) -> Result<u64> {
    // Use field commitment TWAPs vs oracle TWAPs
    let field_price = if field_commitment.twap_0 > 0 && field_commitment.twap_1 > 0 {
        (field_commitment.twap_1 as u128 * Q64) / field_commitment.twap_0 as u128
    } else {
        Q64
    };
    
    let oracle_price = twap_oracle.twap_1_per_0;
    
    // Calculate deviation
    let deviation = if field_price > oracle_price {
        ((field_price - oracle_price) * 10000) / oracle_price
    } else {
        ((oracle_price - field_price) * 10000) / oracle_price
    };
    
    Ok(deviation.min(10000) as u64)
}

/// Calculate time stress from lending utilization and time-weighted positions
fn calculate_time_stress(market_field: &MarketField) -> Result<u64> {
    use crate::constant::Q64;
    
    // Time stress based on T scalar which represents time-weighted liquidity
    // Higher T = more time-locked positions = higher stress
    
    // Normalize T scalar to stress percentage
    // T = Q64 represents neutral state (stress = 0)
    // T = 2*Q64 represents high stress (stress = 5000 bps / 50%)
    
    let stress = if market_field.T > Q64 {
        // Calculate excess above neutral
        let excess = market_field.T.saturating_sub(Q64);
        // Map excess to stress: each Q64 of excess = 5000 bps of stress
        let stress_bps = (excess.saturating_mul(5000)) / Q64;
        stress_bps.min(10000) // Cap at 100%
    } else {
        // Below neutral = low stress
        // Map [0, Q64] to [0, 1000] bps
        let stress_bps = (market_field.T.saturating_mul(1000)) / Q64;
        stress_bps
    };
    
    Ok(stress as u64)
}

/// Calculate leverage stress from long/short imbalance  
fn calculate_leverage_stress(market_field: &MarketField) -> Result<u64> {
    use crate::constant::Q64;
    
    // Leverage stress based on L scalar which represents leverage imbalance
    // L = Q64 represents balanced long/short (stress = 0)
    // Deviation from Q64 in either direction = stress
    
    let stress = if market_field.L > Q64 {
        // Long-heavy imbalance
        let imbalance = market_field.L.saturating_sub(Q64);
        // Each Q64 of imbalance = 2500 bps of stress
        let stress_bps = (imbalance.saturating_mul(2500)) / Q64;
        stress_bps.min(10000)
    } else {
        // Short-heavy imbalance
        let imbalance = Q64.saturating_sub(market_field.L);
        // Same calculation for short imbalance
        let stress_bps = (imbalance.saturating_mul(2500)) / Q64;
        stress_bps.min(10000)
    };
    
    Ok(stress as u64)
}

// ============================================================================
// Initialize Pool Status
// ============================================================================

#[derive(Accounts)]
pub struct InitializePoolStatus<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// Market field
    pub market_field: Account<'info, MarketField>,
    
    /// Pool status account to initialize
    #[account(
        init,
        payer = payer,
        space = 8 + PoolStatus::SIZE,
        seeds = [b"pool_status", market_field.key().as_ref()],
        bump,
    )]
    pub pool_status: AccountLoader<'info, PoolStatus>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_pool_status(ctx: Context<InitializePoolStatus>) -> Result<()> {
    let mut pool_status = ctx.accounts.pool_status.load_init()?;
    let clock = Clock::get()?;
    
    pool_status.pool = ctx.accounts.market_field.key();
    pool_status.status = 0; // Normal
    pool_status.current_base_fee_bps = crate::constant::MIN_FEE_BPS;
    pool_status.last_fee_update_ts = clock.unix_timestamp;
    
    msg!("Initialized pool status for {}", pool_status.pool);
    
    Ok(())
}