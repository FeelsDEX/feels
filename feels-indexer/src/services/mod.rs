//! Business logic services

use crate::database::{Market, Position, Swap, MarketSnapshot};
use crate::repositories::RepositoryManager;
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

pub struct ServiceManager {
    repos: RepositoryManager,
}

impl ServiceManager {
    pub fn new(repos: RepositoryManager) -> Self {
        Self { repos }
    }

    /// Market service operations
    pub async fn process_market_update(&self, market_data: &Market) -> Result<()> {
        // Calculate derived metrics
        let market = market_data.clone();
        
        // Update market in database
        self.repos.upsert_market(&market).await?;
        
        // Create snapshot for analytics
        let snapshot = MarketSnapshot {
            id: Uuid::new_v4(),
            market_id: market.id,
            timestamp: chrono::Utc::now(),
            slot: market.last_updated_slot,
            sqrt_price: market.sqrt_price,
            tick: market.current_tick,
            liquidity: market.liquidity,
            volume_0: Decimal::ZERO, // Calculate from recent swaps
            volume_1: Decimal::ZERO,
            fees_0: Decimal::ZERO,
            fees_1: Decimal::ZERO,
            swap_count: 0,
            tvl_token_0: market.liquidity, // Simplified
            tvl_token_1: market.liquidity,
            tvl_usd: None, // Would need price oracle
        };
        
        self.repos.insert_market_snapshot(&snapshot).await?;
        
        Ok(())
    }

    pub async fn get_market_with_stats(&self, address: &str) -> Result<Option<MarketWithStats>> {
        if let Some(market) = self.repos.get_market_by_address(address).await? {
            // Get recent analytics
            let analytics = self.repos.get_market_analytics(market.id).await?;
            
            // Calculate 24h stats from the snapshot
            let volume_24h = analytics.volume_0 + analytics.volume_1;
            let price_change_24h = 0.0; // TODO: Calculate actual price change from historical data
            
            Ok(Some(MarketWithStats {
                market,
                volume_24h,
                price_change_24h,
                analytics: vec![analytics],
            }))
        } else {
            Ok(None)
        }
    }

    /// Position service operations
    pub async fn process_position_update(&self, position: &Position) -> Result<()> {
        self.repos.upsert_position(position).await
    }

    pub async fn get_user_portfolio(&self, owner: &str) -> Result<UserPortfolio> {
        let positions = self.repos.get_user_positions(owner).await?;
        let swaps = self.repos.get_trader_swaps(owner, 100, 0).await?;
        
        // Calculate portfolio metrics
        let total_positions = positions.len();
        let total_swaps = swaps.len();
        
        // Calculate total value (would need price data)
        let total_value_usd = Decimal::ZERO;
        
        // Calculate PnL (would need historical data)
        let total_pnl_usd = Decimal::ZERO;
        
        Ok(UserPortfolio {
            owner: owner.to_string(),
            positions,
            recent_swaps: swaps,
            total_positions,
            total_swaps,
            total_value_usd,
            total_pnl_usd,
        })
    }

    /// Swap service operations
    pub async fn process_swap(&self, swap: &Swap) -> Result<()> {
        // Calculate derived metrics
        let mut swap = swap.clone();
        
        // Calculate price impact
        if swap.price_impact_bps.is_none() {
            swap.price_impact_bps = Some(calculate_price_impact(
                swap.sqrt_price_before,
                swap.sqrt_price_after,
            ));
        }
        
        // Calculate effective price
        if swap.effective_price.is_none() {
            swap.effective_price = Some(calculate_effective_price(
                swap.amount_in,
                swap.amount_out,
            ));
        }
        
        self.repos.insert_swap(&swap).await
    }

    /// Analytics service operations
    pub async fn get_trending_markets(&self, limit: usize) -> Result<Vec<crate::database::redis::TrendingMarket>> {
        // Try cache first
        if let Some(cached) = self.repos.get_trending_markets().await? {
            return Ok(cached);
        }
        
        // Calculate trending markets from database
        let markets = self.repos.get_markets(100, 0).await?;
        let mut trending = Vec::new();
        
        for (rank, market) in markets.iter().take(limit).enumerate() {
            // Get 24h analytics
            let analytics = self.repos.get_market_analytics(market.id).await?;
            
            // Get 24h volume and price change
            let volume_24h = (analytics.volume_0 + analytics.volume_1).to_string();
            let price_change_24h = 0.0; // TODO: Calculate actual price change from historical data
            
            trending.push(crate::database::redis::TrendingMarket {
                market_id: market.id,
                token_0: market.token_0.clone(),
                token_1: market.token_1.clone(),
                volume_24h,
                price_change_24h,
                rank: rank as i32 + 1,
            });
        }
        
        // Cache the results
        self.repos.cache_trending_markets(&trending).await?;
        
        Ok(trending)
    }

    pub async fn get_global_stats(&self) -> Result<crate::database::redis::GlobalStats> {
        // Try cache first
        if let Some(cached) = self.repos.get_global_stats().await? {
            return Ok(cached);
        }
        
        // Calculate from database
        let markets = self.repos.get_markets(1000, 0).await?;
        
        let total_markets = markets.len() as i64;
        let total_volume_24h = markets
            .iter()
            .map(|m| m.total_volume_0 + m.total_volume_1)
            .sum::<Decimal>()
            .to_string();
        
        let total_tvl = markets
            .iter()
            .map(|m| m.liquidity)
            .sum::<Decimal>()
            .to_string();
        
        let total_fees_24h = markets
            .iter()
            .map(|m| m.total_fees_0 + m.total_fees_1)
            .sum::<Decimal>()
            .to_string();
        
        let stats = crate::database::redis::GlobalStats {
            total_markets,
            total_volume_24h,
            total_tvl,
            total_fees_24h,
            active_traders_24h: 0, // Would need to calculate from swaps
            updated_at: chrono::Utc::now(),
        };
        
        // Cache the results
        self.repos.cache_global_stats(&stats).await?;
        
        Ok(stats)
    }

    /// Search service operations
    pub async fn search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        let results = self.repos.global_search(query, limit).await?;
        
        let mut markets = Vec::new();
        let mut positions = Vec::new();
        let mut swaps = Vec::new();
        
        for result in results {
            match result.content_type.as_str() {
                "market" => markets.push(result),
                "position" => positions.push(result),
                "swap" => swaps.push(result),
                _ => {}
            }
        }
        
        Ok(SearchResults {
            markets,
            positions,
            swaps,
        })
    }
}

// Helper functions
fn calculate_price_change(old_sqrt_price: Decimal, new_sqrt_price: Decimal) -> f64 {
    if old_sqrt_price.is_zero() {
        return 0.0;
    }
    
    let old_price = old_sqrt_price * old_sqrt_price;
    let new_price = new_sqrt_price * new_sqrt_price;
    
    ((new_price - old_price) / old_price * Decimal::from(100))
        .to_f64()
        .unwrap_or(0.0)
}

fn calculate_price_impact(sqrt_price_before: Decimal, sqrt_price_after: Decimal) -> i16 {
    let price_before = sqrt_price_before * sqrt_price_before;
    let price_after = sqrt_price_after * sqrt_price_after;
    
    if price_before.is_zero() {
        return 0;
    }
    
    let impact = ((price_after - price_before).abs() / price_before * Decimal::from(10000))
        .to_i64()
        .unwrap_or(0) as i16;
    
    impact.min(10000) // Cap at 100%
}

fn calculate_effective_price(amount_in: i64, amount_out: i64) -> Decimal {
    if amount_out == 0 {
        return Decimal::ZERO;
    }
    
    Decimal::from(amount_in) / Decimal::from(amount_out)
}

// Response types
#[derive(Debug, Clone)]
pub struct MarketWithStats {
    pub market: Market,
    pub volume_24h: Decimal,
    pub price_change_24h: f64,
    pub analytics: Vec<MarketSnapshot>,
}

#[derive(Debug, Clone)]
pub struct UserPortfolio {
    pub owner: String,
    pub positions: Vec<Position>,
    pub recent_swaps: Vec<Swap>,
    pub total_positions: usize,
    pub total_swaps: usize,
    pub total_value_usd: Decimal,
    pub total_pnl_usd: Decimal,
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub markets: Vec<crate::database::tantivy::SearchResult>,
    pub positions: Vec<crate::database::tantivy::SearchResult>,
    pub swaps: Vec<crate::database::tantivy::SearchResult>,
}
