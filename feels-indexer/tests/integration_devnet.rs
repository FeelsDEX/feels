//! Integration test for Feels indexer with local devnet
//! 
//! This test spins up a local Solana validator with Geyser plugin,
//! deploys the Feels program, and verifies the indexer correctly
//! processes state changes.

use anyhow::Result;
use feels_indexer::config::IndexerConfig;
use feels_indexer::database::DatabaseManager;
use feels_indexer::geyser::FeelsGeyserConsumer;
use solana_sdk::pubkey::Pubkey;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{info, warn};

struct TestEnvironment {
    validator_process: Option<Child>,
    indexer_config: IndexerConfig,
    program_id: Pubkey,
}

impl TestEnvironment {
    async fn setup() -> Result<Self> {
        // Initialize logging
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info,feels_indexer=debug")
            .try_init();

        info!("Setting up test environment...");

        // Start Geyser-enabled devnet
        info!("Starting Geyser-enabled devnet...");
        let validator_process = Command::new("geyser-devnet")
            .spawn()?;

        // Wait for validator to be ready
        info!("Waiting for validator to start...");
        sleep(Duration::from_secs(10)).await;
        
        // Wait for Geyser gRPC to be ready
        info!("Waiting for Geyser gRPC to be ready on port 10000...");
        let mut geyser_ready = false;
        for _ in 0..30 {
            if tokio::net::TcpStream::connect("127.0.0.1:10000").await.is_ok() {
                geyser_ready = true;
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
        
        if !geyser_ready {
            return Err(anyhow::anyhow!("Geyser gRPC failed to start on port 10000"));
        }
        
        info!("Geyser gRPC is ready!");

        // Configure indexer for local devnet
        let mut config = IndexerConfig::default();
        config.geyser.endpoint = "http://localhost:10000".to_string(); // Geyser gRPC endpoint
        config.geyser.program_id = "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N".to_string();
        config.database.postgres_url = "postgresql://localhost/feels_indexer_test".to_string();
        config.storage.rocksdb.path = "./test-data/rocksdb".into();
        config.storage.tantivy_path = "./test-data/tantivy".into();
        config.redis.url = "redis://localhost:6379/1".to_string(); // Use database 1 for tests

        let program_id: Pubkey = config.geyser.program_id.parse()?;

        Ok(Self {
            validator_process: Some(validator_process),
            indexer_config: config,
            program_id,
        })
    }

    async fn cleanup(&mut self) {
        info!("Cleaning up test environment...");
        
        if let Some(mut process) = self.validator_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        // Clean test data
        let _ = tokio::fs::remove_dir_all("./test-data").await;
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test integration_devnet -- --ignored
async fn test_indexer_with_devnet() -> Result<()> {
    let mut env = TestEnvironment::setup().await?;
    
    // Ensure cleanup happens
    let result = run_indexer_test(&env).await;
    
    env.cleanup().await;
    result
}

async fn run_indexer_test(env: &TestEnvironment) -> Result<()> {
    info!("Initializing database connections...");
    
    // Create database if not exists
    setup_test_database(&env.indexer_config.database.postgres_url).await?;
    
    // Initialize database manager
    let db_manager = Arc::new(
        DatabaseManager::new(
            &env.indexer_config.database.postgres_url,
            &env.indexer_config.redis.url,
            env.indexer_config.storage.rocksdb.clone(),
            &env.indexer_config.storage.tantivy_path,
        )
        .await?,
    );

    info!("Starting Geyser consumer...");
    
    // Create and start the Geyser consumer
    let mut consumer = FeelsGeyserConsumer::new(
        env.program_id,
        db_manager.clone(),
        &env.indexer_config.geyser,
    )
    .await?;

    // Start consuming in background
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = consumer.start().await {
            warn!("Consumer error: {}", e);
        }
    });

    // Wait a bit for initial sync
    sleep(Duration::from_secs(5)).await;

    // Deploy and interact with the program to generate events
    info!("Deploying program and generating test transactions...");
    
    // Deploy feels program
    deploy_program().await?;
    
    // Initialize a test market
    let market_address = create_test_market().await?;
    
    // Execute some swaps
    execute_test_swaps(&market_address).await?;

    // Wait for indexer to process
    sleep(Duration::from_secs(5)).await;

    // Verify data was indexed
    verify_indexed_data(&db_manager, &market_address).await?;

    // Shutdown consumer
    consumer_handle.abort();
    
    Ok(())
}

async fn setup_test_database(url: &str) -> Result<()> {
    // Extract database name from URL
    let db_name = url.split('/').last().unwrap_or("feels_indexer_test");
    let base_url = url.rsplit_once('/').map(|(base, _)| base).unwrap_or("postgresql://localhost");
    
    // Create test database
    let output = Command::new("psql")
        .args(&[base_url, "-c", &format!("CREATE DATABASE {} IF NOT EXISTS", db_name)])
        .output()?;
        
    if !output.status.success() {
        warn!("Failed to create test database: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Run migrations from parent directory where migrations folder is
    std::env::set_var("DATABASE_URL", url);
    let output = Command::new("sqlx")
        .args(&["migrate", "run"])
        .current_dir("..")  // Go to parent directory where migrations/ folder is
        .output()?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to run migrations: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn deploy_program() -> Result<()> {
    info!("Deploying Feels program...");
    
    let output = Command::new("anchor")
        .args(&["deploy", "--provider.cluster", "localnet"])
        .output()?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to deploy program: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

async fn create_test_market() -> Result<Pubkey> {
    info!("Creating test market...");
    
    // This would use the Feels SDK to create a market
    // For now, return a dummy address
    let market = Pubkey::new_unique();
    
    // TODO: Implement actual market creation using feels-sdk
    
    Ok(market)
}

async fn execute_test_swaps(market: &Pubkey) -> Result<()> {
    info!("Executing test swaps on market {}...", market);
    
    // TODO: Implement swap execution using feels-sdk
    
    Ok(())
}

async fn verify_indexed_data(db_manager: &Arc<DatabaseManager>, market: &Pubkey) -> Result<()> {
    info!("Verifying indexed data...");
    
    // Check if market was indexed
    let market_data = timeout(
        Duration::from_secs(30),
        wait_for_market(db_manager, market)
    ).await??;
    
    assert!(market_data.is_some(), "Market should be indexed");
    
    // Check for swaps
    // let swaps = db_manager.postgres
    //     .get_swaps_for_market(market.to_string(), 10, 0)
    //     .await?;
    
    // assert!(!swaps.is_empty(), "Should have indexed swaps");
    
    info!("Verification successful!");
    Ok(())
}

async fn wait_for_market(db_manager: &Arc<DatabaseManager>, market: &Pubkey) -> Result<Option<()>> {
    let market_str = market.to_string();
    
    for _ in 0..30 {
        // Try to get market from database
        match db_manager.rocksdb.get_market(&market_str)? {
            Some(_) => return Ok(Some(())),
            None => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_setup() {
        let config = IndexerConfig::default();
        assert_eq!(config.geyser.endpoint, "http://localhost:10000");
    }
}