//! Stream processor implementation for Geyser updates
//!
//! This module implements the core processing logic for different types
//! of Geyser updates using the Feels SDK for deserialization.

use crate::database::{DatabaseManager, Market, Position, Swap};
use crate::sdk_types::feels_sdk;
use crate::sdk_types::AccountType;
use anyhow::{anyhow, Result};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;
use chrono::Utc;

/// Processes raw Geyser account and transaction data
pub struct StreamProcessor {
    db_manager: Arc<DatabaseManager>,
    program_id: Pubkey,
}

impl StreamProcessor {
    pub fn new(db_manager: Arc<DatabaseManager>, program_id: Pubkey) -> Self {
        Self { db_manager, program_id }
    }

    /// Process a raw account update
    pub async fn process_account(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        // Skip if data is too small
        if data.len() < 8 {
            return Ok(());
        }

        // Determine account type using the discriminator
        match AccountType::from_discriminator(&data[..8]) {
            Some(AccountType::Market) => {
                self.process_market_account(pubkey, data, slot).await?;
            }
            Some(AccountType::Position) => {
                self.process_position_account(pubkey, data, slot).await?;
            }
            Some(AccountType::Buffer) => {
                self.process_buffer_account(pubkey, data, slot).await?;
            }
            Some(AccountType::ProtocolConfig) => {
                self.process_protocol_config(pubkey, data, slot).await?;
            }
            Some(AccountType::ProtocolToken) => {
                self.process_protocol_token(pubkey, data, slot).await?;
            }
            _ => {
                debug!("Unknown account type for {}", pubkey);
            }
        }

        Ok(())
    }

    /// Process a market account update
    async fn process_market_account(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        info!("Processing market account: {}", pubkey);
        
        // Decode market using SDK
        let market_data = feels_sdk::decode_market(data).map_err(|e: String| anyhow!(e))?;
        
        // Convert to database model
        let market = Market {
            id: Uuid::new_v4(),
            address: pubkey.to_string(),
            token_0: market_data.token_0.to_string(),
            token_1: market_data.token_1.to_string(),
            sqrt_price: market_data.sqrt_price.into(),
            liquidity: market_data.liquidity.into(),
            current_tick: market_data.current_tick,
            tick_spacing: market_data.tick_spacing as i16,
            fee_bps: market_data.base_fee_bps as i16,
            is_paused: market_data.is_paused,
            phase: if market_data.initial_liquidity_deployed { 
                "steady_state".to_string() 
            } else { 
                "discovery".to_string() 
            },
            global_lower_tick: market_data.global_lower_tick,
            global_upper_tick: market_data.global_upper_tick,
            fee_growth_global_0: market_data.fee_growth_global_0_x64.into(),
            fee_growth_global_1: market_data.fee_growth_global_1_x64.into(),
            total_volume_0: rust_decimal::Decimal::ZERO, // Will be calculated from swaps
            total_volume_1: rust_decimal::Decimal::ZERO,
            total_fees_0: rust_decimal::Decimal::ZERO,
            total_fees_1: rust_decimal::Decimal::ZERO,
            swap_count: 0,
            unique_traders: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_updated_slot: slot as i64,
        };

        // Store in PostgreSQL
        self.db_manager.postgres.upsert_market(&market).await?;
        
        // Cache in Redis for fast lookups
        self.db_manager.redis.cache_market(pubkey.to_string(), &market).await?;

        // Store raw data in RocksDB
        self.db_manager.rocksdb.store_account(pubkey, data, slot).await?;

        Ok(())
    }

    /// Process a position account update
    async fn process_position_account(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        info!("Processing position account: {}", pubkey);
        
        // Decode position using SDK
        let position_data = feels_sdk::decode_position(data).map_err(|e: String| anyhow!(e))?;
        
        // Get market ID from cache or database
        let market_id = self.db_manager.redis
            .get_market_id(&position_data.market.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Market not found: {}", position_data.market))?;

        // Convert to database model
        let position = Position {
            id: Uuid::new_v4(),
            address: pubkey.to_string(),
            market_id,
            owner: position_data.owner.to_string(),
            liquidity: position_data.liquidity.into(),
            tick_lower: position_data.tick_lower,
            tick_upper: position_data.tick_upper,
            fee_growth_inside_0_last: position_data.fee_growth_inside_0_last_x64.into(),
            fee_growth_inside_1_last: position_data.fee_growth_inside_1_last_x64.into(),
            tokens_owed_0: position_data.tokens_owed_0 as i64,
            tokens_owed_1: position_data.tokens_owed_1 as i64,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_updated_slot: slot as i64,
        };

        // Store in PostgreSQL
        self.db_manager.postgres.upsert_position(&position).await?;

        // Store raw data in RocksDB
        self.db_manager.rocksdb.store_account(pubkey, data, slot).await?;

        Ok(())
    }

    /// Process a buffer account update
    async fn process_buffer_account(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        debug!("Processing buffer account: {}", pubkey);
        
        // Decode buffer using SDK
        let buffer_data = feels_sdk::decode_buffer(data).map_err(|e: String| anyhow!(e))?;
        
        // Store raw data in RocksDB
        self.db_manager.rocksdb.store_account(pubkey, data, slot).await?;
        
        // Cache buffer state in Redis
        self.db_manager.redis.cache_buffer_state(pubkey.to_string(), &buffer_data).await?;

        Ok(())
    }

    /// Process protocol config update
    async fn process_protocol_config(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        debug!("Processing protocol config: {}", pubkey);
        
        // Store raw data in RocksDB
        self.db_manager.rocksdb.store_account(pubkey, data, slot).await?;

        Ok(())
    }

    /// Process protocol token update
    async fn process_protocol_token(&self, pubkey: &Pubkey, data: &[u8], slot: u64) -> Result<()> {
        debug!("Processing protocol token: {}", pubkey);
        
        // Store raw data in RocksDB
        self.db_manager.rocksdb.store_account(pubkey, data, slot).await?;

        Ok(())
    }

    /// Process a transaction containing Feels instructions
    pub async fn process_transaction(
        &self, 
        signature: &str,
        transaction_data: &[u8],
        slot: u64,
        block_height: Option<u64>
    ) -> Result<()> {
        info!("Processing transaction: {}", signature);
        
        // Parse transaction using SDK
        match feels_sdk::parse_transaction(transaction_data) {
            Ok(parsed_tx) => {
                // Process each instruction
                for instruction in parsed_tx.instructions {
                    match instruction {
                        feels_sdk::Instruction::Swap(swap_data) => {
                            self.process_swap_instruction(signature, &swap_data, slot, block_height).await?;
                        }
                        feels_sdk::Instruction::OpenPosition(_pos_data) => {
                            debug!("Position opened in tx: {}", signature);
                        }
                        feels_sdk::Instruction::ClosePosition(_) => {
                            debug!("Position closed in tx: {}", signature);
                        }
                        _ => {
                            // Other instruction types
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse transaction {}: {}", signature, e);
            }
        }

        // Store raw transaction in RocksDB
        self.db_manager.rocksdb.store_transaction(signature, transaction_data, slot).await?;

        Ok(())
    }

    /// Process a swap instruction
    async fn process_swap_instruction(
        &self,
        signature: &str,
        swap_data: &feels_sdk::SwapData,
        slot: u64,
        block_height: Option<u64>
    ) -> Result<()> {
        // Get market ID
        let market_id = self.db_manager.redis
            .get_market_id(&swap_data.market.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Market not found for swap"))?;

        // Create swap record
        let swap = Swap {
            id: Uuid::new_v4(),
            signature: signature.to_string(),
            market_id,
            trader: swap_data.trader.to_string(),
            amount_in: swap_data.amount_in as i64,
            amount_out: swap_data.amount_out as i64,
            token_in: swap_data.token_in.to_string(),
            token_out: swap_data.token_out.to_string(),
            sqrt_price_before: swap_data.sqrt_price_before.into(),
            sqrt_price_after: swap_data.sqrt_price_after.into(),
            tick_before: swap_data.tick_before,
            tick_after: swap_data.tick_after,
            liquidity: swap_data.liquidity.into(),
            fee_amount: swap_data.fee_amount as i64,
            timestamp: Utc::now(),
            slot: slot as i64,
            block_height: block_height.map(|h| h as i64),
            price_impact_bps: Some(swap_data.price_impact_bps as i16),
            effective_price: Some(rust_decimal::Decimal::from_f64_retain(swap_data.effective_price).unwrap_or_default()),
        };

        // Store in PostgreSQL
        self.db_manager.postgres.insert_swap(&swap).await?;

        // Update market statistics
        self.update_market_stats(&swap_data.market.to_string(), &swap).await?;

        Ok(())
    }

    /// Update market statistics after a swap
    async fn update_market_stats(&self, market_address: &str, swap: &Swap) -> Result<()> {
        // Update volume and fee counters in Redis
        self.db_manager.redis.increment_market_volume(
            market_address,
            &swap.token_in,
            swap.amount_in.unsigned_abs(),
        ).await?;

        self.db_manager.redis.increment_market_fees(
            market_address,
            &swap.token_out,
            swap.fee_amount.unsigned_abs(),
        ).await?;

        Ok(())
    }
}