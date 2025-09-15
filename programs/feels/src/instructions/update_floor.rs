use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use crate::{
    error::FeelsError,
    events::FloorRatcheted,
    state::{Market, Buffer},
};
use orca_whirlpools_core::tick_index_to_sqrt_price;
use ethnum::U256;

#[derive(Accounts)]
pub struct UpdateFloor<'info> {
    #[account(mut, constraint = market.is_initialized @ FeelsError::MarketNotInitialized)]
    pub market: Account<'info, Market>,
    pub buffer: Account<'info, Buffer>,
    #[account(mut)]
    pub vault_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_1: Account<'info, TokenAccount>,
    pub project_mint: Account<'info, Mint>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn update_floor(ctx: Context<UpdateFloor>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &ctx.accounts.buffer;
    let clock = &ctx.accounts.clock;

    // Identify FeelsSOL as token_0 or token_1
    let feelssol_is_token_0 = market.token_0 == market.feelssol_mint;
    let (feels_vault, project_vault) = if feelssol_is_token_0 {
        (&ctx.accounts.vault_0, &ctx.accounts.vault_1)
    } else {
        (&ctx.accounts.vault_1, &ctx.accounts.vault_0)
    };

    // Compute reserves and circulating supply
    let feels_reserve: u128 = buffer.tau_spot.saturating_add(feels_vault.amount as u128);
    let total_supply: u128 = ctx.accounts.project_mint.supply as u128;
    let pool_owned: u128 = project_vault.amount as u128;
    let circulating: u128 = total_supply.saturating_sub(pool_owned).max(1);

    // Binary search tick for floor price where price = feels/circulating
    // Compare price_num * circulating <= feels << 128, where price_num = (sqrt_price_q64^2)
    let target = U256::from(feels_reserve) << 128;
    let min_tick = market.global_lower_tick.max(-887272);
    let max_tick = market.current_tick.min(887272);
    let mut lo = min_tick;
    let mut hi = max_tick;
    let mut best = lo;
    while lo <= hi {
        let mid = lo + ((hi - lo) / 2);
        let sqrt_q64 = tick_index_to_sqrt_price(mid);
        let sq = U256::from(sqrt_q64) * U256::from(sqrt_q64); // Q128.128
        let lhs = sq * U256::from(circulating);
        if lhs <= target { // price(mid) <= feels/circ
            best = mid; // move up
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    let candidate = best.saturating_sub(market.floor_buffer_ticks);

    if clock.unix_timestamp.saturating_sub(market.last_floor_ratchet_ts) >= market.floor_cooldown_secs
        && candidate > market.floor_tick
    {
        let old = market.floor_tick;
        market.floor_tick = candidate;
        market.last_floor_ratchet_ts = clock.unix_timestamp;
        emit!(FloorRatcheted {
            market: market.key(),
            old_floor_tick: old,
            new_floor_tick: market.floor_tick,
            timestamp: clock.unix_timestamp,
        });
    }
    Ok(())
}

