//! PostgreSQL operations for the indexer

use super::postgres::PostgresManager;
use super::{Market, Position, Swap};
use anyhow::Result;
use sqlx::{query, query_as};
use uuid::Uuid;

impl PostgresManager {
    /// Upsert a market record
    pub async fn upsert_market(&self, market: &Market) -> Result<()> {
        query!(
            r#"
            INSERT INTO markets (
                id, address, token_0, token_1, sqrt_price, liquidity, current_tick,
                tick_spacing, fee_bps, is_paused, phase, global_lower_tick, global_upper_tick,
                fee_growth_global_0, fee_growth_global_1, total_volume_0, total_volume_1,
                total_fees_0, total_fees_1, swap_count, unique_traders, created_at,
                updated_at, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                $16, $17, $18, $19, $20, $21, $22, $23, $24
            )
            ON CONFLICT (address) DO UPDATE SET
                sqrt_price = EXCLUDED.sqrt_price,
                liquidity = EXCLUDED.liquidity,
                current_tick = EXCLUDED.current_tick,
                fee_bps = EXCLUDED.fee_bps,
                is_paused = EXCLUDED.is_paused,
                phase = EXCLUDED.phase,
                global_lower_tick = EXCLUDED.global_lower_tick,
                global_upper_tick = EXCLUDED.global_upper_tick,
                fee_growth_global_0 = EXCLUDED.fee_growth_global_0,
                fee_growth_global_1 = EXCLUDED.fee_growth_global_1,
                updated_at = EXCLUDED.updated_at,
                last_updated_slot = EXCLUDED.last_updated_slot
            "#,
            market.id,
            market.address,
            market.token_0,
            market.token_1,
            market.sqrt_price,
            market.liquidity,
            market.current_tick,
            market.tick_spacing,
            market.fee_bps,
            market.is_paused,
            market.phase,
            market.global_lower_tick,
            market.global_upper_tick,
            market.fee_growth_global_0,
            market.fee_growth_global_1,
            market.total_volume_0,
            market.total_volume_1,
            market.total_fees_0,
            market.total_fees_1,
            market.swap_count,
            market.unique_traders,
            market.created_at,
            market.updated_at,
            market.last_updated_slot,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upsert a position record
    pub async fn upsert_position(&self, position: &Position) -> Result<()> {
        query!(
            r#"
            INSERT INTO positions (
                id, address, market_id, owner, liquidity, tick_lower, tick_upper,
                fee_growth_inside_0_last, fee_growth_inside_1_last, tokens_owed_0,
                tokens_owed_1, created_at, updated_at, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            ON CONFLICT (address) DO UPDATE SET
                liquidity = EXCLUDED.liquidity,
                fee_growth_inside_0_last = EXCLUDED.fee_growth_inside_0_last,
                fee_growth_inside_1_last = EXCLUDED.fee_growth_inside_1_last,
                tokens_owed_0 = EXCLUDED.tokens_owed_0,
                tokens_owed_1 = EXCLUDED.tokens_owed_1,
                updated_at = EXCLUDED.updated_at,
                last_updated_slot = EXCLUDED.last_updated_slot
            "#,
            position.id,
            position.address,
            position.market_id,
            position.owner,
            position.liquidity,
            position.tick_lower,
            position.tick_upper,
            position.fee_growth_inside_0_last,
            position.fee_growth_inside_1_last,
            position.tokens_owed_0,
            position.tokens_owed_1,
            position.created_at,
            position.updated_at,
            position.last_updated_slot,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a new swap record
    pub async fn insert_swap(&self, swap: &Swap) -> Result<()> {
        query!(
            r#"
            INSERT INTO swaps (
                id, signature, market_id, trader, amount_in, amount_out,
                token_in, token_out, sqrt_price_before, sqrt_price_after,
                tick_before, tick_after, liquidity, fee_amount, timestamp,
                slot, block_height, price_impact_bps, effective_price
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19
            )
            "#,
            swap.id,
            swap.signature,
            swap.market_id,
            swap.trader,
            swap.amount_in,
            swap.amount_out,
            swap.token_in,
            swap.token_out,
            swap.sqrt_price_before,
            swap.sqrt_price_after,
            swap.tick_before,
            swap.tick_after,
            swap.liquidity,
            swap.fee_amount,
            swap.timestamp,
            swap.slot,
            swap.block_height,
            swap.price_impact_bps,
            swap.effective_price,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get market by address
    pub async fn get_market_by_address(&self, address: &str) -> Result<Option<Market>> {
        let market = query_as!(
            Market,
            r#"
            SELECT * FROM markets WHERE address = $1
            "#,
            address
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(market)
    }

    /// Get all active markets
    pub async fn get_active_markets(&self) -> Result<Vec<Market>> {
        let markets = query_as!(
            Market,
            r#"
            SELECT * FROM markets WHERE is_paused = false
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(markets)
    }

    /// Get recent swaps for a market
    pub async fn get_recent_swaps(&self, market_id: Uuid, limit: i64) -> Result<Vec<Swap>> {
        let swaps = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps 
            WHERE market_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            market_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(swaps)
    }
    
    /// Get recent swaps across all markets with pagination
    pub async fn get_recent_swaps_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let swaps = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps
            ORDER BY timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(swaps)
    }
    
    /// Get total count of swaps
    pub async fn get_swaps_count(&self) -> Result<i64> {
        let count = query!(
            r#"
            SELECT COUNT(*) as count FROM swaps
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.count.unwrap_or(0))
    }
    
    /// Get swap by signature
    pub async fn get_swap_by_signature(&self, signature: &str) -> Result<Option<Swap>> {
        let swap = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps
            WHERE signature = $1
            LIMIT 1
            "#,
            signature
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(swap)
    }
    
    /// Get swaps by market ID with pagination
    pub async fn get_swaps_by_market_id(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let swaps = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps
            WHERE market_id = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
            market_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(swaps)
    }
    
    /// Get swaps count by market ID
    pub async fn get_swaps_count_by_market_id(&self, market_id: Uuid) -> Result<i64> {
        let count = query!(
            r#"
            SELECT COUNT(*) as count FROM swaps
            WHERE market_id = $1
            "#,
            market_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.count.unwrap_or(0))
    }
    
    /// Get all positions with pagination
    pub async fn get_positions_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let positions = query_as!(
            Position,
            r#"
            SELECT * FROM positions
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(positions)
    }
    
    /// Get total count of positions
    pub async fn get_positions_count(&self) -> Result<i64> {
        let count = query!(
            r#"
            SELECT COUNT(*) as count FROM positions
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.count.unwrap_or(0))
    }
    
    /// Get position by address
    pub async fn get_position_by_address(&self, address: &str) -> Result<Option<Position>> {
        let position = query_as!(
            Position,
            r#"
            SELECT * FROM positions
            WHERE address = $1
            LIMIT 1
            "#,
            address
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(position)
    }
    
    /// Get positions by market ID with pagination
    pub async fn get_positions_by_market_id(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let positions = query_as!(
            Position,
            r#"
            SELECT * FROM positions
            WHERE market_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            market_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(positions)
    }
    
    /// Get positions count by market ID
    pub async fn get_positions_count_by_market_id(&self, market_id: Uuid) -> Result<i64> {
        let count = query!(
            r#"
            SELECT COUNT(*) as count FROM positions
            WHERE market_id = $1
            "#,
            market_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.count.unwrap_or(0))
    }
    
    
    /// Get protocol stats for last 24 hours
    pub async fn get_protocol_stats_24h(&self) -> Result<ProtocolStats24h> {
        let now = chrono::Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        
        let stats = query!(
            r#"
            SELECT 
                COALESCE(SUM(amount_in), 0) as total_volume_24h,
                COALESCE(SUM(fee_amount), 0) as total_fees_24h,
                COUNT(DISTINCT trader) as active_traders_24h
            FROM swaps
            WHERE timestamp > $1
            "#,
            twenty_four_hours_ago
        )
        .fetch_one(&self.pool)
        .await?;
        
        // Get total liquidity from markets
        let liquidity_result = query!(
            r#"
            SELECT COALESCE(SUM(liquidity), 0) as total_liquidity
            FROM markets
            WHERE is_paused = false
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(ProtocolStats24h {
            total_volume_24h: stats.total_volume_24h.unwrap_or(rust_decimal::Decimal::ZERO),
            total_fees_24h: stats.total_fees_24h.unwrap_or(rust_decimal::Decimal::ZERO),
            total_liquidity: liquidity_result.total_liquidity.unwrap_or(rust_decimal::Decimal::ZERO),
            active_traders_24h: stats.active_traders_24h.unwrap_or(0) as u64,
        })
    }
    
    /// Get all markets with pagination
    pub async fn get_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        let markets = query_as!(
            Market,
            r#"
            SELECT * FROM markets
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(markets)
    }
    
    /// Get total count of markets
    pub async fn get_markets_count(&self) -> Result<i64> {
        let count = query!(
            r#"
            SELECT COUNT(*) as count FROM markets
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.count.unwrap_or(0))
    }
    
    /// Search markets by token addresses
    pub async fn search_markets(&self, query: &str, limit: i64) -> Result<Vec<Market>> {
        let markets = query_as!(
            Market,
            r#"
            SELECT * FROM markets
            WHERE token_0 ILIKE $1 OR token_1 ILIKE $1
            ORDER BY swap_count DESC
            LIMIT $2
            "#,
            format!("%{}%", query),
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(markets)
    }
    
    /// Get positions by user address
    pub async fn get_user_positions(&self, user: &str, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let positions = query_as!(
            Position,
            r#"
            SELECT * FROM positions
            WHERE owner = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(positions)
    }
    
    /// Get swaps by trader address
    pub async fn get_swaps_by_trader(&self, trader: &str, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let swaps = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps
            WHERE trader = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
            trader,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(swaps)
    }
    
    /// Get market by ID
    pub async fn get_market_by_id(&self, id: Uuid) -> Result<Option<Market>> {
        let market = query_as!(
            Market,
            r#"
            SELECT * FROM markets
            WHERE id = $1
            LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(market)
    }
    
    /// Get position by ID
    pub async fn get_position_by_id(&self, id: Uuid) -> Result<Option<Position>> {
        let position = query_as!(
            Position,
            r#"
            SELECT * FROM positions
            WHERE id = $1
            LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(position)
    }
    
    /// Get swap by ID
    pub async fn get_swap_by_id(&self, id: Uuid) -> Result<Option<Swap>> {
        let swap = query_as!(
            Swap,
            r#"
            SELECT * FROM swaps
            WHERE id = $1
            LIMIT 1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(swap)
    }
    
    /// Get swaps by market ID (alias for get_swaps_by_market_id)
    pub async fn get_market_swaps(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        self.get_swaps_by_market_id(market_id, limit, offset).await
    }
    
    /// Get swaps by trader
    pub async fn get_trader_swaps(&self, trader: &str, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        self.get_swaps_by_trader(trader, limit, offset).await
    }
    
    /// Insert market snapshot for analytics
    pub async fn insert_market_snapshot(&self, snapshot: &super::MarketSnapshot) -> Result<()> {
        query!(
            r#"
            INSERT INTO market_snapshots (
                id, market_id, timestamp, slot, sqrt_price, tick, liquidity,
                volume_0, volume_1, fees_0, fees_1, swap_count, tvl_token_0,
                tvl_token_1, tvl_usd
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            "#,
            snapshot.id,
            snapshot.market_id,
            snapshot.timestamp,
            snapshot.slot,
            snapshot.sqrt_price,
            snapshot.tick,
            snapshot.liquidity,
            snapshot.volume_0,
            snapshot.volume_1,
            snapshot.fees_0,
            snapshot.fees_1,
            snapshot.swap_count,
            snapshot.tvl_token_0,
            snapshot.tvl_token_1,
            snapshot.tvl_usd,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get market analytics for a given time period
    pub async fn get_market_analytics(&self, market_id: Uuid, hours: i32) -> Result<Vec<super::MarketSnapshot>> {
        let since = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
        
        let snapshots = query_as!(
            super::MarketSnapshot,
            r#"
            SELECT * FROM market_snapshots
            WHERE market_id = $1 AND timestamp > $2
            ORDER BY timestamp ASC
            "#,
            market_id,
            since
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(snapshots)
    }
}

/// Struct for protocol stats
pub struct ProtocolStats24h {
    pub total_volume_24h: rust_decimal::Decimal,
    pub total_fees_24h: rust_decimal::Decimal,
    pub total_liquidity: rust_decimal::Decimal,
    pub active_traders_24h: u64,
}