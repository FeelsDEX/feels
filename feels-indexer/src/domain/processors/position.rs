//! Position account processor

use crate::core::{IndexerResult, ProcessContext, StoragePort};
use crate::domain::models::{IndexedPosition, PositionType};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{debug, error};

/// Processor for position account updates
pub struct PositionAccountProcessor<S: StoragePort> {
    storage: Arc<S>,
}

impl<S: StoragePort> PositionAccountProcessor<S> {
    /// Create a new position processor
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
    
    /// Process a position account update
    pub async fn process(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        context: ProcessContext,
    ) -> IndexerResult<IndexedPosition> {
        debug!("Processing position update for {}", pubkey);
        
        if data.len() < 8 {
            return Err(crate::core::IndexerError::Deserialization(
                "Position data too short".to_string()
            ));
        }
        
        // Skip discriminator
        let _data = &data[8..];
        
        // TODO: Implement actual deserialization
        // For now, create placeholder position
        let indexed_position = IndexedPosition {
            address: pubkey,
            market: Pubkey::default(),
            owner: Pubkey::default(),
            tick_lower: 0,
            tick_upper: 0,
            liquidity: 0,
            fee_growth_inside_0_last: 0,
            fee_growth_inside_1_last: 0,
            fees_owed_0: 0,
            fees_owed_1: 0,
            position_type: PositionType::UserLP,
            is_protocol_owned: false,
            total_fees_earned_0: 0,
            total_fees_earned_1: 0,
            impermanent_loss: 0.0,
            created_at: context.block_info.clone(),
            last_updated: context.block_info,
            is_closed: false,
        };
        
        // Store the position
        if let Err(e) = self.storage.store_position(&indexed_position).await {
            error!("Failed to store position {}: {}", pubkey, e);
        }
        
        Ok(indexed_position)
    }
}

