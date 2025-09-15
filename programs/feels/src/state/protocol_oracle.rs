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
        4; // flags

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
}
