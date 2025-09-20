//! Repository layer for data access

use crate::database::{DatabaseManager, Market, Position, Swap, MarketSnapshot};
use anyhow::Result;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

pub struct RepositoryManager {
    db: DatabaseManager,
}

impl RepositoryManager {
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    /// Market repository operations
    pub async fn upsert_market(&self, market: &Market) -> Result<()> {
        // Store in PostgreSQL
        self.db.postgres.upsert_market(market).await?;
        
        // Cache in Redis
        let stats = crate::database::redis::MarketStats {
            market_id: market.id,
            price_24h_change: 0.0, // Calculate from snapshots
            volume_24h: market.total_volume_0.to_string(),
            tvl: market.liquidity.to_string(),
            fees_24h: market.total_fees_0.to_string(),
            apr: 0.0, // Calculate
        };
        self.db.redis.cache_market_stats(market.id, &stats, 300).await?;
        
        // Index in Tantivy
        let _searchable = crate::database::tantivy::SearchableMarket {
            id: market.id,
            address: market.address.clone(),
            token_0: market.token_0.clone(),
            token_1: market.token_1.clone(),
            phase: market.phase.clone(),
            created_at: market.created_at,
        };
        // Note: Would need mutable access to index
        
        Ok(())
    }

    pub async fn get_market_by_address(&self, address: &str) -> Result<Option<Market>> {
        self.db.postgres.get_market_by_address(address).await
    }

    pub async fn get_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        self.db.postgres.get_markets(limit, offset).await
    }

    pub async fn search_markets(&self, query: &str, limit: i64) -> Result<Vec<Market>> {
        // First try PostgreSQL text search
        let markets = self.db.postgres.search_markets(Some(query), limit).await?;
        
        // If not enough results, supplement with Tantivy
        if markets.len() < limit as usize {
            let _search_results = self.db.tantivy.search_markets(query, limit as usize).await?;
            // Would need to convert search results to markets
        }
        
        Ok(markets)
    }

    /// Position repository operations
    pub async fn upsert_position(&self, position: &Position) -> Result<()> {
        // Store in PostgreSQL
        self.db.postgres.upsert_position(position).await?;
        
        // Cache user positions in Redis
        let user_positions = self.db.postgres.get_user_positions(&position.owner, 100, 0).await?;
        let cached_positions: Vec<_> = user_positions
            .iter()
            .map(|p| crate::database::redis::CachedPosition {
                id: p.id,
                market_id: p.market_id,
                liquidity: p.liquidity.to_string(),
                tick_lower: p.tick_lower,
                tick_upper: p.tick_upper,
                tokens_owed_0: p.tokens_owed_0,
                tokens_owed_1: p.tokens_owed_1,
            })
            .collect();
        
        self.db.redis.cache_user_positions(&position.owner, &cached_positions, 300).await?;
        
        Ok(())
    }

    pub async fn get_user_positions(&self, owner: &str) -> Result<Vec<Position>> {
        // Try cache first
        if let Some(_cached) = self.db.redis.get_user_positions(owner).await? {
            // Convert cached positions back to full positions
            // This is simplified - in practice you'd need to fetch full data
        }
        
        // Fallback to database with default pagination
        self.db.postgres.get_user_positions(owner, 100, 0).await
    }

    /// Swap repository operations
    pub async fn insert_swap(&self, swap: &Swap) -> Result<()> {
        // Store in PostgreSQL
        self.db.postgres.insert_swap(swap).await?;
        
        // Publish real-time event
        let swap_event = crate::database::redis::SwapEvent {
            market_id: swap.market_id,
            signature: swap.signature.clone(),
            trader: swap.trader.clone(),
            amount_in: swap.amount_in,
            amount_out: swap.amount_out,
            price: swap.effective_price.unwrap_or_default().to_f64().unwrap_or(0.0),
            timestamp: swap.timestamp,
        };
        self.db.redis.publish_swap_event(&swap_event).await?;
        
        // Update counters
        self.db.redis.increment_swap_counter(swap.market_id).await?;
        
        // Add to recent swaps
        let swap_data = serde_json::to_string(&swap_event)?;
        self.db.redis.add_recent_swap(swap.market_id, &swap_data).await?;
        
        Ok(())
    }

    pub async fn get_market_swaps(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        self.db.postgres.get_market_swaps(&market_id.to_string(), limit, offset).await
    }

    pub async fn get_trader_swaps(&self, trader: &str, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        self.db.postgres.get_trader_swaps(trader, limit, offset).await
    }

    /// Analytics operations
    pub async fn insert_market_snapshot(&self, snapshot: &MarketSnapshot) -> Result<()> {
        self.db.postgres.insert_market_snapshot(snapshot).await
    }

    pub async fn get_market_analytics(&self, market_id: Uuid) -> Result<MarketSnapshot> {
        self.db.postgres.get_market_analytics(&market_id.to_string()).await
    }

    /// Search operations
    pub async fn global_search(&self, query: &str, limit: usize) -> Result<Vec<crate::database::tantivy::SearchResult>> {
        self.db.tantivy.global_search(query, limit).await
    }

    /// Cache operations
    pub async fn get_trending_markets(&self) -> Result<Option<Vec<crate::database::redis::TrendingMarket>>> {
        self.db.redis.get_trending_markets().await
    }

    pub async fn cache_trending_markets(&self, markets: &[crate::database::redis::TrendingMarket]) -> Result<()> {
        self.db.redis.cache_trending_markets(markets, 300).await
    }

    pub async fn get_global_stats(&self) -> Result<Option<crate::database::redis::GlobalStats>> {
        self.db.redis.get_global_stats().await
    }

    pub async fn cache_global_stats(&self, stats: &crate::database::redis::GlobalStats) -> Result<()> {
        self.db.redis.cache_global_stats(stats, 300).await
    }
}
