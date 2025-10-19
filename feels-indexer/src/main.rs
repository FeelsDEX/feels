//! Feels Protocol Geyser Indexer
//!
//! Real-time indexer for the Feels Protocol that consumes Solana Geyser streams
//! and stores protocol state in RocksDB for efficient querying.

#![allow(dead_code)]

mod config;
mod database;
mod geyser;
mod models;
mod processors;
mod api;
mod repositories;
mod services;
mod sdk_types;

use anyhow::Result;
use clap::Parser;
use config::IndexerConfig;
use std::sync::Arc;
use std::str::FromStr;
use tokio::signal;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "feels-indexer")]
#[command(about = "Feels Protocol Geyser Indexer")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "indexer.toml")]
    config: String,

    /// Override log level
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
        warn!("Config file not found, using defaults: {}", cli.config);
        IndexerConfig::default()
    };

    // Override log level if provided
    if let Some(log_level) = cli.log_level {
        config.monitoring.log_level = log_level;
    }

    // Initialize logging
    init_logging(&config)?;

    info!("Starting Feels Protocol Indexer");
    info!("Program ID: {}", config.geyser.program_id);
    info!("Geyser endpoint: {}", config.geyser.endpoint);
    info!("RocksDB path: {:?}", config.storage.rocksdb.path);

    // Validate configuration and create directories
    config.validate()?;
    config.ensure_directories()?;
    let program_id = solana_sdk::pubkey::Pubkey::from_str(&config.geyser.program_id)?;
    info!("Configuration validated successfully");

    if cli.dry_run {
        info!("Dry run mode - configuration is valid, exiting");
        return Ok(());
    }

    // Initialize database manager
    info!("Initializing database connections...");
    let db_manager = Arc::new(database::DatabaseManager::new(
        &config.database.postgres_url,
        &config.redis.url,
        config.storage.rocksdb.clone(),
        &config.storage.tantivy_path,
    ).await?);
    info!("Database connections initialized successfully");

    // Initialize Geyser consumer
    info!("Initializing Geyser consumer...");
    let mut consumer = geyser::FeelsGeyserConsumer::new(
        program_id,
        db_manager.clone(),
        &config.geyser,
    ).await?;
    info!("Geyser consumer initialized successfully");

    // Start API server
    info!("Starting API server on {}", config.api.bind_address);
    let api_server = api::start_server(db_manager.clone(), &config.api).await?;

    // Start metrics server if enabled
    let _metrics_server = if config.monitoring.metrics_port > 0 {
        info!("Starting metrics server on port {}", config.monitoring.metrics_port);
        Some(api::start_metrics_server(config.monitoring.metrics_port).await?)
    } else {
        None
    };

    // Start the consumer
    info!("Starting Geyser stream consumption...");
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = consumer.start().await {
            error!("Geyser consumer error: {}", e);
        }
    });

    // Wait for shutdown signal
    info!("Indexer started successfully. Press Ctrl+C to shutdown.");
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
        _ = api_server => {
            info!("API server finished");
        }
    }

    info!("Shutting down Feels Protocol Indexer");
    Ok(())
}

fn init_logging(config: &IndexerConfig) -> Result<()> {
    let log_level = config.monitoring.log_level.parse()
        .unwrap_or(tracing::Level::INFO);

    if config.monitoring.structured_logging {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
                        format!("feels_indexer={},yellowstone_grpc_client=info", log_level).into()
                    })
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
                        format!("feels_indexer={},yellowstone_grpc_client=info", log_level).into()
                    })
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    Ok(())
}
