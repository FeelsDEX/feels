//! Initialize protocol configuration
//!
//! One-time setup instruction to initialize global protocol parameters

use crate::{error::FeelsError, state::ProtocolConfig};
use anchor_lang::prelude::*;

/// Initialize protocol parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeProtocolParams {
    /// Initial mint fee in FeelsSOL lamports
    pub mint_fee: u64,
    /// Treasury account to receive fees
    pub treasury: Pubkey,
    /// Default protocol fee rate (basis points, e.g. 1000 = 10%)
    pub default_protocol_fee_rate: Option<u16>,
    /// Default creator fee rate for protocol tokens (basis points, e.g. 500 = 5%)
    pub default_creator_fee_rate: Option<u16>,
    /// Maximum allowed protocol fee rate (basis points)
    pub max_protocol_fee_rate: Option<u16>,
    /// DEX TWAP updater authority
    pub dex_twap_updater: Pubkey,
    /// De-peg threshold (bps)
    pub depeg_threshold_bps: u16,
    /// Consecutive breaches to pause
    pub depeg_required_obs: u8,
    /// Consecutive clears to resume
    pub clear_required_obs: u8,
    /// DEX TWAP window seconds
    pub dex_twap_window_secs: u32,
    /// DEX TWAP stale age seconds
    pub dex_twap_stale_age_secs: u32,
    /// Initial DEX whitelist (optional; empty ok)
    pub dex_whitelist: Vec<Pubkey>,
}

/// Initialize protocol accounts
#[derive(Accounts)]
#[instruction(params: InitializeProtocolParams)]
pub struct InitializeProtocol<'info> {
    /// Protocol authority (deployer)
    #[account(
        mut,
        constraint = authority.owner == &System::id() @ FeelsError::InvalidAuthority
    )]
    pub authority: Signer<'info>,

    /// Protocol config account
    #[account(
        init,
        payer = authority,
        space = ProtocolConfig::LEN,
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Protocol oracle account (singleton)
    #[account(
        init,
        payer = authority,
        space = crate::state::ProtocolOracle::LEN,
        seeds = [crate::state::ProtocolOracle::SEED],
        bump,
    )]
    pub protocol_oracle: Account<'info, crate::state::ProtocolOracle>,

    /// Safety controller (singleton)
    #[account(
        init,
        payer = authority,
        space = crate::state::SafetyController::LEN,
        seeds = [crate::state::SafetyController::SEED],
        bump,
    )]
    pub safety: Account<'info, crate::state::SafetyController>,
}

/// Initialize protocol handler
pub fn initialize_protocol(
    ctx: Context<InitializeProtocol>,
    params: InitializeProtocolParams,
) -> Result<()> {
    // Basic param validation (MVP ranges)
    require!(
        params.depeg_threshold_bps >= 25 && params.depeg_threshold_bps <= 5000,
        FeelsError::InvalidMarket
    );
    require!(
        params.depeg_required_obs > 0 && params.depeg_required_obs <= 10,
        FeelsError::InvalidMarket
    );
    require!(
        params.clear_required_obs > 0 && params.clear_required_obs <= 10,
        FeelsError::InvalidMarket
    );
    require!(
        params.dex_twap_window_secs >= 300 && params.dex_twap_window_secs <= 7200,
        FeelsError::InvalidMarket
    );
    require!(
        params.dex_twap_stale_age_secs >= params.dex_twap_window_secs,
        FeelsError::InvalidMarket
    );
    let config = &mut ctx.accounts.protocol_config;

    // Set initial configuration
    config.authority = ctx.accounts.authority.key();
    config.mint_fee = params.mint_fee;
    config.treasury = params.treasury;
    config.default_protocol_fee_rate = params.default_protocol_fee_rate.unwrap_or(1000); // Default 10%
    config.default_creator_fee_rate = params.default_creator_fee_rate.unwrap_or(500); // Default 5%
    config.max_protocol_fee_rate = params.max_protocol_fee_rate.unwrap_or(2500); // Max 25%
    config.token_expiration_seconds = 7 * 24 * 60 * 60; // 7 days default
                                                        // Oracle + safety params
    config.dex_twap_updater = params.dex_twap_updater;
    config.depeg_threshold_bps = params.depeg_threshold_bps;
    config.depeg_required_obs = params.depeg_required_obs;
    config.clear_required_obs = params.clear_required_obs;
    config.dex_twap_window_secs = params.dex_twap_window_secs;
    config.dex_twap_stale_age_secs = params.dex_twap_stale_age_secs;
    config._reserved = [0; 7];
    // Initialize DEX whitelist (truncate to fit)
    config.dex_whitelist = [Pubkey::default(); 8];
    let mut i = 0usize;
    for k in params.dex_whitelist.iter().take(8) {
        config.dex_whitelist[i] = *k;
        i += 1;
    }
    config.dex_whitelist_len = i as u8;
    // Per-slot caps default to 0 (unlimited) in MVP; adjustable via update_protocol
    config.mint_per_slot_cap_feelssol = 0;
    config.redeem_per_slot_cap_feelssol = 0;

    // Initialize protocol oracle defaults
    let oracle = &mut ctx.accounts.protocol_oracle;
    oracle.native_rate_q64 = 0;
    oracle.dex_twap_rate_q64 = 0;
    oracle.dex_last_update_slot = 0;
    oracle.native_last_update_slot = 0;
    oracle.dex_last_update_ts = 0;
    oracle.native_last_update_ts = 0;
    oracle.dex_window_secs = params.dex_twap_window_secs;
    oracle.flags = 0;

    // Initialize safety controller
    let safety = &mut ctx.accounts.safety;
    safety.redemptions_paused = false;
    safety.consecutive_breaches = 0;
    safety.consecutive_clears = 0;
    safety.last_change_slot = 0;
    safety.mint_last_slot = 0;
    safety.mint_slot_amount = 0;
    safety.redeem_last_slot = 0;
    safety.redeem_slot_amount = 0;

    msg!("Protocol initialized with:");
    msg!("  Authority: {}", config.authority);
    msg!("  Mint fee: {} FeelsSOL", config.mint_fee);
    msg!("  Treasury: {}", config.treasury);
    msg!("  Default protocol fee rate: {} bps", config.default_protocol_fee_rate);
    msg!("  Default creator fee rate: {} bps", config.default_creator_fee_rate);
    msg!("  Max protocol fee rate: {} bps", config.max_protocol_fee_rate);

    emit!(crate::events::ProtocolParamsUpdated {
        authority: config.authority,
        depeg_threshold_bps: config.depeg_threshold_bps,
        depeg_required_obs: config.depeg_required_obs,
        clear_required_obs: config.clear_required_obs,
        dex_twap_window_secs: config.dex_twap_window_secs,
        dex_twap_stale_age_secs: config.dex_twap_stale_age_secs,
        dex_twap_updater: config.dex_twap_updater,
    });

    Ok(())
}

/// Update protocol configuration parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UpdateProtocolParams {
    /// New mint fee (None to keep current)
    pub mint_fee: Option<u64>,
    /// New treasury (None to keep current)
    pub treasury: Option<Pubkey>,
    /// New authority (None to keep current)
    pub authority: Option<Pubkey>,
    /// New default protocol fee rate (None to keep current)
    pub default_protocol_fee_rate: Option<u16>,
    /// New default creator fee rate (None to keep current)
    pub default_creator_fee_rate: Option<u16>,
    /// New max protocol fee rate (None to keep current)
    pub max_protocol_fee_rate: Option<u16>,
    /// Optional: DEX TWAP updater
    pub dex_twap_updater: Option<Pubkey>,
    /// Optional: safety thresholds
    pub depeg_threshold_bps: Option<u16>,
    pub depeg_required_obs: Option<u8>,
    pub clear_required_obs: Option<u8>,
    /// Optional: TWAP timing params
    pub dex_twap_window_secs: Option<u32>,
    pub dex_twap_stale_age_secs: Option<u32>,
    /// Replace DEX whitelist (set)
    pub dex_whitelist: Option<Vec<Pubkey>>,
    /// Optional: per-slot caps
    pub mint_per_slot_cap_feelssol: Option<u64>,
    pub redeem_per_slot_cap_feelssol: Option<u64>,
}

/// Update protocol accounts
#[derive(Accounts)]
pub struct UpdateProtocol<'info> {
    /// Current protocol authority
    #[account(
        mut,
        constraint = authority.key() == protocol_config.authority @ FeelsError::UnauthorizedSigner
    )]
    pub authority: Signer<'info>,

    /// Protocol config account
    #[account(
        mut,
        seeds = [ProtocolConfig::SEED],
        bump,
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
}

/// Update protocol handler
pub fn update_protocol(ctx: Context<UpdateProtocol>, params: UpdateProtocolParams) -> Result<()> {
    let config = &mut ctx.accounts.protocol_config;

    // Update parameters if provided
    if let Some(mint_fee) = params.mint_fee {
        config.mint_fee = mint_fee;
        msg!("Updated mint fee to: {} FeelsSOL", mint_fee);
    }

    if let Some(treasury) = params.treasury {
        config.treasury = treasury;
        msg!("Updated treasury to: {}", treasury);
    }

    if let Some(protocol_fee_rate) = params.default_protocol_fee_rate {
        require!(
            protocol_fee_rate <= config.max_protocol_fee_rate,
            FeelsError::InvalidMarket
        );
        config.default_protocol_fee_rate = protocol_fee_rate;
        msg!("Updated default protocol fee rate to: {} bps", protocol_fee_rate);
    }

    if let Some(creator_fee_rate) = params.default_creator_fee_rate {
        require!(creator_fee_rate <= 1000, FeelsError::InvalidMarket); // Max 10% creator fee
        config.default_creator_fee_rate = creator_fee_rate;
        msg!("Updated default creator fee rate to: {} bps", creator_fee_rate);
    }

    if let Some(max_fee_rate) = params.max_protocol_fee_rate {
        require!(max_fee_rate <= 5000, FeelsError::InvalidMarket); // Max 50% total protocol fee
        require!(
            max_fee_rate >= config.default_protocol_fee_rate,
            FeelsError::InvalidMarket
        );
        config.max_protocol_fee_rate = max_fee_rate;
        msg!("Updated max protocol fee rate to: {} bps", max_fee_rate);
    }

    if let Some(authority) = params.authority {
        config.authority = authority;
        msg!("Updated authority to: {}", authority);
    }
    if let Some(k) = params.dex_twap_updater {
        config.dex_twap_updater = k;
    }
    if let Some(x) = params.depeg_threshold_bps {
        require!((25..=5000).contains(&x), FeelsError::InvalidMarket);
        config.depeg_threshold_bps = x;
    }
    if let Some(x) = params.depeg_required_obs {
        require!(x > 0 && x <= 10, FeelsError::InvalidMarket);
        config.depeg_required_obs = x;
    }
    if let Some(x) = params.clear_required_obs {
        require!(x > 0 && x <= 10, FeelsError::InvalidMarket);
        config.clear_required_obs = x;
    }
    if let Some(x) = params.dex_twap_window_secs {
        require!((300..=7200).contains(&x), FeelsError::InvalidMarket);
        config.dex_twap_window_secs = x;
    }
    if let Some(x) = params.dex_twap_stale_age_secs {
        require!(x >= config.dex_twap_window_secs, FeelsError::InvalidMarket);
        config.dex_twap_stale_age_secs = x;
    }
    if let Some(list) = params.dex_whitelist.as_ref() {
        config.dex_whitelist = [Pubkey::default(); 8];
        let mut i = 0usize;
        for k in list.iter().take(8) {
            config.dex_whitelist[i] = *k;
            i += 1;
        }
        config.dex_whitelist_len = i as u8;
    }
    if let Some(x) = params.mint_per_slot_cap_feelssol {
        config.mint_per_slot_cap_feelssol = x;
    }
    if let Some(x) = params.redeem_per_slot_cap_feelssol {
        config.redeem_per_slot_cap_feelssol = x;
    }

    emit!(crate::events::ProtocolParamsUpdated {
        authority: config.authority,
        depeg_threshold_bps: config.depeg_threshold_bps,
        depeg_required_obs: config.depeg_required_obs,
        clear_required_obs: config.clear_required_obs,
        dex_twap_window_secs: config.dex_twap_window_secs,
        dex_twap_stale_age_secs: config.dex_twap_stale_age_secs,
        dex_twap_updater: config.dex_twap_updater,
    });

    Ok(())
}
