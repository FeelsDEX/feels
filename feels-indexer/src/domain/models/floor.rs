//! Floor liquidity domain models

use crate::core::BlockInfo;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Indexed floor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFloor {
    pub address: Pubkey,
    pub market: Pubkey,
    pub current_floor: i32,
    pub floor_buffer: i32,
    pub last_ratchet_slot: u64,
    pub jitosol_reserves: u128,
    pub total_feels_supply: u128,
    pub last_updated: BlockInfo,
}

impl IndexedFloor {
    /// Calculate the actual floor price
    pub fn floor_price(&self) -> f64 {
        if self.total_feels_supply == 0 {
            return 0.0;
        }
        self.jitosol_reserves as f64 / self.total_feels_supply as f64
    }
}

