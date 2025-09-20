//! Account and transaction processors for Feels Protocol

mod registry;
mod market;
mod swap;
mod buffer;
mod position;
mod floor;

pub use registry::*;
pub use market::*;
pub use swap::*;
pub use buffer::*;
pub use position::*;
pub use floor::*;

use crate::models::BlockInfo;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;

/// Trait for processing account updates
#[async_trait::async_trait]
pub trait AccountProcessor: Send + Sync {
    /// Process an account update
    async fn process_account_update(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()>;
}

/// Trait for processing transactions
#[async_trait::async_trait]
pub trait TransactionProcessor: Send + Sync {
    /// Process a transaction
    async fn process_transaction(
        &self,
        signature: String,
        transaction: &solana_transaction_status::EncodedTransaction,
        block_info: BlockInfo,
    ) -> Result<()>;
}
