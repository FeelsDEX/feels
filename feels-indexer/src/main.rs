//! Feels Protocol Geyser Indexer
//!
//! Real-time indexer for the Feels Protocol that consumes Solana Geyser streams
//! and stores protocol state in PostgreSQL, Redis, RocksDB, and Tantivy.

use anyhow::Result;
use clap::Parser;
use feels_indexer::{
    api,
    config::IndexerConfig,
    database::DatabaseManager,
    geyser::{FeelsGeyserConsumer, should_use_real_client},
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "feels-indexer")]
#[command(about = "Feels Protocol Geyser Indexer", version, author)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "indexer-config.toml")]
    config: String,

    /// Override log level (trace, debug, info, warn, error)
    #[arg(long)]
    log_level: Option<String>,

    /// Dry run mode (validate config and exit)
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let mut config = if std::path::Path::new(&cli.config).exists() {
        IndexerConfig::from_file(&cli.config)?
    } else {
        warn!("Config file '{}' not found, using defaults", cli.config);
        IndexerConfig::default()
    };

    // Override log level if provided
    if let Some(log_level) = cli.log_level {
        config.monitoring.log_level = log_level;
    }

    // Initialize logging
    init_logging(&config)?;

    // Display startup banner
    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║         Feels Protocol Geyser Indexer v{}          ║", env!("CARGO_PKG_VERSION"));
    info!("╚══════════════════════════════════════════════════════════╝");
    
    // Parse and validate program ID
    let program_id = Pubkey::from_str(&config.geyser.program_id)?;
    
    // Display configuration
    info!("Configuration:");
    info!("  Program ID: {}", program_id);
    info!("  Geyser endpoint: {}", config.geyser.endpoint);
    info!("  Network: {}", config.geyser.network);
    info!("  Use Triton: {}", config.geyser.use_triton);
    info!("  RocksDB path: {:?}", config.storage.rocksdb.path);
    info!("  PostgreSQL: {}", mask_url(&config.database.postgres_url));
    info!("  Redis: {}", mask_url(&config.redis.url));
    
    // Show which client mode will be used
    let use_real = should_use_real_client(config.geyser.use_triton, &config.geyser.network);
    info!("  Geyser client: {}", if use_real { "Real (Yellowstone gRPC)" } else { "Mock (Testing)" });

    // Validate configuration and create directories
    config.validate()?;
    config.ensure_directories()?;
    info!("✓ Configuration validated successfully");

    if cli.dry_run {
        info!("Dry run mode - configuration is valid, exiting");
        return Ok(());
    }

    // Initialize database manager
    info!("Initializing storage layers...");
    let db_manager = Arc::new(DatabaseManager::new(
        &config.database.postgres_url,
        &config.redis.url,
        config.storage.rocksdb.clone(),
        &config.storage.tantivy_path,
    ).await?);
    info!("✓ Storage layers initialized");

    // Initialize Geyser consumer
    info!("Initializing Geyser consumer...");
    let mut consumer = FeelsGeyserConsumer::new(
        config.geyser.clone(),
        program_id,
        db_manager.clone(),
    ).await?;
    info!("✓ Geyser consumer initialized");

    // Start API server
    info!("Starting API server...");
    let api_handle = api::start_server(db_manager.clone(), &config.api).await?;
    info!("✓ API server started on {}", config.api.bind_address);

    // Start metrics server
    info!("Starting metrics server...");
    let metrics_handle = api::start_metrics_server(config.monitoring.metrics_port).await?;
    info!("✓ Metrics server started on port {}", config.monitoring.metrics_port);

    // Start the consumer
    info!("Starting Geyser stream consumption...");
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = consumer.start().await {
            error!("Geyser consumer error: {}", e);
        }
    });

    // Wait for shutdown signal
    info!("✓ Indexer started successfully");
    info!("Press Ctrl+C to shutdown");
    
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        result = consumer_handle => {
            match result {
                Ok(_) => info!("Consumer finished"),
                Err(e) => error!("Consumer task error: {}", e),
            }
        }
        result = api_handle => {
            match result {
                Ok(_) => info!("API server finished"),
                Err(e) => error!("API server task error: {}", e),
            }
        }
        result = metrics_handle => {
            match result {
                Ok(_) => info!("Metrics server finished"),
                Err(e) => error!("Metrics server task error: {}", e),
            }
        }
    }

    info!("Shutting down Feels Protocol Indexer");
    Ok(())
}

/// Initialize tracing subscriber with configurable log levels
fn init_logging(config: &IndexerConfig) -> Result<()> {
    let log_level = config.monitoring.log_level.parse()
        .unwrap_or(tracing::Level::INFO);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            format!(
                "feels_indexer={},yellowstone_grpc_client=info,solana_sdk=warn",
                log_level
            ).into()
        });

    if config.monitoring.structured_logging {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }

    Ok(())
}

/// Mask sensitive parts of URLs (passwords, tokens)
fn mask_url(url: &str) -> String {
    // Simple masking: if there's a password in the URL, mask it
    // Format: scheme://user:password@host
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(scheme_end) = url.find("://") {
                if colon_pos > scheme_end {
                    // There's a password to mask
                    return format!("{}:***{}", &url[..colon_pos], &url[at_pos..]);
                }
            }
        }
    }
    // No password found, return as-is
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_url() {
        assert_eq!(
            mask_url("postgresql://user:pass@localhost/db"),
            "postgresql://user:***@localhost/db"
        );
        assert_eq!(
            mask_url("redis://localhost:6379"),
            "redis://localhost:6379"
        );
    }
}
