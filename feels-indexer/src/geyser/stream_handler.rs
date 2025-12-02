//! Stream handler for processing Geyser updates

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, info, warn, error};

use crate::database::DatabaseManager;
use crate::core::types::{BlockInfo, ProcessContext};
use std::sync::Arc;

use crate::adapters::solana::geyser::{SubscribeUpdate, UpdateOneof};

#[cfg(feature = "real-geyser")]
use crate::adapters::solana::geyser::helpers;

// Import the Feels program state types for deserialization
use feels;

/// Account discriminators for Feels Protocol accounts
mod discriminators {
    pub const MARKET: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];
    pub const POSITION: [u8; 8] = [170, 188, 143, 228, 122, 64, 247, 208];
    pub const BUFFER: [u8; 8] = [128, 48, 191, 243, 107, 128, 246, 60];
    pub const FLOOR: [u8; 8] = [213, 53, 89, 64, 185, 124, 122, 168];
    
    // Instruction discriminators (sighash of "global:instruction_name")
    // Note: These are examples - actual values depend on Anchor IDL
    pub const SWAP_IX: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];
}

/// Handles incoming Geyser stream messages and dispatches them to processors
pub struct StreamHandler {
    program_id: Pubkey,
    db_manager: Arc<DatabaseManager>,
    
    // Track metrics
    accounts_processed: std::sync::atomic::AtomicU64,
    slots_processed: std::sync::atomic::AtomicU64,
}

impl StreamHandler {
    pub fn new(
        program_id: Pubkey,
        db_manager: Arc<DatabaseManager>,
    ) -> Self {
        Self {
            program_id,
            db_manager,
            accounts_processed: std::sync::atomic::AtomicU64::new(0),
            slots_processed: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Process a Geyser update (account, slot, or transaction)
    pub async fn handle_update(&self, update: SubscribeUpdate) -> Result<()> {
        match update.update_oneof {
            Some(UpdateOneof::Account(account_update)) => {
                if let Some(account) = account_update.account {
                    let pubkey = if account.pubkey.len() == 32 {
                        let mut bytes = [0u8; 32];
                        bytes.copy_from_slice(&account.pubkey);
                        Pubkey::new_from_array(bytes)
                    } else {
                        return Err(anyhow::anyhow!("Invalid pubkey length"));
                    };
                    
                    let slot = account_update.slot;
                    let data = &account.data;
                    
                    // Process account based on discriminator
                    if let Err(e) = self.process_account(pubkey, data, slot).await {
                        error!("Failed to process account {}: {}", pubkey, e);
                    } else {
                        let count = self.accounts_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if count % 10 == 0 {
                            info!("✓ Processed {} accounts", count + 1);
                        }
                    }
                }
                Ok(())
            }
            Some(UpdateOneof::Slot(slot_update)) => {
                let count = self.slots_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 100 == 0 {
                    debug!(
                        "✓ Slot progress: slot={}, parent={:?} (processed {} slots)",
                        slot_update.slot,
                        slot_update.parent,
                        count + 1
                    );
                }
                Ok(())
            }
            Some(UpdateOneof::Transaction(tx_update)) => {
                if let Some(tx_info) = tx_update.transaction {
                    // Skip vote transactions
                    if tx_info.is_vote {
                        return Ok(());
                    }
                    
                    debug!(
                        "Transaction update: slot={}, signature_len={}",
                        tx_update.slot,
                        tx_info.signature.len()
                    );
                    
                    // Process transaction for swap instructions
                    // Note: Full implementation requires parsing transaction message
                    // For now, we log that we received a non-vote transaction
                    // TODO: Implement full transaction parsing when swap events are available
                }
                Ok(())
            }
            _ => {
                debug!("Received unhandled or empty update type");
                Ok(())
            }
        }
    }
    
    /// Process an account update based on its discriminator
    async fn process_account(&self, pubkey: Pubkey, data: &[u8], slot: u64) -> Result<()> {
        // Check minimum data length
        if data.len() < 8 {
            debug!("Account {} data too short ({} bytes), skipping", pubkey, data.len());
            return Ok(());
        }
        
        // Extract discriminator
        let discriminator: [u8; 8] = data[0..8].try_into()?;
        
        // Route to appropriate processor
        match discriminator {
            discriminators::MARKET => {
                info!("📊 Market account update: {} (slot: {}, {} bytes)", pubkey, slot, data.len());
                self.process_market_account(pubkey, data, slot).await?;
            }
            discriminators::POSITION => {
                info!("💼 Position account update: {} (slot: {}, {} bytes)", pubkey, slot, data.len());
                self.process_position_account(pubkey, data, slot).await?;
            }
            discriminators::BUFFER => {
                info!("💰 Buffer account update: {} (slot: {}, {} bytes)", pubkey, slot, data.len());
                // Buffer accounts don't need to be stored in the database for now
                debug!("Buffer account {} tracked at slot {}", pubkey, slot);
            }
            discriminators::FLOOR => {
                info!("📈 Floor account update: {} (slot: {}, {} bytes)", pubkey, slot, data.len());
                // Floor accounts are embedded in Market, not separate entities
                debug!("Floor account {} tracked at slot {}", pubkey, slot);
            }
            _ => {
                warn!(
                    "Unknown account discriminator for {}: [{:02x}, {:02x}, {:02x}, {:02x}, {:02x}, {:02x}, {:02x}, {:02x}]",
                    pubkey,
                    discriminator[0], discriminator[1], discriminator[2], discriminator[3],
                    discriminator[4], discriminator[5], discriminator[6], discriminator[7]
                );
            }
        }
        
        Ok(())
    }
    
    /// Process a Market account
    async fn process_market_account(&self, pubkey: Pubkey, data: &[u8], slot: u64) -> Result<()> {
        use crate::database::Market;
        use rust_decimal::Decimal;
        use uuid::Uuid;
        use anchor_lang::AnchorDeserialize;
        
        // Deserialize the Market account (skip 8-byte discriminator)
        let account_data = &data[8..];
        let market: feels::state::Market = AnchorDeserialize::try_from_slice(account_data)?;
        
        // Convert to database model
        let db_market = Market {
            id: Uuid::new_v4(),
            address: pubkey.to_string(),
            token_0: market.token_0.to_string(),
            token_1: market.token_1.to_string(),
            sqrt_price: Decimal::from(market.sqrt_price),
            liquidity: Decimal::from(market.liquidity),
            current_tick: market.current_tick,
            tick_spacing: market.tick_spacing as i16,
            fee_bps: market.policy.base_fee_bps as i16,
            is_paused: market.is_paused,
            phase: format!("{:?}", market.phase),
            global_lower_tick: market.global_lower_tick,
            global_upper_tick: market.global_upper_tick,
            fee_growth_global_0: Decimal::from(market.fee_growth_global_0),
            fee_growth_global_1: Decimal::from(market.fee_growth_global_1),
            total_volume_0: Decimal::ZERO, // Will be calculated from swaps
            total_volume_1: Decimal::ZERO,
            total_fees_0: Decimal::ZERO,
            total_fees_1: Decimal::ZERO,
            swap_count: 0,
            unique_traders: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_updated_slot: slot as i64,
        };
        
        // Store in database
        self.db_manager.postgres.insert_market(&db_market).await?;
        info!("✓ Stored market {} in database", pubkey);
        
        Ok(())
    }
    
    /// Process a Position account
    async fn process_position_account(&self, pubkey: Pubkey, data: &[u8], slot: u64) -> Result<()> {
        use crate::database::Position;
        use rust_decimal::Decimal;
        use uuid::Uuid;
        use anchor_lang::AnchorDeserialize;
        
        // Deserialize the Position account (skip 8-byte discriminator)
        let account_data = &data[8..];
        let position: feels::state::Position = AnchorDeserialize::try_from_slice(account_data)?;
        
        // Get market_id from database (we need to look up the market by address)
        let market = self.db_manager.postgres
            .get_market_by_address(&position.market.to_string())
            .await?;
        
        let market_id = match market {
            Some(m) => m.id,
            None => {
                warn!("Position {} references unknown market {}, skipping", pubkey, position.market);
                return Ok(());
            }
        };
        
        // Convert to database model
        let db_position = Position {
            id: Uuid::new_v4(),
            address: pubkey.to_string(),
            market_id,
            owner: position.owner.to_string(),
            liquidity: Decimal::from(position.liquidity),
            tick_lower: position.tick_lower,
            tick_upper: position.tick_upper,
            fee_growth_inside_0_last: Decimal::from(position.fee_growth_inside_0_last),
            fee_growth_inside_1_last: Decimal::from(position.fee_growth_inside_1_last),
            tokens_owed_0: position.tokens_owed_0 as i64,
            tokens_owed_1: position.tokens_owed_1 as i64,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_updated_slot: slot as i64,
        };
        
        // Store in database
        self.db_manager.postgres.insert_position(&db_position).await?;
        info!("✓ Stored position {} in database", pubkey);
        
        Ok(())
    }
    
    /// Get processing metrics
    pub fn get_metrics(&self) -> (u64, u64) {
        (
            self.accounts_processed.load(std::sync::atomic::Ordering::Relaxed),
            self.slots_processed.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}
