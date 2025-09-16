//! Epoch parameters
//!
//! Frozen parameters per epoch for deterministic pricing

use anchor_lang::prelude::*;

/// Epoch parameters PDA - frozen values for deterministic swaps
#[account]
pub struct EpochParams {
    /// Associated market
    pub market: Pubkey,

    /// Current epoch number
    pub epoch_number: u64,

    /// Epoch start timestamp
    pub epoch_start: i64,

    /// Epoch length in seconds
    pub epoch_length: i64,

    /// Frozen parameters (empty for MVP, will be populated in Phase 2+)
    pub lambda_s: u16, // λ_s coefficient (0 for MVP)
    pub lambda_t: u16, // λ_t coefficient (0 for MVP)
    pub lambda_l: u16, // λ_l coefficient (0 for MVP)

    pub weight_s: u16, // w_s domain weight (10000 = 100% for MVP)
    pub weight_t: u16, // w_t domain weight (0 for MVP)
    pub weight_l: u16, // w_l domain weight (0 for MVP)

    /// Reserved space for future parameters
    pub _reserved: [u8; 32],
}

impl EpochParams {
    pub const LEN: usize = 8 + // discriminator
        32 + // market
        8 + // epoch_number
        8 + // epoch_start
        8 + // epoch_length
        2 + // lambda_s
        2 + // lambda_t
        2 + // lambda_l
        2 + // weight_s
        2 + // weight_t
        2 + // weight_l
        32 + // _reserved
        4; // padding added by Rust compiler for alignment

    /// Seeds for PDA derivation
    pub fn seeds(market: &Pubkey) -> Vec<Vec<u8>> {
        vec![b"epoch_params".to_vec(), market.to_bytes().to_vec()]
    }

    /// Check if epoch is expired
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp >= self.epoch_start + self.epoch_length
    }

    /// Default values for MVP
    pub fn default_mvp(market: Pubkey, epoch_number: u64, current_timestamp: i64) -> Self {
        Self {
            market,
            epoch_number,
            epoch_start: current_timestamp,
            epoch_length: 3600, // 1 hour epochs
            lambda_s: 0,        // No dynamic fees in MVP
            lambda_t: 0,
            lambda_l: 0,
            weight_s: 10000, // 100% spot weight
            weight_t: 0,
            weight_l: 0,
            _reserved: [0; 32],
        }
    }
}
