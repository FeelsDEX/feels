/// Field commitment data structures and utilities for SDK.
/// Uses types from feels_types crate to avoid duplication.

// Re-export from shared types
pub use feels_types::{FieldCommitmentData, LocalCoefficients};

impl FieldCommitmentData {
    /// Check if commitment is still valid
    pub fn is_valid(&self, current_time: i64) -> bool {
        let age = current_time - self.snapshot_ts;
        age <= self.max_staleness
    }
    
    /// Convert to market field data for work calculator
    pub fn to_market_field_data(&self) -> feels_types::MarketFieldData {
        feels_types::MarketFieldData {
            S: self.S,
            T: self.T,
            L: self.L,
            w_s: self.w_s,
            w_t: self.w_t,
            w_l: self.w_l,
            w_tau: self.w_tau,
            sigma_price: self.sigma_price,
            sigma_rate: self.sigma_rate,
            sigma_leverage: self.sigma_leverage,
            twap_0: self.twap_0,
            twap_1: self.twap_1,
        }
    }
}