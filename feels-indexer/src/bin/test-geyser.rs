//! Simple binary to test the Geyser client functionality
//! This can use either mock (test data) or real (RPC polling) client based on configuration

use anyhow::Result;
use feels_indexer::geyser::{FeelsGeyserClient, should_use_real_client};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use futures::StreamExt;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,feels_indexer=debug,test_geyser=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Geyser client test");

    // Parse command line arguments for network selection
    let args: Vec<String> = std::env::args().collect();
    let network = if args.len() > 1 {
        args[1].clone()
    } else {
        "devnet".to_string() // Default to devnet
    };

    let config_file = match network.as_str() {
        "localnet" => "indexer-localnet.toml",
        "devnet" => "indexer-devnet.toml", 
        "mainnet" => "indexer-mainnet.toml",
        _ => {
            error!("Invalid network: {}. Use 'localnet', 'devnet', or 'mainnet'", network);
            return Err(anyhow::anyhow!("Invalid network"));
        }
    };

    info!("Using network: {} with config: {}", network, config_file);

    // Load configuration
    let mut config = feels_indexer::config::IndexerConfig::from_file(config_file)?;
    config.load_env_vars()?; // Load environment variables for endpoints/tokens

    let geyser_config = config.geyser;
    let program_id = Pubkey::from_str(&geyser_config.program_id)?;
    
    info!("Testing with program ID: {}", program_id);
    info!("Endpoint: {}", geyser_config.endpoint);
    info!("Network: {}", geyser_config.network);

    // Determine client mode
    let use_real = should_use_real_client(geyser_config.use_triton, &geyser_config.network);
    let token = if geyser_config.token.is_empty() { None } else { Some(geyser_config.token.as_str()) };

    if use_real {
        info!("🌐 REAL MODE: Connecting to {} network for actual blockchain data", network);
        warn!("This will poll real RPC endpoints for program account data");
    } else {
        info!("🧪 MOCK MODE: Generating test data for local development");
        warn!("This will generate synthetic data for testing purposes");
    }

    // Connect to client
    let mut client = FeelsGeyserClient::connect(&geyser_config.endpoint, token, program_id, use_real).await?;

    info!("✅ Connected successfully! Subscribing to program accounts...");
    let mut stream = client.subscribe_to_program_accounts().await?;

    info!("🔄 Listening for updates (press Ctrl+C to stop)...");
    let mut update_count = 0;
    
    while let Some(update_result) = stream.next().await {
        match update_result {
            Ok(update) => {
                update_count += 1;
                
                // Log different types of updates
                #[cfg(feature = "real-geyser")]
                {
                    use feels_indexer::adapters::solana::geyser::UpdateOneof;
                    if let Some(update_oneof) = &update.update_oneof {
                        match update_oneof {
                            UpdateOneof::Account(account_update) => {
                                if let Some(account_info) = &account_update.account {
                                    let pubkey_bytes = &account_info.pubkey;
                                    let pubkey = if pubkey_bytes.len() == 32 {
                                        let mut array = [0u8; 32];
                                        array.copy_from_slice(pubkey_bytes);
                                        Pubkey::from(array)
                                    } else {
                                        Pubkey::default()
                                    };
                                    
                                    info!("📊 ACCOUNT UPDATE #{}: {} (lamports: {}, data: {} bytes)", 
                                          update_count, pubkey, account_info.lamports, account_info.data.len());
                                }
                            }
                            UpdateOneof::Slot(slot_update) => {
                                info!("🎰 SLOT UPDATE #{}: slot {} (parent: {:?})", 
                                      update_count, slot_update.slot, slot_update.parent);
                            }
                            _ => {
                                info!("📦 OTHER UPDATE #{}", update_count);
                            }
                        }
                    }
                }
                
                #[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
                {
                    info!("📦 UPDATE #{}: Received mock update", update_count);
                }
                
                // Show first 20 updates, then every 10th
                if update_count <= 20 || update_count % 10 == 0 {
                    info!("Total updates received: {}", update_count);
                }
            }
            Err(e) => {
                error!("❌ Stream error: {}", e);
                break;
            }
        }
    }

    info!("🏁 Geyser client test completed! Received {} total updates", update_count);
    Ok(())
}