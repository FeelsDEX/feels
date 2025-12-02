//! Service container for dependency injection

use crate::adapters::storage::StorageAdapter;
use crate::config::IndexerConfig;
use crate::core::{IndexerResult, StoragePort};
use crate::domain::processors::ProcessorRegistry;
use std::sync::Arc;
use tracing::info;

/// Service container managing all dependencies
pub struct ServiceContainer {
    /// Configuration
    pub config: IndexerConfig,
    
    /// Storage adapter (coordinating all storage backends)
    pub storage: Arc<StorageAdapter>,
    
    /// Processor registry
    pub processors: Arc<ProcessorRegistry<StorageAdapter>>,
}

impl ServiceContainer {
    /// Initialize the service container with all dependencies
    pub async fn new(config: IndexerConfig) -> IndexerResult<Self> {
        info!("Initializing service container");
        
        // Initialize storage adapter
        let storage = Arc::new(
            StorageAdapter::new(
                &config.database.postgres_url,
                &config.redis.url,
                config.storage.rocksdb.clone(),
                &config.search.index_path,
            )
            .await?,
        );
        
        info!("Storage backends initialized");
        
        // Initialize processor registry
        let processors = Arc::new(ProcessorRegistry::new(storage.clone()));
        
        info!("Processor registry initialized");
        
        Ok(Self {
            config,
            storage,
            processors,
        })
    }
    
    /// Perform health check on all services
    pub async fn health_check(&self) -> IndexerResult<()> {
        info!("Performing health check");
        
        let health = self.storage.health_check().await?;
        
        if !health.is_healthy() {
            return Err(crate::core::IndexerError::Configuration(
                format!("Health check failed: postgres={}, rocksdb={}, redis={}, tantivy={}",
                    health.postgres, health.rocksdb, health.redis, health.tantivy)
            ));
        }
        
        info!("All services healthy");
        Ok(())
    }
    
    /// Graceful shutdown
    pub async fn shutdown(&self) {
        info!("Shutting down service container");
        // Add cleanup logic here if needed
    }
}

