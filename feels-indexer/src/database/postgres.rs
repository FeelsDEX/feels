//! PostgreSQL database manager

use super::{DatabaseOperations, Market, Position, Swap, MarketSnapshot};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

pub struct PostgresManager {
    pool: PgPool,
}

impl PostgresManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }

    /// Insert or update a market
    pub async fn upsert_market(&self, market: &Market) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO markets (
                address, token_0, token_1, sqrt_price, liquidity, current_tick,
                tick_spacing, fee_bps, is_paused, phase, global_lower_tick,
                global_upper_tick, fee_growth_global_0, fee_growth_global_1,
                total_volume_0, total_volume_1, total_fees_0, total_fees_1,
                swap_count, unique_traders, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21
            )
            ON CONFLICT (address) DO UPDATE SET
                sqrt_price = EXCLUDED.sqrt_price,
                liquidity = EXCLUDED.liquidity,
                current_tick = EXCLUDED.current_tick,
                is_paused = EXCLUDED.is_paused,
                phase = EXCLUDED.phase,
                fee_growth_global_0 = EXCLUDED.fee_growth_global_0,
                fee_growth_global_1 = EXCLUDED.fee_growth_global_1,
                total_volume_0 = EXCLUDED.total_volume_0,
                total_volume_1 = EXCLUDED.total_volume_1,
                total_fees_0 = EXCLUDED.total_fees_0,
                total_fees_1 = EXCLUDED.total_fees_1,
                swap_count = EXCLUDED.swap_count,
                unique_traders = EXCLUDED.unique_traders,
                last_updated_slot = EXCLUDED.last_updated_slot,
                updated_at = NOW()
            "#,
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
            market.last_updated_slot
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get market by address
    pub async fn get_market_by_address(&self, address: &str) -> Result<Option<Market>> {
        let market = sqlx::query_as!(
            Market,
            "SELECT * FROM markets WHERE address = $1",
            address
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(market)
    }

    /// Get all markets with pagination
    pub async fn get_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        let markets = sqlx::query_as!(
            Market,
            "SELECT * FROM markets ORDER BY total_volume_0 + total_volume_1 DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(markets)
    }

    /// Search markets by token symbols
    pub async fn search_markets(&self, query: &str, limit: i64) -> Result<Vec<Market>> {
        let markets = sqlx::query_as!(
            Market,
            r#"
            SELECT * FROM markets 
            WHERE token_0 ILIKE $1 OR token_1 ILIKE $1 
               OR (token_0 || '/' || token_1) ILIKE $1
            ORDER BY total_volume_0 + total_volume_1 DESC 
            LIMIT $2
            "#,
            format!("%{}%", query),
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(markets)
    }

    /// Insert a new swap
    pub async fn insert_swap(&self, swap: &Swap) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO swaps (
                signature, market_id, trader, amount_in, amount_out,
                token_in, token_out, sqrt_price_before, sqrt_price_after,
                tick_before, tick_after, liquidity, fee_amount,
                timestamp, slot, block_height, price_impact_bps, effective_price
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            ON CONFLICT (signature) DO NOTHING
            "#,
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
            swap.effective_price
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get swaps for a market
    pub async fn get_market_swaps(
        &self,
        market_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Swap>> {
        let swaps = sqlx::query_as!(
            Swap,
            "SELECT * FROM swaps WHERE market_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
            market_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(swaps)
    }

    /// Get swaps for a trader
    pub async fn get_trader_swaps(
        &self,
        trader: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Swap>> {
        let swaps = sqlx::query_as!(
            Swap,
            "SELECT * FROM swaps WHERE trader = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
            trader,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(swaps)
    }

    /// Insert or update a position
    pub async fn upsert_position(&self, position: &Position) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO positions (
                address, market_id, owner, liquidity, tick_lower, tick_upper,
                fee_growth_inside_0_last, fee_growth_inside_1_last,
                tokens_owed_0, tokens_owed_1, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            ON CONFLICT (address) DO UPDATE SET
                liquidity = EXCLUDED.liquidity,
                fee_growth_inside_0_last = EXCLUDED.fee_growth_inside_0_last,
                fee_growth_inside_1_last = EXCLUDED.fee_growth_inside_1_last,
                tokens_owed_0 = EXCLUDED.tokens_owed_0,
                tokens_owed_1 = EXCLUDED.tokens_owed_1,
                last_updated_slot = EXCLUDED.last_updated_slot,
                updated_at = NOW()
            "#,
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
            position.last_updated_slot
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get positions for an owner
    pub async fn get_user_positions(&self, owner: &str) -> Result<Vec<Position>> {
        let positions = sqlx::query_as!(
            Position,
            "SELECT * FROM positions WHERE owner = $1 ORDER BY updated_at DESC",
            owner
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(positions)
    }

    /// Insert market snapshot
    pub async fn insert_market_snapshot(&self, snapshot: &MarketSnapshot) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO market_snapshots (
                market_id, timestamp, slot, sqrt_price, tick, liquidity,
                volume_0, volume_1, fees_0, fees_1, swap_count,
                tvl_token_0, tvl_token_1, tvl_usd
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            ON CONFLICT (market_id, timestamp) DO UPDATE SET
                sqrt_price = EXCLUDED.sqrt_price,
                tick = EXCLUDED.tick,
                liquidity = EXCLUDED.liquidity,
                volume_0 = EXCLUDED.volume_0,
                volume_1 = EXCLUDED.volume_1,
                fees_0 = EXCLUDED.fees_0,
                fees_1 = EXCLUDED.fees_1,
                swap_count = EXCLUDED.swap_count,
                tvl_token_0 = EXCLUDED.tvl_token_0,
                tvl_token_1 = EXCLUDED.tvl_token_1,
                tvl_usd = EXCLUDED.tvl_usd
            "#,
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
            snapshot.tvl_usd
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get market analytics
    pub async fn get_market_analytics(&self, market_id: Uuid, hours: i32) -> Result<Vec<MarketSnapshot>> {
        let snapshots = sqlx::query_as!(
            MarketSnapshot,
            r#"
            SELECT * FROM market_snapshots 
            WHERE market_id = $1 AND timestamp > NOW() - INTERVAL '%d hours'
            ORDER BY timestamp ASC
            "#,
            market_id,
            hours
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(snapshots)
    }
}

#[async_trait]
impl DatabaseOperations for PostgresManager {
    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
