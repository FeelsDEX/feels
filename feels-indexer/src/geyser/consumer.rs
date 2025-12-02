//! Geyser stream consumer for Feels Protocol
//! Manages the connection to Geyser streams and processes incoming updates

use anyhow::Result;
use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::{error, info, warn};

use super::{FeelsGeyserClient, should_use_real_client};
use crate::adapters::solana::geyser::SubscribeUpdate;
use super::stream_handler::StreamHandler;
use crate::config::GeyserConfig;
use crate::database::DatabaseManager;

/// Main consumer for Geyser streams
pub struct FeelsGeyserConsumer {
    client: FeelsGeyserClient,
    stream_handler: StreamHandler,
    program_id: Pubkey,
}

impl FeelsGeyserConsumer {
    /// Create a new Geyser consumer
    pub async fn new(
        config: GeyserConfig,
        program_id: Pubkey,
        db_manager: Arc<DatabaseManager>,
    ) -> Result<Self> {
        info!("Initializing Geyser consumer for program: {}", program_id);

        // Determine client mode
        let use_real = should_use_real_client(config.use_triton, &config.network);
        let token = if config.token.is_empty() { None } else { Some(config.token.as_str()) };

        // Connect to Geyser client
        let client = FeelsGeyserClient::connect(&config.endpoint, token, program_id, use_real).await?;

        // Create stream handler
        let stream_handler = StreamHandler::new(program_id, db_manager);

        Ok(Self {
            client,
            stream_handler,
            program_id,
        })
    }

    /// Start consuming the Geyser stream
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Geyser stream consumption for program: {}", self.program_id);

        // Subscribe to program account updates
        let mut stream = self.client.subscribe_to_program_accounts().await?;

        info!("Successfully subscribed to program accounts, processing updates...");

        // Process updates from the stream
        while let Some(update_result) = stream.next().await {
            match update_result {
                Ok(update) => {
                    if let Err(e) = self.handle_update(update).await {
                        error!("Failed to handle update: {}", e);
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    // TODO: Implement reconnection logic
                    break;
                }
            }
        }

        warn!("Geyser stream ended");
        Ok(())
    }

    /// Handle a single update from the stream
    async fn handle_update(&self, update: SubscribeUpdate) -> Result<()> {
        // Delegate to the stream handler for processing
        self.stream_handler.handle_update(update).await
    }
}