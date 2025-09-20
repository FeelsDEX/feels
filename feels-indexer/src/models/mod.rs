//! Data models for indexed Feels Protocol state

pub mod market;
pub mod swap;
pub mod floor;
pub mod buffer;
pub mod position;

pub use market::*;
pub use floor::*;
pub use position::*;

use serde::{Deserialize, Serialize};

/// Pool phase enum matching the protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolPhase {
    PriceDiscovery,
    SteadyState,
}

/// Common timestamp and slot tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub slot: u64,
    pub timestamp: i64,
    pub block_height: Option<u64>,
}

impl BlockInfo {
    pub fn new(slot: u64) -> Self {
        Self {
            slot,
            timestamp: chrono::Utc::now().timestamp(),
            block_height: None,
        }
    }
}

/// Update type for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Created,
    Updated,
    Deleted,
}

/// Generic update record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord<T> {
    pub data: T,
    pub update_type: UpdateType,
    pub block_info: BlockInfo,
    pub signature: Option<String>,
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub signature: String,
    pub slot: u64,
    pub timestamp: i64,
    pub block_height: Option<u64>,
}

/// Slot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    pub slot: u64,
    pub parent_slot: u64,
    pub block_height: Option<u64>,
    pub timestamp: i64,
    pub blockhash: String,
}
