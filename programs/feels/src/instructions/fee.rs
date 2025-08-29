/// Fee management instructions including collection of LP fees, protocol fees, and dynamic fee updates.
/// These operations handle the revenue model of the AMM where liquidity providers earn fees from trades
/// and the protocol collects a configurable share for treasury operations and development funding.
use anchor_lang::prelude::*;
use crate::logic::event::{FeeCollectionEvent, ProtocolFeeCollectionEvent};
use crate::logic::fee_manager::FeeManager;
use crate::state::{FeelsProtocolError, DynamicFeeConfig};
use crate::utils::cpi_helpers::{transfer_pair_from_pool_to_user, collect_protocol_fees as transfer_protocol_fees};

// ============================================================================
// LP Fee Collection
// ============================================================================

/// Collect accumulated fees for a liquidity position
///
/// This implementation:
/// 1. Calculates fees owed based on fee growth since last collection
/// 2. Updates position's fee growth checkpoints
/// 3. Transfers fees from pool to user
/// 4. Resets tokens_owed counters
pub fn collect_pool_fees(
    ctx: Context<crate::CollectFees>,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Result<(u64, u64)> {
    let pool = &ctx.accounts.pool.load()?;
    let position = &mut ctx.accounts.position;

    // Validate position belongs to pool
    require!(
        position.pool == ctx.accounts.pool.key(),
        FeelsProtocolError::InvalidPool
    );

    // Validate position owner
    require!(
        position.owner == ctx.accounts.position_authority.key(),
        FeelsProtocolError::InvalidAuthority
    );

    // Calculate fees based on fee growth
    let collected_amount_0 = std::cmp::min(amount_0_requested, position.tokens_owed_0);
    let collected_amount_1 = std::cmp::min(amount_1_requested, position.tokens_owed_1);

    // Update position fees owed
    position.tokens_owed_0 = position.tokens_owed_0.saturating_sub(collected_amount_0);
    position.tokens_owed_1 = position.tokens_owed_1.saturating_sub(collected_amount_1);

    // Update fee growth inside
    position.fee_growth_inside_0_last = pool.fee_growth_global_a;
    position.fee_growth_inside_1_last = pool.fee_growth_global_b;

    // Transfer fees from pool to user
    if collected_amount_0 > 0 || collected_amount_1 > 0 {
        let pool_bump = ctx.bumps.pool;
        transfer_pair_from_pool_to_user(
            &ctx.accounts.token_vault_a,
            &ctx.accounts.token_vault_b,
            &ctx.accounts.user_token_a,
            &ctx.accounts.user_token_b,
            &ctx.accounts.pool,
            &ctx.accounts.token_program,
            collected_amount_0,
            collected_amount_1,
            pool_bump,
        )?;
    }

    // Emit fee collection event
    emit!(FeeCollectionEvent {
        pool: ctx.accounts.pool.key(),
        position: position.key(),
        owner: position.owner,
        amount_0: collected_amount_0,
        amount_1: collected_amount_1,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool fees collected");
    msg!("Amount 0: {}", collected_amount_0);
    msg!("Amount 1: {}", collected_amount_1);

    Ok((collected_amount_0, collected_amount_1))
}

// ============================================================================
// Protocol Fee Collection
// ============================================================================

/// Collect accumulated protocol fees from a pool
///
/// Protocol fees accumulate in the pool and can be collected by the protocol authority.
pub fn collect_protocol_fees(
    ctx: Context<crate::CollectProtocolFees>,
    amount_0_requested: u64,
    amount_1_requested: u64,
) -> Result<(u64, u64)> {
    let pool = &mut ctx.accounts.pool.load_mut()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Calculate amounts to collect (cannot exceed available)
    let collect_amount_0 = std::cmp::min(amount_0_requested, pool.protocol_fees_a);
    let collect_amount_1 = std::cmp::min(amount_1_requested, pool.protocol_fees_b);

    // Update pool protocol fee counters
    pool.protocol_fees_a = pool.protocol_fees_a.saturating_sub(collect_amount_0);
    pool.protocol_fees_b = pool.protocol_fees_b.saturating_sub(collect_amount_1);

    // Transfer protocol fees to treasury
    if collect_amount_0 > 0 || collect_amount_1 > 0 {
        let pool_bump = ctx.bumps.pool;
        collect_protocol_fees(
            &ctx.accounts.token_vault_a,
            &ctx.accounts.token_vault_b,
            &ctx.accounts.treasury_token_a,
            &ctx.accounts.treasury_token_b,
            &ctx.accounts.pool,
            &ctx.accounts.token_program,
            collect_amount_0,
            collect_amount_1,
            pool_bump,
        )?;
    }

    // Update pool timestamp
    pool.last_updated_at = Clock::get()?.unix_timestamp;

    // Emit protocol fee collection event
    emit!(ProtocolFeeCollectionEvent {
        pool: ctx.accounts.pool.key(),
        authority: ctx.accounts.authority.key(),
        treasury_a: ctx.accounts.treasury_token_a.key(),
        treasury_b: ctx.accounts.treasury_token_b.key(),
        amount_0: collect_amount_0,
        amount_1: collect_amount_1,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Protocol fees collected");
    msg!("Amount 0: {}", collect_amount_0);
    msg!("Amount 1: {}", collect_amount_1);
    msg!("Remaining protocol fees - A: {}, B: {}", pool.protocol_fees_a, pool.protocol_fees_b);

    Ok((collect_amount_0, collect_amount_1))
}

// ============================================================================
// Dynamic Fee Updates
// ============================================================================

/// Update dynamic fee configuration for a pool
/// Dynamic fees adjust based on market conditions like volatility and volume
pub fn update_dynamic_fees(
    ctx: Context<crate::UpdateDynamicFees>,
    params: crate::UpdateDynamicFeesParams,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool.load_mut()?;

    // Validate authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Validate fee parameters
    require!(
        params.min_fee <= params.base_fee && params.base_fee <= params.max_fee,
        FeelsProtocolError::InvalidFeeRate
    );
    require!(
        params.max_fee <= 1000, // Max 10%
        FeelsProtocolError::InvalidFeeRate
    );

    // Create dynamic fee config
    let config = DynamicFeeConfig {
        base_fee: params.base_fee,
        min_fee: params.min_fee,
        max_fee: params.max_fee,
        min_multiplier: 5000,  // 50% minimum (0.5x)
        max_multiplier: 20000, // 200% maximum (2x)
        _padding: 0,
        volatility_coefficient: params.volatility_coefficient,
        volume_discount_threshold: params.volume_discount_threshold,
    };

    // Use FeeManager to update dynamic fee configuration
    FeeManager::update_dynamic_fee_config(pool, config)?;

    // Update pool timestamp
    pool.last_updated_at = Clock::get()?.unix_timestamp;

    msg!("Dynamic fees updated");
    msg!("Base fee: {} bps", params.base_fee);
    msg!("Fee range: {} - {} bps", params.min_fee, params.max_fee);
    msg!("Volatility coefficient: {}", params.volatility_coefficient);
    msg!("Volume discount threshold: {}", params.volume_discount_threshold);

    Ok(())
}