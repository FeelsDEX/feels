//! Protocol configuration state
//!
//! Global protocol parameters that can be updated by governance

use anchor_lang::prelude::*;

/// Protocol configuration account
#[account]
pub struct ProtocolConfig {
    /// Authority that can update protocol parameters
    pub authority: Pubkey,

    /// Fee for minting a new token (in FeelsSOL lamports)
    pub mint_fee: u64,

    /// Treasury account to receive protocol fees
    pub treasury: Pubkey,

    /// Default protocol fee rate (basis points, e.g. 1000 = 10%)
    pub default_protocol_fee_rate: u16,
    /// Default creator fee rate for protocol tokens (basis points, e.g. 500 = 5%)
    pub default_creator_fee_rate: u16,
    /// Maximum allowed protocol fee rate (basis points)
    pub max_protocol_fee_rate: u16,

    /// Time window (in seconds) for deploying liquidity after token mint
    /// If liquidity isn't deployed within this window, token can be destroyed
    pub token_expiration_seconds: i64,

    /// De-peg circuit breaker threshold (bps of divergence)
    pub depeg_threshold_bps: u16,
    /// Required consecutive breach observations to pause
    pub depeg_required_obs: u8,
    /// Required consecutive clear observations to resume
    pub clear_required_obs: u8,
    /// DEX TWAP window and staleness thresholds (seconds)
    pub dex_twap_window_secs: u32,
    pub dex_twap_stale_age_secs: u32,
    /// Authorized updater for DEX TWAP feed (MVP single updater)
    pub dex_twap_updater: Pubkey,
    /// DEX whitelist (venues/pools) - fixed size for MVP
    pub dex_whitelist: [Pubkey; 8],
    pub dex_whitelist_len: u8,
    /// Reserved for future protocol parameters
    pub _reserved: [u8; 7],
    /// Optional per-slot caps for mint/redeem (FeelsSOL units). 0 = unlimited.
    pub mint_per_slot_cap_feelssol: u64,
    pub redeem_per_slot_cap_feelssol: u64,

    /// Default AMM parameters (admin-controlled during memecoin phase)
    /// Default base fee in basis points (e.g., 30 = 0.3%)
    pub default_base_fee_bps: u16,
    /// Default tick spacing for new markets
    pub default_tick_spacing: u16,
    /// Default initial sqrt price for new markets (Q64.64 format)
    pub default_initial_sqrt_price: u128,
    /// Default tick step size for bonding curve deployment
    pub default_tick_step_size: u16,
}

impl ProtocolConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        8 +  // mint_fee
        32 + // treasury
        2 +  // default_protocol_fee_rate
        2 +  // default_creator_fee_rate
        2 +  // max_protocol_fee_rate
        8 +  // token_expiration_seconds
        2 +  // depeg_threshold_bps
        1 +  // depeg_required_obs
        1 +  // clear_required_obs
        4 +  // dex_twap_window_secs
        4 +  // dex_twap_stale_age_secs
        32 + // dex_twap_updater
        (32*8) + // dex_whitelist
        1 + // dex_whitelist_len
        7 +  // _reserved
        8 +  // mint_per_slot_cap_feelssol
        8 + // redeem_per_slot_cap_feelssol
        2 +  // default_base_fee_bps
        2 +  // default_tick_spacing
        16 + // default_initial_sqrt_price (u128)
        2; // default_tick_step_size

    /// Seed for deriving the protocol config PDA
    pub const SEED: &'static [u8] = b"protocol_config";
}
