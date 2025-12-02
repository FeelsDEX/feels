//! Processor registry for routing account updates

use crate::core::{IndexerResult, ProcessContext, StoragePort};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::warn;

use super::{MarketAccountProcessor, PositionAccountProcessor};

/// Discriminator values for Feels accounts (first 8 bytes)
pub struct Discriminators;

impl Discriminators {
    pub const MARKET: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];
    pub const POSITION: [u8; 8] = [170, 188, 143, 228, 122, 64, 247, 208];
    pub const BUFFER: [u8; 8] = [128, 48, 191, 243, 107, 128, 246, 60];
    pub const FLOOR: [u8; 8] = [213, 53, 89, 64, 185, 124, 122, 168];
}

/// Registry for all account processors
pub struct ProcessorRegistry<S: StoragePort> {
    market_processor: MarketAccountProcessor<S>,
    position_processor: PositionAccountProcessor<S>,
}

impl<S: StoragePort + 'static> ProcessorRegistry<S> {
    /// Create a new processor registry
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            market_processor: MarketAccountProcessor::new(storage.clone()),
            position_processor: PositionAccountProcessor::new(storage.clone()),
        }
    }
    
    /// Route an account update to the appropriate processor
    pub async fn process_account(
        &self,
        pubkey: Pubkey,
        data: &[u8],
        context: ProcessContext,
    ) -> IndexerResult<()> {
        // Check discriminator (first 8 bytes)
        if data.len() < 8 {
            warn!("Account data too short for {}", pubkey);
            return Ok(());
        }
        
        let discriminator: [u8; 8] = data[0..8].try_into().unwrap();
        
        match discriminator {
            Discriminators::MARKET => {
                self.market_processor.process(pubkey, data, context).await?;
            }
            Discriminators::POSITION => {
                self.position_processor.process(pubkey, data, context).await?;
            }
            Discriminators::BUFFER => {
                // TODO: Implement buffer processor
                warn!("Buffer processor not yet implemented for {}", pubkey);
            }
            Discriminators::FLOOR => {
                // TODO: Implement floor processor
                warn!("Floor processor not yet implemented for {}", pubkey);
            }
            _ => {
                warn!("Unknown account discriminator for {}: {:?}", pubkey, &discriminator);
            }
        }
        
        Ok(())
    }
}

