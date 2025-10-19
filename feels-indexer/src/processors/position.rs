//! Position account processor

use super::AccountProcessor;
use crate::database::DatabaseManager;
use crate::models::{BlockInfo};
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::debug;
use rust_decimal::Decimal;

/// Processor for position account updates
pub struct PositionProcessor {
    db_manager: Arc<DatabaseManager>,
}

impl PositionProcessor {
    /// Create a new position processor
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }
}

#[async_trait::async_trait]
impl AccountProcessor for PositionProcessor {
    async fn process_account_update(
        &self,
        pubkey: Pubkey,
        _data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()> {
        debug!("Processing position update for {}", pubkey);
        
        // Parse position data (placeholder implementation)
        // In reality, this would use the Feels SDK to deserialize the position data
        let market_address = "market_placeholder"; // Would parse from data
        let owner_address = "owner_placeholder";   // Would parse from data
        
        // Check if market exists
        let market = self.db_manager.postgres
            .get_market(market_address)
            .await?;
        
        if let Some(market) = market {
            let position = crate::database::Position {
                id: uuid::Uuid::new_v4(), // Would check if position already exists
                address: pubkey.to_string(),
                market_id: market.id,
                owner: owner_address.to_string(),
                liquidity: Decimal::from(0), // Would parse from data
                tick_lower: 0,               // Would parse from data
                tick_upper: 0,               // Would parse from data
                fee_growth_inside_0_last: Decimal::from(0), // Would parse from data
                fee_growth_inside_1_last: Decimal::from(0), // Would parse from data
                tokens_owed_0: 0,            // Would parse from data
                tokens_owed_1: 0,            // Would parse from data
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                last_updated_slot: block_info.slot as i64,
            };
            
            // Store in PostgreSQL
            self.db_manager.postgres
                .upsert_position(&position)
                .await?;
            
            // Also store in RocksDB for fast access
            self.db_manager.rocksdb
                .put_position(&pubkey.to_string(), &crate::models::IndexedPosition {
                    address: pubkey,
                    market: solana_sdk::pubkey::Pubkey::default(), // Would parse actual pubkey
                    owner: solana_sdk::pubkey::Pubkey::default(),  // Would parse actual pubkey
                    tick_lower: 0,
                    tick_upper: 0,
                    liquidity: 0,
                    fee_growth_inside_0_last: 0,
                    fee_growth_inside_1_last: 0,
                    fees_owed_0: 0,
                    fees_owed_1: 0,
                    position_type: crate::models::PositionType::UserLP,
                    is_protocol_owned: false,
                    total_fees_earned_0: 0,
                    total_fees_earned_1: 0,
                    impermanent_loss: 0.0,
                    created_at: block_info.clone(),
                    last_updated: block_info,
                    is_closed: false,
                })?;
        } else {
            debug!("Market {} not found for position {}, skipping", market_address, pubkey);
        }
        
        Ok(())
    }
}