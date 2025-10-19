//! Geyser consumer implementation for Feels Protocol

use crate::config::GeyserConfig;
use crate::database::DatabaseManager;
use crate::processors::ProcessorRegistry;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::client::{FeelsGeyserClient}; //, geyser_stub::{SubscribeUpdate, UpdateOneof}, helpers};

/// Geyser consumer for Feels Protocol
pub struct FeelsGeyserConsumer {
    program_id: Pubkey,
    _db_manager: Arc<DatabaseManager>,
    config: GeyserConfig,
    _processor_registry: ProcessorRegistry,
}

impl FeelsGeyserConsumer {
    /// Create a new Geyser consumer
    pub async fn new(
        program_id: Pubkey,
        db_manager: Arc<DatabaseManager>,
        config: &GeyserConfig,
    ) -> Result<Self> {
        let processor_registry = ProcessorRegistry::new(db_manager.clone());
        
        Ok(Self {
            program_id,
            _db_manager: db_manager,
            config: config.clone(),
            _processor_registry: processor_registry,
        })
    }

    /// Start consuming the Geyser stream
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Feels Geyser consumer for program: {}", self.program_id);

        loop {
            match self.run_consumer().await {
                Ok(_) => {
                    warn!("Geyser stream ended unexpectedly, reconnecting...");
                }
                Err(e) => {
                    error!("Geyser consumer error: {}", e);
                    warn!("Retrying in 5 seconds...");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn run_consumer(&mut self) -> Result<()> {
        let mut client = FeelsGeyserClient::connect(&self.config.endpoint, self.program_id).await?;
        
        let _stream = client.subscribe_to_program_accounts().await?;
        
        info!("Connected to Geyser stream, processing updates...");
        
        // TODO: Re-enable when geyser client is fixed
        // while let Some(update_result) = stream.next().await {
        //     match update_result {
        //         Ok(update) => {
        //             if let Err(e) = self.handle_update(update).await {
        //                 error!("Error handling update: {}", e);
        //             }
        //         }
        //         Err(e) => {
        //             error!("Stream error: {}", e);
        //             return Err(e.into());
        //         }
        //     }
        // }
        warn!("Geyser streaming temporarily disabled due to tonic Body trait issue");
        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }

    /* TODO: Re-enable when tonic Body trait issue is fixed
    async fn handle_update(&self, update: SubscribeUpdate) -> Result<()> {
        match update.update_oneof {
            Some(update_oneof) => {
                match update_oneof {
                    UpdateOneof::Account(account_update) => {
                        self.handle_account_update(account_update).await?;
                    }
                    UpdateOneof::Transaction(transaction_update) => {
                        self.handle_transaction_update(transaction_update).await?;
                    }
                    UpdateOneof::Slot(slot_update) => {
                        self.handle_slot_update(slot_update).await?;
                    }
                }
            }
            None => {
                warn!("Received empty update");
            }
        }
        Ok(())
    }

    async fn handle_account_update(&self, update: super::client::geyser_stub::SubscribeUpdateAccount) -> Result<()> {
        if !helpers::is_feels_account_update(&update, &self.program_id) {
            return Ok(());
        }

        if let Some(pubkey) = helpers::extract_account_pubkey(&update) {
            debug!("Processing account update: {}", pubkey);
            
            if let Some(data) = helpers::extract_account_data(&update) {
                // Process the account update through our registry
                self.processor_registry.process_account_update(
                    &pubkey,
                    data,
                    update.slot,
                ).await?;
            }
        }

        Ok(())
    }

    async fn handle_transaction_update(&self, update: super::client::geyser_stub::SubscribeUpdateTransaction) -> Result<()> {
        if let Some(transaction_info) = &update.transaction {
            debug!("Processing transaction: {:?}", transaction_info.signature);
            
            // For now, just log transaction updates
            // TODO: Implement transaction processing
        }
        Ok(())
    }

    async fn handle_slot_update(&self, update: super::client::geyser_stub::SubscribeUpdateSlot) -> Result<()> {
        debug!("Slot update: {} (status: {:?})", update.slot, update.status);
        Ok(())
    }
    */

}