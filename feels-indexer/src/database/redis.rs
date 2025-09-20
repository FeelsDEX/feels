//! Redis cache manager

#![allow(dependency_on_unit_never_type_fallback)]

use super::DatabaseOperations;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct RedisManager {
    pub(crate) pool: Pool,
}

impl RedisManager {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        
        Ok(Self { pool })
    }

    /// Cache market price with TTL
    pub async fn cache_market_price(&self, market_id: Uuid, price: f64, ttl_secs: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:price", market_id);
        
        conn.set_ex(&key, price, ttl_secs).await?;
        Ok(())
    }

    /// Get cached market price
    pub async fn get_market_price(&self, market_id: Uuid) -> Result<Option<f64>> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:price", market_id);
        
        let price: Option<f64> = conn.get(&key).await?;
        Ok(price)
    }

    /// Cache user positions with TTL
    pub async fn cache_user_positions(&self, user: &str, positions: &[CachedPosition], ttl_secs: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("user:{}:positions", user);
        let serialized = serde_json::to_string(positions)?;
        
        conn.set_ex(&key, serialized, ttl_secs).await?;
        Ok(())
    }

    /// Get cached user positions
    pub async fn get_user_positions(&self, user: &str) -> Result<Option<Vec<CachedPosition>>> {
        let mut conn = self.pool.get().await?;
        let key = format!("user:{}:positions", user);
        
        let cached: Option<String> = conn.get(&key).await?;
        match cached {
            Some(data) => {
                let positions: Vec<CachedPosition> = serde_json::from_str(&data)?;
                Ok(Some(positions))
            }
            None => Ok(None),
        }
    }

    /// Cache market stats with TTL
    pub async fn cache_market_stats(&self, market_id: Uuid, stats: &MarketStats, ttl_secs: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:stats", market_id);
        let serialized = serde_json::to_string(stats)?;
        
        conn.set_ex(&key, serialized, ttl_secs).await?;
        Ok(())
    }

    /// Get cached market stats
    pub async fn get_market_stats(&self, market_id: Uuid) -> Result<Option<MarketStats>> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:stats", market_id);
        
        let cached: Option<String> = conn.get(&key).await?;
        match cached {
            Some(data) => {
                let stats: MarketStats = serde_json::from_str(&data)?;
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }

    /// Publish real-time price update
    pub async fn publish_price_update(&self, market_id: Uuid, price: f64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let channel = format!("price_updates:{}", market_id);
        let message = serde_json::json!({
            "market_id": market_id,
            "price": price,
            "timestamp": chrono::Utc::now()
        });
        
        conn.publish(&channel, message.to_string()).await?;
        Ok(())
    }

    /// Publish new swap event
    pub async fn publish_swap_event(&self, swap_event: &SwapEvent) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let channel = format!("swaps:{}", swap_event.market_id);
        let message = serde_json::to_string(swap_event)?;
        
        conn.publish(&channel, message).await?;
        Ok(())
    }

    /// Cache trending markets
    pub async fn cache_trending_markets(&self, markets: &[TrendingMarket], ttl_secs: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = "trending_markets";
        let serialized = serde_json::to_string(markets)?;
        
        conn.set_ex(key, serialized, ttl_secs).await?;
        Ok(())
    }

    /// Get trending markets
    pub async fn get_trending_markets(&self) -> Result<Option<Vec<TrendingMarket>>> {
        let mut conn = self.pool.get().await?;
        let key = "trending_markets";
        
        let cached: Option<String> = conn.get(key).await?;
        match cached {
            Some(data) => {
                let markets: Vec<TrendingMarket> = serde_json::from_str(&data)?;
                Ok(Some(markets))
            }
            None => Ok(None),
        }
    }

    /// Increment swap counter for analytics
    pub async fn increment_swap_counter(&self, market_id: Uuid) -> Result<i64> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:swap_count", market_id);
        
        let count: i64 = conn.incr(&key, 1).await?;
        conn.expire(&key, 86400).await?; // 24 hour TTL
        
        Ok(count)
    }

    /// Generic method to get JSON data from Redis
    pub async fn get_json<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut conn = self.pool.get().await?;
        let cached: Option<String> = conn.get(key).await?;
        
        match cached {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }
    
    /// Generic method to set JSON data in Redis with TTL
    pub async fn set_json<T>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()>
    where
        T: serde::Serialize,
    {
        let mut conn = self.pool.get().await?;
        let json = serde_json::to_string(value)?;
        conn.set_ex(key, json, ttl_secs).await?;
        Ok(())
    }
    
    /// Delete a key from Redis
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        conn.del(key).await?;
        Ok(())
    }
    
    /// Add to recent swaps list
    pub async fn add_recent_swap(&self, market_id: Uuid, swap_data: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:recent_swaps", market_id);
        
        // Add to list and keep only last 100
        conn.lpush(&key, swap_data).await?;
        conn.ltrim(&key, 0, 99).await?;
        conn.expire(&key, 3600).await?; // 1 hour TTL
        
        Ok(())
    }

    /// Get recent swaps
    pub async fn get_recent_swaps(&self, market_id: Uuid, limit: isize) -> Result<Vec<String>> {
        let mut conn = self.pool.get().await?;
        let key = format!("market:{}:recent_swaps", market_id);
        
        let swaps: Vec<String> = conn.lrange(&key, 0, limit - 1).await?;
        Ok(swaps)
    }

    /// Cache global stats
    pub async fn cache_global_stats(&self, stats: &GlobalStats, ttl_secs: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = "global_stats";
        let serialized = serde_json::to_string(stats)?;
        
        conn.set_ex(key, serialized, ttl_secs).await?;
        Ok(())
    }

    /// Get global stats
    pub async fn get_global_stats(&self) -> Result<Option<GlobalStats>> {
        let mut conn = self.pool.get().await?;
        let key = "global_stats";
        
        let cached: Option<String> = conn.get(key).await?;
        match cached {
            Some(data) => {
                let stats: GlobalStats = serde_json::from_str(&data)?;
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl DatabaseOperations for RedisManager {
    async fn health_check(&self) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosition {
    pub id: Uuid,
    pub market_id: Uuid,
    pub liquidity: String, // Store as string to avoid precision loss
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub tokens_owed_0: i64,
    pub tokens_owed_1: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStats {
    pub market_id: Uuid,
    pub price_24h_change: f64,
    pub volume_24h: String,
    pub tvl: String,
    pub fees_24h: String,
    pub apr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub market_id: Uuid,
    pub signature: String,
    pub trader: String,
    pub amount_in: i64,
    pub amount_out: i64,
    pub price: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingMarket {
    pub market_id: Uuid,
    pub token_0: String,
    pub token_1: String,
    pub volume_24h: String,
    pub price_change_24h: f64,
    pub rank: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_markets: i64,
    pub total_volume_24h: String,
    pub total_tvl: String,
    pub total_fees_24h: String,
    pub active_traders_24h: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
