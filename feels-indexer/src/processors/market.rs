//! Market account processor

use super::AccountProcessor;
use crate::database::DatabaseManager;
use crate::models::{BlockInfo};
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{debug, error};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Processor for market account updates
pub struct MarketProcessor {
    db_manager: Arc<DatabaseManager>,
}

impl MarketProcessor {
    /// Create a new market processor
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }
    
    /// Parse market account data
    fn parse_market_data(&self, data: &[u8]) -> Result<MarketData> {
        // This would use the actual Feels SDK to deserialize market data
        // For now, we'll create a placeholder implementation
        
        if data.len() < 8 {
            return Err(anyhow::anyhow!("Market data too short"));
        }
        
        // Skip discriminator (first 8 bytes)
        let _data = &data[8..];
        
        // This is a simplified parser - in reality we'd use anchor deserialization
        // or the Feels SDK market parsing
        Ok(MarketData {
            token_0: Pubkey::default(), // Would parse from data
            token_1: Pubkey::default(), // Would parse from data
            sqrt_price: 0,              // Would parse from data
            liquidity: 0,               // Would parse from data
            current_tick: 0,            // Would parse from data
            tick_spacing: 0,            // Would parse from data
            fee_bps: 0,                 // Would parse from data
            is_paused: false,           // Would parse from data
        })
    }
}

#[async_trait::async_trait]
impl AccountProcessor for MarketProcessor {
    async fn process_account_update(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        block_info: BlockInfo,
    ) -> Result<()> {
        debug!("Processing market update for {}", pubkey);
        
        // Parse market data
        let market_data = match self.parse_market_data(data) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to parse market data for {}: {}", pubkey, e);
                return Ok(()); // Continue processing other updates
            }
        };
        
        // Check if this market already exists
        let existing_market = self.db_manager.postgres
            .get_market(&pubkey.to_string())
            .await?;
        
        // Create market structure for database
        let market = crate::database::Market {
            id: existing_market.as_ref().map(|m| m.id).unwrap_or(uuid::Uuid::new_v4()),
            address: pubkey.to_string(),
            token_0: market_data.token_0.to_string(),
            token_1: market_data.token_1.to_string(),
            sqrt_price: Decimal::from_u128(market_data.sqrt_price).unwrap_or_default(),
            liquidity: Decimal::from_u128(market_data.liquidity).unwrap_or_default(),
            current_tick: market_data.current_tick,
            tick_spacing: market_data.tick_spacing as i16,
            fee_bps: market_data.fee_bps as i16,
            is_paused: market_data.is_paused,
            phase: "PriceDiscovery".to_string(), // Would determine from data
            global_lower_tick: -100_800,      // Would parse from data
            global_upper_tick: 100_800,       // Would parse from data
            fee_growth_global_0: Decimal::from(0),
            fee_growth_global_1: Decimal::from(0),
            total_volume_0: existing_market.as_ref().map(|m| m.total_volume_0).unwrap_or_default(),
            total_volume_1: existing_market.as_ref().map(|m| m.total_volume_1).unwrap_or_default(),
            total_fees_0: existing_market.as_ref().map(|m| m.total_fees_0).unwrap_or_default(),
            total_fees_1: existing_market.as_ref().map(|m| m.total_fees_1).unwrap_or_default(),
            swap_count: existing_market.as_ref().map(|m| m.swap_count).unwrap_or(0),
            unique_traders: existing_market.as_ref().map(|m| m.unique_traders).unwrap_or(0),
            created_at: existing_market.as_ref().map(|m| m.created_at).unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::Utc::now(),
            last_updated_slot: block_info.slot as i64,
        };
        
        // Store in PostgreSQL
        self.db_manager.postgres
            .upsert_market(&market)
            .await?;
        
        // Also store in RocksDB for fast access
        self.db_manager.rocksdb
            .put_market(&pubkey.to_string(), &crate::models::IndexedMarket {
                address: pubkey,
                token_0: market_data.token_0,
                token_1: market_data.token_1,
                sqrt_price: market_data.sqrt_price,
                liquidity: market_data.liquidity,
                current_tick: market_data.current_tick,
                tick_spacing: market_data.tick_spacing,
                fee_bps: market_data.fee_bps,
                is_paused: market_data.is_paused,
                phase: crate::models::PoolPhase::PriceDiscovery,
                global_lower_tick: -100_800,
                global_upper_tick: 100_800,
                fee_growth_global_0: 0,
                fee_growth_global_1: 0,
                last_updated: block_info.clone(),
                total_volume_0: 0,
                total_volume_1: 0,
                total_fees_0: 0,
                total_fees_1: 0,
                swap_count: 0,
                unique_traders: 0,
            })?;
        
        debug!("Successfully processed market update for {}", pubkey);
        Ok(())
    }
}

/// Parsed market data structure
#[derive(Debug)]
struct MarketData {
    token_0: Pubkey,
    token_1: Pubkey,
    sqrt_price: u128,
    liquidity: u128,
    current_tick: i32,
    tick_spacing: u16,
    fee_bps: u16,
    is_paused: bool,
}