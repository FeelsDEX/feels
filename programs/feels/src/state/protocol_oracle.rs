//! Protocol-level oracle state (MVP)
//!
//! Stores native reserve rate and a filtered DEX TWAP. The effective
//! protocol rate is min(native, dex_twap). A safety controller monitors
//! divergence and may pause redemptions.

use anchor_lang::prelude::*;

#[account]
pub struct ProtocolOracle {
    /// Native reserve rate (Q64)
    pub native_rate_q64: u128,
    /// Filtered DEX TWAP rate (Q64)
    pub dex_twap_rate_q64: u128,
    /// Last update slot for DEX TWAP
    pub dex_last_update_slot: u64,
    /// Last update slot for native rate
    pub native_last_update_slot: u64,
    /// Last update timestamp for DEX TWAP
    pub dex_last_update_ts: i64,
    /// Last update timestamp for native rate
    pub native_last_update_ts: i64,
    /// Observation window (seconds) for DEX TWAP
    pub dex_window_secs: u32,
    /// Current flags (bitmask)
    pub flags: u32,
}

impl ProtocolOracle {
    pub const SEED: &'static [u8] = b"protocol_oracle";
    pub const LEN: usize = 8 + // disc
        16 + // native_rate_q64
        16 + // dex_twap_rate_q64
        8 +  // dex_last_update_slot
        8 +  // native_last_update_slot
        8 +  // dex_last_update_ts
        8 +  // native_last_update_ts
        4 +  // dex_window_secs
        4 + // flags
        8; // padding added by Rust compiler for alignment

    #[inline]
    pub fn min_rate_q64(&self) -> u128 {
        if self.dex_twap_rate_q64 == 0 {
            return self.native_rate_q64;
        }
        if self.native_rate_q64 == 0 {
            return self.dex_twap_rate_q64;
        }
        self.native_rate_q64.min(self.dex_twap_rate_q64)
    }

    /// Check if the DEX TWAP oracle is stale
    pub fn is_dex_oracle_stale(&self, current_ts: i64, max_age_secs: u32) -> bool {
        if self.dex_last_update_ts == 0 {
            // Never updated, consider stale
            return true;
        }
        (current_ts - self.dex_last_update_ts) > max_age_secs as i64
    }

    /// Check if the native oracle is stale
    pub fn is_native_oracle_stale(&self, current_ts: i64, max_age_secs: u32) -> bool {
        if self.native_last_update_ts == 0 {
            // Never updated, consider stale
            return true;
        }
        (current_ts - self.native_last_update_ts) > max_age_secs as i64
    }

    /// Get the minimum rate only if oracles are fresh
    /// Returns None if either oracle is stale
    pub fn min_rate_q64_checked(&self, current_ts: i64, max_age_secs: u32) -> Option<u128> {
        // If either oracle that contributes to min_rate is stale, return None
        if self.dex_twap_rate_q64 > 0 && self.is_dex_oracle_stale(current_ts, max_age_secs) {
            return None;
        }
        if self.native_rate_q64 > 0 && self.is_native_oracle_stale(current_ts, max_age_secs) {
            return None;
        }

        // Both oracles are fresh, return the minimum
        Some(self.min_rate_q64())
    }
}
