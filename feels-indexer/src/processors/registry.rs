//! Processor registry for routing updates to appropriate handlers

use super::{
    AccountProcessor, TransactionProcessor,
    MarketProcessor, SwapProcessor, BufferProcessor, 
    PositionProcessor, FloorProcessor,
};
use crate::database::DatabaseManager;
use crate::models::BlockInfo;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use solana_transaction_status::{ConfirmedBlock, EncodedTransaction};
use chrono;

/// Registry for all processors
pub struct ProcessorRegistry {
    market_processor: MarketProcessor,
    swap_processor: SwapProcessor,
    buffer_processor: BufferProcessor,
    position_processor: PositionProcessor,
    floor_processor: FloorProcessor,
}

impl ProcessorRegistry {
    /// Create a new processor registry
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            market_processor: MarketProcessor::new(db_manager.clone()),
            swap_processor: SwapProcessor::new(db_manager.clone()),
            buffer_processor: BufferProcessor::new(db_manager.clone()),
            position_processor: PositionProcessor::new(db_manager.clone()),
            floor_processor: FloorProcessor::new(db_manager.clone()),
        }
    }

    /// Process an account update by routing to the appropriate processor
    pub async fn process_account_update(
        &self,
        pubkey: &Pubkey,
        data: &[u8],
        slot: u64,
    ) -> Result<()> {
        let block_info = BlockInfo {
            slot,
            timestamp: chrono::Utc::now().timestamp(),
            block_height: None,
        };

        // Determine account type and route to appropriate processor
        if self.is_market_account(pubkey) {
            self.market_processor.process_account_update(*pubkey, data, block_info).await?;
        } else if self.is_buffer_account(pubkey) {
            self.buffer_processor.process_account_update(*pubkey, data, block_info).await?;
        } else if self.is_position_account(pubkey) {
            self.position_processor.process_account_update(*pubkey, data, block_info).await?;
        } else if self.is_floor_account(pubkey) {
            self.floor_processor.process_account_update(*pubkey, data, block_info).await?;
        } else {
            tracing::debug!("Unknown account type for {}", pubkey);
        }
        
        Ok(())
    }

    // Helper methods to determine account types
    fn is_market_account(&self, _pubkey: &Pubkey) -> bool {
        // TODO: Implement proper account type detection
        // For now, assume all accounts could be markets
        true
    }

    fn is_buffer_account(&self, _pubkey: &Pubkey) -> bool {
        // TODO: Implement proper account type detection
        false
    }

    fn is_position_account(&self, _pubkey: &Pubkey) -> bool {
        // TODO: Implement proper account type detection
        false
    }

    fn is_floor_account(&self, _pubkey: &Pubkey) -> bool {
        // TODO: Implement proper account type detection
        false
    }
    
    /// Process market account update
    pub async fn process_market_update(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()> {
        self.market_processor.process_account_update(pubkey, data, block_info).await
    }
    
    /// Process buffer account update
    pub async fn process_buffer_update(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()> {
        self.buffer_processor.process_account_update(pubkey, data, block_info).await
    }
    
    /// Process position account update
    pub async fn process_position_update(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()> {
        self.position_processor.process_account_update(pubkey, data, block_info).await
    }
    
    /// Process oracle account update
    pub async fn process_oracle_update(
        &self,
        pubkey: Pubkey,
        _data: &[u8],
        _block_info: BlockInfo,
    ) -> Result<()> {
        // Oracle processing would go here
        // For now, just log it
        tracing::debug!("Oracle update for {}", pubkey);
        Ok(())
    }
    
    /// Process protocol config update
    pub async fn process_protocol_config_update(
        &self,
        pubkey: Pubkey,
        _data: &[u8],
        _block_info: BlockInfo,
    ) -> Result<()> {
        // Protocol config processing would go here
        tracing::debug!("Protocol config update for {}", pubkey);
        Ok(())
    }
    
    /// Process safety controller update
    pub async fn process_safety_controller_update(
        &self,
        pubkey: Pubkey,
        _data: &[u8],
        _block_info: BlockInfo,
    ) -> Result<()> {
        // Safety controller processing would go here
        tracing::debug!("Safety controller update for {}", pubkey);
        Ok(())
    }
    
    /// Process transaction
    pub async fn process_transaction(
        &self,
        signature: String,
        transaction: &EncodedTransaction,
        block_info: BlockInfo,
    ) -> Result<()> {
        // Route to swap processor for now
        // In a full implementation, we'd parse the transaction to determine type
        self.swap_processor.process_transaction(signature, transaction, block_info).await
    }
    
    /// Process block
    pub async fn process_block(&self, block: &ConfirmedBlock) -> Result<()> {
        // Block-level processing (aggregations, etc.)
        tracing::debug!("Processing block: {}", block.parent_slot);
        Ok(())
    }
}
