//! Market state for MVP
//!
//! Minimal market structure with spot AMM only

use crate::state::{TokenOrigin, TokenType};
use anchor_lang::prelude::*;

// Oracle types moved to separate oracle.rs module

/// Feature flags for future phases (all OFF in MVP)
#[derive(Clone, Copy, Debug, PartialEq, Default, AnchorSerialize, AnchorDeserialize)]
pub struct FeatureFlags {
    pub dynamic_fees: bool,
    pub precision_mode: bool,
    pub autopilot_lambda: bool,
    pub autopilot_weights: bool,
    pub targets_adaptive: bool,
    pub time_domain: bool,
    pub leverage_domain: bool,
    pub _reserved: [bool; 9], // Reserved for future flags
}

/// Policy configuration (minimal for MVP)
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PolicyV1 {
    pub version: u8,
    pub feature_flags: FeatureFlags,
    pub base_fee_bps: u16,              // Base fee in basis points
    pub max_surcharge_bps: u16,         // For future use
    pub max_instantaneous_fee_bps: u16, // For future use
    pub _reserved: [u8; 4],             // Minimal reserved space
}

impl Default for PolicyV1 {
    fn default() -> Self {
        Self {
            version: 1,
            feature_flags: FeatureFlags::default(),
            base_fee_bps: 30, // 0.30% default base fee
            max_surcharge_bps: 0,
            max_instantaneous_fee_bps: 0,
            _reserved: [0; 4],
        }
    }
}

/// Main market account
#[account]
pub struct Market {
    /// Version for upgradability
    pub version: u8,

    /// Market status
    pub is_initialized: bool,
    pub is_paused: bool,

    /// Token configuration
    pub token_0: Pubkey, // First token mint
    pub token_1: Pubkey,       // Second token mint
    pub feelssol_mint: Pubkey, // Hub token mint

    /// Token types (for future Token-2022 support)
    pub token_0_type: TokenType,
    pub token_1_type: TokenType,

    /// Token origins (for market creation restrictions)
    pub token_0_origin: TokenOrigin,
    pub token_1_origin: TokenOrigin,

    /// Spot AMM state (simplified constant product for MVP)
    pub sqrt_price: u128, // Q64 sqrt(price = token1/token0)
    pub liquidity: u128, // Active liquidity (inside range)

    /// CLMM tick state
    pub current_tick: i32,
    pub tick_spacing: u16,

    /// Floor liquidity bounds (TEMPORARY - will be removed when POMM uses pure positions)
    /// These currently serve as bounds for pool-owned liquidity but will be
    /// replaced with actual position NFTs in a future upgrade.
    /// Global swap bounds - hard limits for all swaps in this market
    pub global_lower_tick: i32,
    pub global_upper_tick: i32,
    /// Liquidity at the global bounds (legacy POMM field, kept for compatibility)
    pub floor_liquidity: u128,

    /// Global fee growth (Q64) per liquidity unit
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,

    /// Fee configuration
    pub base_fee_bps: u16, // Base fee only for MVP

    /// Buffer (τ) reference
    pub buffer: Pubkey,

    /// Authority
    pub authority: Pubkey,

    /// Epoch tracking
    pub last_epoch_update: i64,
    pub epoch_number: u64,

    /// Oracle account reference
    /// Oracle data is stored in a separate account to reduce stack usage
    pub oracle: Pubkey,

    /// Oracle account bump seed
    pub oracle_bump: u8,

    /// Policy configuration
    pub policy: PolicyV1,

    /// Canonical bump for market authority PDA
    /// SECURITY: Storing prevents recomputation and ensures consistency
    pub market_authority_bump: u8,

    /// Canonical bumps for vault PDAs
    pub vault_0_bump: u8,
    pub vault_1_bump: u8,

    /// Re-entrancy guard
    /// SECURITY: Set to true at the start of sensitive operations and false at the end
    /// Prevents re-entrant calls during critical state transitions
    pub reentrancy_guard: bool,

    /// Initial liquidity deployment status
    pub initial_liquidity_deployed: bool,

    /// JIT v0 feature flag (per-market)
    pub jit_enabled: bool,
    /// JIT budget caps (bps of Buffer.tau_spot)
    pub jit_per_swap_q_bps: u16,
    pub jit_per_slot_q_bps: u16,

    /// Floor management (MVP)
    pub floor_tick: i32,
    pub floor_buffer_ticks: i32,
    pub last_floor_ratchet_ts: i64,
    pub floor_cooldown_secs: i64,

    /// Graduation flags (idempotent)
    pub steady_state_seeded: bool,
    pub cleanup_complete: bool,

    /// Reserved space for future expansion
    pub _reserved: [u8; 31], // Extra space for future upgrades - reduced by 1 for initial_liquidity_deployed
}

impl Market {
    pub const LEN: usize = 8 + // discriminator
        1 + // version
        1 + // is_initialized
        1 + // is_paused
        32 + // token_0
        32 + // token_1
        32 + // feelssol_mint
        1 + // token_0_type
        1 + // token_1_type
        1 + // token_0_origin
        1 + // token_1_origin
        16 + // sqrt_price
        16 + // liquidity
        4 +  // current_tick (i32)
        2 +  // tick_spacing (u16)
        4 +  // global_lower_tick
        4 +  // global_upper_tick
        16 + // floor_liquidity
        16 + // fee_growth_global_0_x64
        16 + // fee_growth_global_1_x64
        2 + // base_fee_bps
        32 + // buffer
        32 + // authority
        8 + // last_epoch_update
        8 + // epoch_number
        32 + // oracle
        1 + // oracle_bump
        (1 + 16 + 2 + 2 + 2 + 4) + // PolicyV1 (minimal reserved and feature flags)
        1 + // market_authority_bump
        1 + // vault_0_bump
        1 + // vault_1_bump
        1 + // reentrancy_guard
        1 + // initial_liquidity_deployed
        1 + // jit_enabled
        2 + // jit_per_swap_q_bps
        2 + // jit_per_slot_q_bps
        4 + // floor_tick
        4 + // floor_buffer_ticks
        8 + // last_floor_ratchet_ts
        8 + // floor_cooldown_secs
        1 + // steady_state_seeded
        1 + // cleanup_complete
        31; // _reserved

    /// Get the current tick from sqrt_price
    pub fn get_current_tick(&self) -> i32 {
        self.current_tick
    }

    /// Check if we're past the epoch boundary
    pub fn epoch_due(&self, current_timestamp: i64) -> bool {
        const EPOCH_LENGTH: i64 = 3600; // 1 hour epochs for MVP
        current_timestamp - self.last_epoch_update >= EPOCH_LENGTH
    }

    /// Derive vault address for a given mint
    /// SECURITY: This ensures vaults are always derived deterministically and cannot be spoofed
    /// Note: market_key must be passed in since self doesn't have access to its own key
    pub fn derive_vault_address_with_key(
        &self,
        market_key: &Pubkey,
        mint: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        crate::utils::derive_vault(market_key, mint, program_id)
    }

    /// Static method to derive vault address without Market instance
    pub fn derive_vault_address(
        market_key: &Pubkey,
        mint: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        crate::utils::derive_vault(market_key, mint, program_id)
    }

    /// Get the derived vault addresses for token_0 and token_1
    /// Note: market_key must be passed in since self doesn't have access to its own key
    pub fn get_vault_addresses(
        &self,
        market_key: &Pubkey,
        program_id: &Pubkey,
    ) -> ((Pubkey, u8), (Pubkey, u8)) {
        let vault_0 = self.derive_vault_address_with_key(market_key, &self.token_0, program_id);
        let vault_1 = self.derive_vault_address_with_key(market_key, &self.token_1, program_id);
        (vault_0, vault_1)
    }

    /// Derive the unified market authority address
    /// SECURITY: Single authority for all market operations
    /// Note: market_key must be passed in since self doesn't have access to its own key
    pub fn derive_market_authority_with_key(
        &self,
        market_key: &Pubkey,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        crate::utils::derive_market_authority(market_key, program_id)
    }

    /// Static method to derive market authority without Market instance
    pub fn derive_market_authority(market_key: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        crate::utils::derive_market_authority(market_key, program_id)
    }
}
