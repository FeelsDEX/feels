//! Unified Geyser client that switches between mock and real implementations

use anyhow::Result;
use futures::Stream;
use solana_sdk::pubkey::Pubkey;
use std::pin::Pin;
use tracing::info;

#[cfg(feature = "real-geyser")]
use super::client_real::RealGeyserClient;
#[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
use super::client_mock::MockGeyserClient;

// Re-export the appropriate types based on features
#[cfg(feature = "real-geyser")]
pub use super::client_real::{SubscribeUpdate, helpers};

#[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
pub use super::client_mock::{geyser_stub::SubscribeUpdate, helpers};

// Type alias for the stream
pub type GeyserStream = Pin<Box<dyn Stream<Item = Result<SubscribeUpdate, tonic::Status>> + Send>>;

/// Unified Geyser client that can switch between real and mock implementations
pub enum FeelsGeyserClient {
    #[cfg(feature = "real-geyser")]
    Real(RealGeyserClient),
    #[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
    Mock(MockGeyserClient),
}

impl FeelsGeyserClient {
    /// Connect to a Geyser endpoint
    pub async fn connect(
        endpoint: &str,
        token: Option<&str>,
        program_id: Pubkey,
        _use_real: bool,
    ) -> Result<Self> {
        #[cfg(all(not(feature = "real-geyser"), not(feature = "mock-geyser")))]
        {
            let _ = (endpoint, token, program_id, _use_real);
            return Err(anyhow::anyhow!("No Geyser client feature enabled"));
        }

        #[cfg(feature = "real-geyser")]
        {
            info!("Using real Geyser client");
            return Ok(FeelsGeyserClient::Real(
                RealGeyserClient::connect(endpoint, token, program_id).await?,
            ));
        }

        #[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
        {
            info!("Using mock Geyser client with test data generation");
            return Ok(FeelsGeyserClient::Mock(
                MockGeyserClient::connect(endpoint, program_id).await?,
            ));
        }
    }

    /// Subscribe to program account updates
    pub async fn subscribe_to_program_accounts(&mut self) -> Result<GeyserStream> {
        match self {
            #[cfg(feature = "real-geyser")]
            FeelsGeyserClient::Real(client) => {
                let stream = client.subscribe_to_program_accounts().await?;
                Ok(Box::pin(stream))
            }
            #[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
            FeelsGeyserClient::Mock(client) => {
                let stream = client.subscribe_to_program_accounts().await?;
                Ok(Box::pin(stream))
            }
        }
    }

    /// Subscribe to specific account updates
    pub async fn subscribe_to_specific_accounts(&mut self, accounts: Vec<Pubkey>) -> Result<GeyserStream> {
        match self {
            #[cfg(feature = "real-geyser")]
            FeelsGeyserClient::Real(client) => {
                let stream = client.subscribe_to_specific_accounts(accounts).await?;
                Ok(Box::pin(stream))
            }
            #[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
            FeelsGeyserClient::Mock(client) => {
                let stream = client.subscribe_to_specific_accounts(accounts).await?;
                Ok(Box::pin(stream))
            }
        }
    }
}

/// Determine whether to use real client based on configuration
pub fn should_use_real_client(use_triton: bool, network: &str) -> bool {
    // Use real client for devnet and mainnet, mock for localnet
    match network {
        "devnet" | "mainnet" => true,
        "localnet" => false,
        _ => use_triton, // Fall back to triton setting for unknown networks
    }
}
