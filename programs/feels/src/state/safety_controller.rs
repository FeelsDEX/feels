//! Protocol Safety Controller (MVP)
//!
//! Tracks de-peg observations and pauses redemptions when criteria are met.

use crate::events::{
    CircuitBreakerActivated, RedemptionsPaused, RedemptionsResumed, SafetyPaused, SafetyResumed,
};
use crate::state::{ProtocolConfig, ProtocolOracle};
use anchor_lang::prelude::*;

#[account]
pub struct SafetyController {
    /// Whether redemptions are paused due to de-peg
    pub redemptions_paused: bool,
    /// Consecutive divergence observations over threshold
    pub consecutive_breaches: u8,
    /// Consecutive safe observations since last breach
    pub consecutive_clears: u8,
    /// Last state change slot
    pub last_change_slot: u64,
    /// Per-slot mint tracking (FeelsSOL units)
    pub mint_last_slot: u64,
    pub mint_slot_amount: u64,
    /// Per-slot redeem tracking (FeelsSOL units)
    pub redeem_last_slot: u64,
    pub redeem_slot_amount: u64,
    /// Last slot when divergence was checked (prevents double-counting)
    pub last_divergence_check_slot: u64,
    /// Degraded mode flags
    pub degrade_flags: DegradeFlags,
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

/// Degraded mode flags for various safety conditions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct DegradeFlags {
    /// GTWAP stale: disable advanced features
    pub gtwap_stale: bool,
    /// Protocol oracle stale: pause exits
    pub oracle_stale: bool,
    /// High volatility detected: raise minimum fees
    pub high_volatility: bool,
    /// Low liquidity: restrict large trades
    pub low_liquidity: bool,
    /// Reserved flags for future use
    pub _reserved: [bool; 4],
}

impl SafetyController {
    pub const SEED: &'static [u8] = b"safety_controller";
    pub const LEN: usize = 8 + // disc
        1 + // redemptions_paused
        1 + // consecutive_breaches
        1 + // consecutive_clears
        8 +  // last_change_slot
        8 +  // mint_last_slot
        8 +  // mint_slot_amount
        8 +  // redeem_last_slot
        8 + // redeem_slot_amount
        8 + // last_divergence_check_slot
        8 + // degrade_flags (1 + 1 + 1 + 1 + 4)
        32 + // _reserved
        5; // padding added by Rust compiler for alignment

    /// Centralized divergence check and state update
    /// This should only be called from oracle update instructions, not from exit paths
    pub fn check_and_update_divergence(
        &mut self,
        oracle: &ProtocolOracle,
        config: &ProtocolConfig,
        current_slot: u64,
        current_ts: i64,
    ) -> Result<bool> {
        // Prevent double-counting within the same slot
        if self.last_divergence_check_slot == current_slot {
            return Ok(self.redemptions_paused);
        }
        self.last_divergence_check_slot = current_slot;

        // Check if DEX TWAP is fresh
        let dex_stale = if oracle.dex_last_update_ts > 0 {
            (current_ts - oracle.dex_last_update_ts) > config.dex_twap_stale_age_secs as i64
        } else {
            true
        };

        // Only check divergence if both rates are set and DEX is fresh
        if !dex_stale && oracle.native_rate_q64 > 0 && oracle.dex_twap_rate_q64 > 0 {
            let div_bps = compute_divergence_bps(oracle.native_rate_q64, oracle.dex_twap_rate_q64);

            if div_bps >= config.depeg_threshold_bps {
                // Breach detected
                self.consecutive_breaches = self.consecutive_breaches.saturating_add(1);
                self.consecutive_clears = 0;

                // Check if we should pause
                if !self.redemptions_paused
                    && self.consecutive_breaches >= config.depeg_required_obs
                {
                    self.redemptions_paused = true;
                    self.last_change_slot = current_slot;

                    // Emit events
                    emit!(CircuitBreakerActivated {
                        threshold_bps: config.depeg_threshold_bps,
                        window_secs: oracle.dex_window_secs
                    });
                    emit!(RedemptionsPaused {
                        timestamp: current_ts
                    });
                    emit!(SafetyPaused {
                        timestamp: current_ts
                    });
                }
            } else {
                // Clear detected
                self.consecutive_clears = self.consecutive_clears.saturating_add(1);
                self.consecutive_breaches = 0;

                // Check if we should resume
                if self.redemptions_paused && self.consecutive_clears >= config.clear_required_obs {
                    self.redemptions_paused = false;
                    self.last_change_slot = current_slot;

                    // Emit events
                    emit!(RedemptionsResumed {
                        timestamp: current_ts
                    });
                    emit!(SafetyResumed {
                        timestamp: current_ts
                    });
                }
            }
        }

        Ok(self.redemptions_paused)
    }

    /// Check if redemptions are allowed (read-only, for exit paths)
    pub fn check_redemptions_allowed(
        &self,
        oracle: &ProtocolOracle,
        config: &ProtocolConfig,
        current_ts: i64,
    ) -> Result<()> {
        // First check if already paused
        if self.redemptions_paused {
            return Err(crate::error::FeelsError::MarketPaused.into());
        }

        // Check if either oracle is stale
        let dex_stale = oracle.is_dex_oracle_stale(current_ts, config.dex_twap_stale_age_secs);
        let native_stale =
            oracle.is_native_oracle_stale(current_ts, config.dex_twap_stale_age_secs);

        // If either oracle that contributes to pricing is stale, reject the redemption
        if (oracle.dex_twap_rate_q64 > 0 && dex_stale)
            || (oracle.native_rate_q64 > 0 && native_stale)
        {
            msg!(
                "Oracle data is stale - redemptions blocked. DEX stale: {}, Native stale: {}",
                dex_stale,
                native_stale
            );
            msg!("  Current timestamp: {}", current_ts);
            msg!("  DEX last update: {}", oracle.dex_last_update_ts);
            msg!("  Native last update: {}", oracle.native_last_update_ts);
            msg!("  Max age seconds: {}", config.dex_twap_stale_age_secs);
            msg!("  DEX rate q64: {}", oracle.dex_twap_rate_q64);
            msg!("  Native rate q64: {}", oracle.native_rate_q64);
            return Err(crate::error::FeelsError::OracleStale.into());
        }

        Ok(())
    }

    /// Update degrade matrix based on market conditions
    pub fn update_degrade_matrix(
        &mut self,
        gtwap_age_seconds: u32,
        oracle_age_seconds: u32,
        volatility_score: u16, // bps of price movement
        liquidity_depth_usd: u64,
        config: &ProtocolConfig,
    ) -> Result<()> {
        let mut changed = false;

        // GTWAP staleness check
        let gtwap_stale_threshold = 300; // 5 minutes
        let new_gtwap_stale = gtwap_age_seconds > gtwap_stale_threshold;
        if self.degrade_flags.gtwap_stale != new_gtwap_stale {
            self.degrade_flags.gtwap_stale = new_gtwap_stale;
            changed = true;
        }

        // Protocol oracle staleness
        let oracle_stale_threshold = config.dex_twap_stale_age_secs;
        let new_oracle_stale = oracle_age_seconds > oracle_stale_threshold;
        if self.degrade_flags.oracle_stale != new_oracle_stale {
            self.degrade_flags.oracle_stale = new_oracle_stale;
            changed = true;
        }

        // High volatility detection (>5% in window)
        let volatility_threshold = 500; // 5%
        let new_high_volatility = volatility_score > volatility_threshold;
        if self.degrade_flags.high_volatility != new_high_volatility {
            self.degrade_flags.high_volatility = new_high_volatility;
            changed = true;
        }

        // Low liquidity detection
        let liquidity_threshold = 100_000; // $100k USD
        let new_low_liquidity = liquidity_depth_usd < liquidity_threshold;
        if self.degrade_flags.low_liquidity != new_low_liquidity {
            self.degrade_flags.low_liquidity = new_low_liquidity;
            changed = true;
        }

        if changed {
            emit!(crate::events::SafetyDegradeMatrixUpdated {
                gtwap_stale: self.degrade_flags.gtwap_stale,
                oracle_stale: self.degrade_flags.oracle_stale,
                high_volatility: self.degrade_flags.high_volatility,
                low_liquidity: self.degrade_flags.low_liquidity,
                timestamp: Clock::get()?.unix_timestamp,
            });
        }

        Ok(())
    }

    /// Get adjusted minimum fee based on degrade matrix
    pub fn get_adjusted_min_fee_bps(&self, base_min_fee_bps: u16) -> u16 {
        let mut min_fee = base_min_fee_bps;

        // Increase minimum fee under degraded conditions
        if self.degrade_flags.high_volatility {
            min_fee = min_fee.saturating_add(20); // +0.2%
        }
        if self.degrade_flags.low_liquidity {
            min_fee = min_fee.saturating_add(10); // +0.1%
        }
        if self.degrade_flags.gtwap_stale {
            min_fee = min_fee.saturating_add(5); // +0.05%
        }

        min_fee
    }

    /// Check if advanced features should be disabled
    pub fn should_disable_advanced_features(&self) -> bool {
        self.degrade_flags.gtwap_stale
            || self.degrade_flags.oracle_stale
            || self.degrade_flags.high_volatility
    }

    /// Check if large trades should be restricted
    pub fn should_restrict_large_trades(&self) -> bool {
        self.degrade_flags.low_liquidity || self.degrade_flags.high_volatility
    }
}

/// Compute divergence in basis points between native and DEX TWAP
/// Uses consistent formula: |native - dex| / min(native, dex) * 10000
pub fn compute_divergence_bps(native_q64: u128, dex_q64: u128) -> u16 {
    if native_q64 == 0 || dex_q64 == 0 {
        return 0;
    }

    let (max_rate, min_rate) = if native_q64 > dex_q64 {
        (native_q64, dex_q64)
    } else {
        (dex_q64, native_q64)
    };

    let diff = max_rate - min_rate;
    ((diff.saturating_mul(10_000)) / min_rate).min(u16::MAX as u128) as u16
}
