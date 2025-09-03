use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use feels_types::{FeelsResult, FeelsProtocolError};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;

use feels_keeper::{Keeper, KeeperConfig};

#[derive(Parser, Debug)]
#[command(name = "feels-keeper")]
#[command(about = "Feels Protocol off-chain field computation service")]
struct Args {
    /// Path to keeper configuration file
    #[arg(short, long, default_value = "keeper.toml")]
    config: String,

    /// Keeper private key file path
    #[arg(short, long)]
    keypair: Option<String>,

    /// RPC URL for Solana cluster
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,

    /// Update interval in seconds
    #[arg(short, long, default_value = "30")]
    interval: u64,

    /// Dry run mode - compute but don't submit updates
    #[arg(long)]
    dry_run: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> FeelsResult<()> {
    let args = Args::parse();
    
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or(if args.verbose { "debug" } else { "info" })
    ).init();

    log::info!("Starting Feels Protocol Keeper");
    log::info!("RPC URL: {}", args.rpc_url);
    log::info!("Update interval: {}s", args.interval);
    
    if args.dry_run {
        log::warn!("Running in DRY RUN mode - no updates will be submitted");
    }

    // Load configuration
    let config = KeeperConfig::load(&args.config)?;
    
    log::info!("Loaded configuration for {} markets", config.markets.len());

    // Load keypair
    let keypair = if let Some(keypair_path) = args.keypair {
        Keypair::read_from_file(&keypair_path)
            .map_err(|e| FeelsProtocolError::generic(&format!("Failed to load keypair from {}: {}", keypair_path, e)))?
    } else {
        log::warn!("No keypair provided, using random keypair (dry run only)");
        Keypair::new()
    };

    log::info!("Keeper authority: {}", keypair.pubkey());

    // Create RPC client
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        args.rpc_url,
        CommitmentConfig::confirmed(),
    ));

    // Initialize keeper
    let mut keeper = Keeper::new(
        rpc_client,
        Arc::new(keypair),
        config,
        args.dry_run,
    )?;

    log::info!("Keeper initialized successfully");

    // Start main update loop
    let mut interval_timer = time::interval(Duration::from_secs(args.interval));
    let mut iteration = 0u64;

    loop {
        interval_timer.tick().await;
        iteration += 1;
        
        log::debug!("Starting keeper iteration {}", iteration);

        match keeper.update_all_markets().await {
            Ok(updates) => {
                if updates > 0 {
                    log::info!("Iteration {}: Updated {} markets", iteration, updates);
                } else {
                    log::debug!("Iteration {}: No markets needed updates", iteration);
                }
            }
            Err(e) => {
                log::error!("Error in keeper iteration {}: {}", iteration, e);
                // Continue running even if individual iterations fail
            }
        }

        // Basic health metrics every 100 iterations
        if iteration % 100 == 0 {
            log::info!("Keeper health check - iteration {}", iteration);
            if let Err(e) = keeper.health_check().await {
                log::warn!("Health check warning: {}", e);
            }
        }
    }
}