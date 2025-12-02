//! Redis operations for caching and real-time data

#![allow(dependency_on_unit_never_type_fallback)]

use super::redis::RedisManager;
use super::{Market};
use anyhow::Result;
use redis::AsyncCommands;
use serde_json;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

/// Simplified buffer data (pending full deserialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferData {
    pub market: Pubkey,
    pub tau_spot: u64,
    pub tau_time: u64,
    pub tau_leverage: u64,
}

impl RedisManager {
    /// Cache a market
    pub async fn cache_market(&self, address: String, market: &Market) -> Result<()> {
        let mut conn = self.pool.get().await?;
        
        // Cache market data with TTL of 5 minutes
        let key = format!("market:{}", address);
        let value = serde_json::to_string(market)?;
        conn.set_ex(&key, value, 300).await?;
        
        // Cache market ID mapping
        let id_key = format!("market_id:{}", address);
        conn.set_ex(&id_key, market.id.to_string(), 300).await?;
        
        Ok(())
    }

    /// Get market ID from address
    pub async fn get_market_id(&self, address: &str) -> Result<Option<Uuid>> {
        let mut conn = self.pool.get().await?;
        let key = format!("market_id:{}", address);
        
        let id_str: Option<String> = conn.get(&key).await?;
        match id_str {
            Some(id) => Ok(Some(Uuid::parse_str(&id)?)),
            None => Ok(None),
        }
    }

    /// Cache buffer state
    pub async fn cache_buffer_state(&self, address: String, buffer_data: &BufferData) -> Result<()> {
        let mut conn = self.pool.get().await?;
        
        let key = format!("buffer:{}", address);
        let value = serde_json::to_string(buffer_data)?;
        conn.set_ex(&key, value, 60).await?; // 1 minute TTL
        
        Ok(())
    }

    /// Increment market volume
    pub async fn increment_market_volume(&self, market_address: &str, token: &str, amount: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        
        let key = format!("market:{}:volume:{}", market_address, token);
        conn.incr(&key, amount).await?;
        
        // Set expiry to end of day
        conn.expire(&key, 86400).await?;
        
        Ok(())
    }

    /// Increment market fees
    pub async fn increment_market_fees(&self, market_address: &str, token: &str, fee: u64) -> Result<()> {
        let mut conn = self.pool.get().await?;
        
        let key = format!("market:{}:fees:{}", market_address, token);
        conn.incr(&key, fee).await?;
        
        // Set expiry to end of day
        conn.expire(&key, 86400).await?;
        
        Ok(())
    }

    /// Get real-time market stats
    pub async fn get_market_stats_by_address(&self, market_address: &str) -> Result<MarketStats> {
        let mut conn = self.pool.get().await?;
        
        let volume_0_key = format!("market:{}:volume:token0", market_address);
        let volume_1_key = format!("market:{}:volume:token1", market_address);
        let fees_0_key = format!("market:{}:fees:token0", market_address);
        let fees_1_key = format!("market:{}:fees:token1", market_address);
        
        let volume_0: u64 = conn.get(&volume_0_key).await.unwrap_or(0);
        let volume_1: u64 = conn.get(&volume_1_key).await.unwrap_or(0);
        let fees_0: u64 = conn.get(&fees_0_key).await.unwrap_or(0);
        let fees_1: u64 = conn.get(&fees_1_key).await.unwrap_or(0);
        
        Ok(MarketStats {
            volume_token_0: volume_0,
            volume_token_1: volume_1,
            fees_token_0: fees_0,
            fees_token_1: fees_1,
        })
    }

    /// Publish market update event
    pub async fn publish_market_update(&self, market_address: &str, update_type: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        
        let channel = format!("market:{}:updates", market_address);
        let message = serde_json::json!({
            "type": update_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        conn.publish(&channel, message.to_string()).await?;
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct MarketStats {
    pub volume_token_0: u64,
    pub volume_token_1: u64,
    pub fees_token_0: u64,
    pub fees_token_1: u64,
}