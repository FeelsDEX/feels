//! Update protocol oracle (MVP)

use anchor_lang::prelude::*;
use crate::{
    error::FeelsError,
    events::{OracleUpdatedProtocol, RedemptionsPaused, RedemptionsResumed, CircuitBreakerActivated, SafetyPaused, SafetyResumed},
    state::{ProtocolConfig, ProtocolOracle, SafetyController},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateDexTwapParams {
    pub dex_twap_rate_q64: u128,
    pub window_secs: u32,
    pub obs: u16,
    pub venue_id: Pubkey,
}

#[derive(Accounts)]
pub struct UpdateDexTwap<'info> {
    /// Updater authorized in ProtocolConfig
    #[account(mut)]
    pub updater: Signer<'info>,

    /// Protocol config (for params and updater key)
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// Protocol oracle (singleton)
    #[account(
        mut,
        seeds = [ProtocolOracle::SEED],
        bump,
    )]
    pub protocol_oracle: Account<'info, ProtocolOracle>,

    /// Safety controller (singleton)
    #[account(
        mut,
        seeds = [SafetyController::SEED],
        bump,
    )]
    pub safety: Account<'info, SafetyController>,

    /// Clock sysvar
    pub clock: Sysvar<'info, Clock>,
}

pub fn update_dex_twap(ctx: Context<UpdateDexTwap>, params: UpdateDexTwapParams) -> Result<()> {
    let cfg = &ctx.accounts.protocol_config;
    let oracle = &mut ctx.accounts.protocol_oracle;
    let safety = &mut ctx.accounts.safety;
    let clock = &ctx.accounts.clock;

    // Authorization: updater must match configured updater
    require_keys_eq!(ctx.accounts.updater.key(), cfg.dex_twap_updater, FeelsError::UnauthorizedSigner);

    // Basic validation
    require!(params.window_secs >= 300 && params.window_secs <= 7200, FeelsError::InvalidMarket);
    // Accept obs >= 1
    // Whitelist validation
    let mut allowed = false;
    for i in 0..(cfg.dex_whitelist_len as usize) {
        if cfg.dex_whitelist[i] == params.venue_id { allowed = true; break; }
    }
    require!(allowed || cfg.dex_whitelist_len == 0, FeelsError::UnauthorizedSigner);

    oracle.dex_twap_rate_q64 = params.dex_twap_rate_q64;
    oracle.dex_last_update_slot = Clock::get()?.slot;
    oracle.dex_last_update_ts = clock.unix_timestamp;
    oracle.dex_window_secs = params.window_secs;

    // Emit oracle update with both rates (native may be zero if not set yet)
    let div_bps = if oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        compute_div_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64)
    } else { 0 };
    emit!(OracleUpdatedProtocol {
        native_q64: oracle.native_rate_q64,
        dex_twap_q64: oracle.dex_twap_rate_q64,
        min_rate_q64: oracle.min_rate_q64(),
        div_bps,
        threshold_bps: cfg.depeg_threshold_bps,
        window_secs: oracle.dex_window_secs,
        paused: safety.redemptions_paused,
        timestamp: clock.unix_timestamp,
    });

    // Safety: compute divergence if both are set and DEX TWAP not stale
    let dex_stale = if oracle.dex_last_update_ts > 0 {
        (clock.unix_timestamp - oracle.dex_last_update_ts) > cfg.dex_twap_stale_age_secs as i64
    } else { true };
    if !dex_stale && oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        let div_bps = compute_div_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64);

        if div_bps >= cfg.depeg_threshold_bps {
            // breach
            safety.consecutive_breaches = safety.consecutive_breaches.saturating_add(1);
            safety.consecutive_clears = 0;
            if !safety.redemptions_paused && safety.consecutive_breaches >= cfg.depeg_required_obs {
                safety.redemptions_paused = true;
                safety.last_change_slot = Clock::get()?.slot;
                emit!(CircuitBreakerActivated { threshold_bps: cfg.depeg_threshold_bps, window_secs: oracle.dex_window_secs });
                emit!(RedemptionsPaused { timestamp: clock.unix_timestamp });
                emit!(SafetyPaused { timestamp: clock.unix_timestamp });
            }
        } else {
            // clear
            safety.consecutive_clears = safety.consecutive_clears.saturating_add(1);
            safety.consecutive_breaches = 0;
            if safety.redemptions_paused && safety.consecutive_clears >= cfg.clear_required_obs {
                safety.redemptions_paused = false;
                safety.last_change_slot = Clock::get()?.slot;
                emit!(RedemptionsResumed { timestamp: clock.unix_timestamp });
                emit!(SafetyResumed { timestamp: clock.unix_timestamp });
            }
        }
    }

    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateNativeRateParams {
    pub native_rate_q64: u128,
}

#[derive(Accounts)]
pub struct UpdateNativeRate<'info> {
    /// Protocol authority
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Protocol config
    #[account(
        seeds = [ProtocolConfig::SEED],
        bump,
        constraint = protocol_config.authority == authority.key() @ FeelsError::UnauthorizedSigner,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    /// Protocol oracle
    #[account(
        mut,
        seeds = [ProtocolOracle::SEED],
        bump,
    )]
    pub protocol_oracle: Account<'info, ProtocolOracle>,
    /// Safety controller
    #[account(
        mut,
        seeds = [SafetyController::SEED],
        bump,
    )]
    pub safety: Account<'info, SafetyController>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn update_native_rate(ctx: Context<UpdateNativeRate>, params: UpdateNativeRateParams) -> Result<()> {
    let oracle = &mut ctx.accounts.protocol_oracle;
    oracle.native_rate_q64 = params.native_rate_q64;
    oracle.native_last_update_slot = Clock::get()?.slot;
    oracle.native_last_update_ts = ctx.accounts.clock.unix_timestamp;

    // Also emit with full context
    let cfg = &ctx.accounts.protocol_config;
    let safety = &ctx.accounts.safety;
    let div_bps = if oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        compute_div_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64)
    } else { 0 };
    emit!(OracleUpdatedProtocol {
        native_q64: oracle.native_rate_q64,
        dex_twap_q64: oracle.dex_twap_rate_q64,
        min_rate_q64: oracle.min_rate_q64(),
        div_bps,
        threshold_bps: cfg.depeg_threshold_bps,
        window_secs: oracle.dex_window_secs,
        paused: safety.redemptions_paused,
        timestamp: ctx.accounts.clock.unix_timestamp,
    });

    // Re-run safety evaluation via the same path
    // Delegate to dex update logic if both set (reuse code pattern inline for simplicity)
    if oracle.dex_last_update_ts > 0 && (ctx.accounts.clock.unix_timestamp - oracle.dex_last_update_ts) <= ctx.accounts.protocol_config.dex_twap_stale_age_secs as i64
        && oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        let cfg = &ctx.accounts.protocol_config;
        let safety = &mut ctx.accounts.safety;
        let div_bps = compute_div_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64);
        if div_bps >= cfg.depeg_threshold_bps {
            safety.consecutive_breaches = safety.consecutive_breaches.saturating_add(1);
            safety.consecutive_clears = 0;
            if !safety.redemptions_paused && safety.consecutive_breaches >= cfg.depeg_required_obs {
                safety.redemptions_paused = true;
                safety.last_change_slot = Clock::get()?.slot;
                emit!(CircuitBreakerActivated { threshold_bps: cfg.depeg_threshold_bps, window_secs: oracle.dex_window_secs });
                emit!(RedemptionsPaused { timestamp: ctx.accounts.clock.unix_timestamp });
                emit!(SafetyPaused { timestamp: ctx.accounts.clock.unix_timestamp });
            }
        } else {
            safety.consecutive_clears = safety.consecutive_clears.saturating_add(1);
            safety.consecutive_breaches = 0;
            if safety.redemptions_paused && safety.consecutive_clears >= cfg.clear_required_obs {
                safety.redemptions_paused = false;
                safety.last_change_slot = Clock::get()?.slot;
                emit!(RedemptionsResumed { timestamp: ctx.accounts.clock.unix_timestamp });
                emit!(SafetyResumed { timestamp: ctx.accounts.clock.unix_timestamp });
            }
        }
    }

    Ok(())
}

/// Compute divergence in basis points between native and DEX TWAP
pub fn compute_div_bps(native_q64: u128, dex_q64: u128) -> u16 {
    if native_q64 == 0 { return 0; }
    let n = native_q64 as i128;
    let d = dex_q64 as i128;
    let diff = (n - d).unsigned_abs();
    ((diff.saturating_mul(10_000)) / (native_q64.max(1))) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_div_bps() {
        assert_eq!(compute_div_bps(1000, 1000), 0);
        assert_eq!(compute_div_bps(1000, 900), 1000);
        assert_eq!(compute_div_bps(1000, 800), 2000);
        assert_eq!(compute_div_bps(0, 1000), 0);
    }
}
