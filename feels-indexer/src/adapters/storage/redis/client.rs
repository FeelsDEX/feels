//! Redis client implementation

use crate::core::{CachePort, IndexerResult, IndexerError, StorageError};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error};

/// Redis cache client
pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    /// Connect to Redis
    pub async fn connect(url: &str) -> IndexerResult<Self> {
        let client = Client::open(url)
            .map_err(|e| IndexerError::Storage(StorageError::Cache(e.to_string())))?;
        
        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| IndexerError::Storage(StorageError::Cache(e.to_string())))?;
        
        debug!("Redis client connected successfully");
        
        Ok(Self { manager })
    }
}

#[async_trait]
impl CachePort for RedisClient {
    async fn get<T>(&self, key: &str) -> IndexerResult<Option<T>>
    where
        T: DeserializeOwned + Send,
    {
        let mut conn = self.manager.clone();
        
        let data: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| IndexerError::Storage(StorageError::Cache(e.to_string())))?;
        
        match data {
            Some(json) => {
                let value = serde_json::from_str(&json)
                    .map_err(|e| IndexerError::Deserialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    
    async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> IndexerResult<()>
    where
        T: Serialize + Send + Sync,
    {
        let mut conn = self.manager.clone();
        
        let json = serde_json::to_string(value)
            .map_err(|e| IndexerError::Serialization(e.to_string()))?;
        
        let ttl_secs = ttl.as_secs();
        
        let _: () = conn.set_ex(key, json, ttl_secs)
            .await
            .map_err(|e| IndexerError::Storage(StorageError::Cache(e.to_string())))?;
        
        debug!("Cached key: {} with TTL: {}s", key, ttl_secs);
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> IndexerResult<()> {
        let mut conn = self.manager.clone();
        
        let _: () = conn.del(key)
            .await
            .map_err(|e| IndexerError::Storage(StorageError::Cache(e.to_string())))?;
        
        debug!("Deleted cache key: {}", key);
        
        Ok(())
    }
    
    async fn health_check(&self) -> IndexerResult<()> {
        let mut conn = self.manager.clone();
        
        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Redis health check failed: {}", e);
                IndexerError::Storage(StorageError::Cache(format!("Health check failed: {}", e)))
            })?;
        
        if pong == "PONG" {
            Ok(())
        } else {
            Err(IndexerError::Storage(StorageError::Cache(
                "Unexpected PING response".to_string(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires running Redis
    async fn test_redis_roundtrip() {
        let client = RedisClient::connect("redis://localhost:6379").await.unwrap();
        
        let test_data = vec![1, 2, 3, 4, 5];
        client.set("test_key", &test_data, Duration::from_secs(60)).await.unwrap();
        
        let retrieved: Option<Vec<i32>> = client.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data));
        
        client.delete("test_key").await.unwrap();
        let after_delete: Option<Vec<i32>> = client.get("test_key").await.unwrap();
        assert_eq!(after_delete, None);
    }
}

