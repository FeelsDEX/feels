//! Buffer domain models

use crate::core::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed buffer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBuffer {
    pub address: Pubkey,
    pub market: Pubkey,
    pub tau_spot: u128,
    pub total_fees_0: u128,
    pub total_fees_1: u128,
    pub last_updated: BlockInfo,
}

