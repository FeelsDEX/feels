//! Core domain types

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::fmt;

/// Block information context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    
    pub fn with_timestamp(slot: u64, timestamp: i64) -> Self {
        Self {
            slot,
            timestamp,
            block_height: None,
        }
    }
}

/// Processing context for account updates
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub block_info: BlockInfo,
    pub signature: Option<String>,
}

impl ProcessContext {
    pub fn new(block_info: BlockInfo) -> Self {
        Self {
            block_info,
            signature: None,
        }
    }
    
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }
    
    #[cfg(test)]
    pub fn test() -> Self {
        Self {
            block_info: BlockInfo::new(0),
            signature: None,
        }
    }
}

/// Update type for tracking changes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateType {
    Created,
    Updated,
    Deleted,
}

impl fmt::Display for UpdateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateType::Created => write!(f, "created"),
            UpdateType::Updated => write!(f, "updated"),
            UpdateType::Deleted => write!(f, "deleted"),
        }
    }
}

/// Generic update record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord<T> {
    pub data: T,
    pub update_type: UpdateType,
    pub block_info: BlockInfo,
    pub signature: Option<String>,
}

impl<T> UpdateRecord<T> {
    pub fn created(data: T, block_info: BlockInfo) -> Self {
        Self {
            data,
            update_type: UpdateType::Created,
            block_info,
            signature: None,
        }
    }
    
    pub fn updated(data: T, block_info: BlockInfo) -> Self {
        Self {
            data,
            update_type: UpdateType::Updated,
            block_info,
            signature: None,
        }
    }
    
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }
}

/// Market query parameters
#[derive(Debug, Clone, Default)]
pub struct MarketQuery {
    pub token_0: Option<Pubkey>,
    pub token_1: Option<Pubkey>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl MarketQuery {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_token_0(mut self, token: Pubkey) -> Self {
        self.token_0 = Some(token);
        self
    }
    
    pub fn with_token_1(mut self, token: Pubkey) -> Self {
        self.token_1 = Some(token);
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Account stream item
pub type AccountUpdate = (Pubkey, Vec<u8>, BlockInfo);

/// Transaction stream item  
pub type TransactionUpdate = (String, Vec<u8>, BlockInfo);

