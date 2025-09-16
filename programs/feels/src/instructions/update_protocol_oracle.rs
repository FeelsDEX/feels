//! Update protocol oracle (MVP)

use crate::{
    error::FeelsError,
    events::OracleUpdatedProtocol,
    state::{ProtocolConfig, ProtocolOracle, SafetyController, compute_divergence_bps},
};
use anchor_lang::prelude::*;

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
    require_keys_eq!(
        ctx.accounts.updater.key(),
        cfg.dex_twap_updater,
        FeelsError::UnauthorizedSigner
    );

    // Basic validation
    require!(
        params.window_secs >= 300 && params.window_secs <= 7200,
        FeelsError::InvalidMarket
    );
    // Accept obs >= 1
    // Whitelist validation
    let mut allowed = false;
    for i in 0..(cfg.dex_whitelist_len as usize) {
        if cfg.dex_whitelist[i] == params.venue_id {
            allowed = true;
            break;
        }
    }
    require!(
        allowed || cfg.dex_whitelist_len == 0,
        FeelsError::UnauthorizedSigner
    );

    oracle.dex_twap_rate_q64 = params.dex_twap_rate_q64;
    oracle.dex_last_update_slot = Clock::get()?.slot;
    oracle.dex_last_update_ts = clock.unix_timestamp;
    oracle.dex_window_secs = params.window_secs;

    // Emit oracle update with both rates (native may be zero if not set yet)
    let div_bps = if oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        compute_divergence_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64)
    } else {
        0
    };
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

    // Use centralized divergence check
    safety.check_and_update_divergence(
        oracle,
        cfg,
        Clock::get()?.slot,
        clock.unix_timestamp,
    )?;

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

pub fn update_native_rate(
    ctx: Context<UpdateNativeRate>,
    params: UpdateNativeRateParams,
) -> Result<()> {
    let oracle = &mut ctx.accounts.protocol_oracle;
    oracle.native_rate_q64 = params.native_rate_q64;
    oracle.native_last_update_slot = Clock::get()?.slot;
    oracle.native_last_update_ts = ctx.accounts.clock.unix_timestamp;

    // Also emit with full context
    let cfg = &ctx.accounts.protocol_config;
    let safety = &ctx.accounts.safety;
    let div_bps = if oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
        compute_divergence_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64)
    } else {
        0
    };
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

    // Use centralized divergence check after native rate update
    let safety = &mut ctx.accounts.safety;
    safety.check_and_update_divergence(
        oracle,
        &ctx.accounts.protocol_config,
        Clock::get()?.slot,
        ctx.accounts.clock.unix_timestamp,
    )?;

    Ok(())
}

