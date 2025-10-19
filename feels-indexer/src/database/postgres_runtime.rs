//! PostgreSQL connection manager with runtime queries (no compile-time checking)

use super::{Market, Swap, Position, MarketSnapshot, DatabaseOperations};
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use sqlx::Row;
use async_trait::async_trait;

#[derive(Clone)]
pub struct PostgresManager {
    pub pool: PgPool,
}

impl PostgresManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }

    /// Insert or update a market
    pub async fn upsert_market(&self, market: &Market) -> Result<()> {
        let query = r#"
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
        "#;
        
        sqlx::query(query)
            .bind(&market.address)
            .bind(&market.token_0)
            .bind(&market.token_1)
            .bind(market.sqrt_price)
            .bind(market.liquidity)
            .bind(market.current_tick)
            .bind(market.tick_spacing)
            .bind(market.fee_bps)
            .bind(market.is_paused)
            .bind(&market.phase)
            .bind(market.global_lower_tick)
            .bind(market.global_upper_tick)
            .bind(market.fee_growth_global_0)
            .bind(market.fee_growth_global_1)
            .bind(market.total_volume_0)
            .bind(market.total_volume_1)
            .bind(market.total_fees_0)
            .bind(market.total_fees_1)
            .bind(market.swap_count)
            .bind(market.unique_traders)
            .bind(market.last_updated_slot)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a market by address
    pub async fn get_market(&self, address: &str) -> Result<Option<Market>> {
        let query = "SELECT * FROM markets WHERE address = $1";
        
        let result = sqlx::query(query)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;
            
        match result {
            Some(row) => Ok(Some(Market {
                id: row.get("id"),
                address: row.get("address"),
                token_0: row.get("token_0"),
                token_1: row.get("token_1"),
                sqrt_price: row.get("sqrt_price"),
                liquidity: row.get("liquidity"),
                current_tick: row.get("current_tick"),
                tick_spacing: row.get("tick_spacing"),
                fee_bps: row.get("fee_bps"),
                is_paused: row.get("is_paused"),
                phase: row.get("phase"),
                global_lower_tick: row.get("global_lower_tick"),
                global_upper_tick: row.get("global_upper_tick"),
                fee_growth_global_0: row.get("fee_growth_global_0"),
                fee_growth_global_1: row.get("fee_growth_global_1"),
                total_volume_0: row.get("total_volume_0"),
                total_volume_1: row.get("total_volume_1"),
                total_fees_0: row.get("total_fees_0"),
                total_fees_1: row.get("total_fees_1"),
                swap_count: row.get("swap_count"),
                unique_traders: row.get("unique_traders"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_updated_slot: row.get("last_updated_slot"),
            })),
            None => Ok(None),
        }
    }

    /// Get top markets by volume
    pub async fn get_top_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        let query = "SELECT * FROM markets ORDER BY total_volume_0 + total_volume_1 DESC LIMIT $1 OFFSET $2";
        
        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        
        let markets = rows.into_iter().map(|row| Market {
            id: row.get("id"),
            address: row.get("address"),
            token_0: row.get("token_0"),
            token_1: row.get("token_1"),
            sqrt_price: row.get("sqrt_price"),
            liquidity: row.get("liquidity"),
            current_tick: row.get("current_tick"),
            tick_spacing: row.get("tick_spacing"),
            fee_bps: row.get("fee_bps"),
            is_paused: row.get("is_paused"),
            phase: row.get("phase"),
            global_lower_tick: row.get("global_lower_tick"),
            global_upper_tick: row.get("global_upper_tick"),
            fee_growth_global_0: row.get("fee_growth_global_0"),
            fee_growth_global_1: row.get("fee_growth_global_1"),
            total_volume_0: row.get("total_volume_0"),
            total_volume_1: row.get("total_volume_1"),
            total_fees_0: row.get("total_fees_0"),
            total_fees_1: row.get("total_fees_1"),
            swap_count: row.get("swap_count"),
            unique_traders: row.get("unique_traders"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_updated_slot: row.get("last_updated_slot"),
        }).collect();
        
        Ok(markets)
    }

    /// Get markets by liquidity
    pub async fn get_markets_by_liquidity(&self, min_liquidity: rust_decimal::Decimal, limit: i64) -> Result<Vec<Market>> {
        let query = r#"
            SELECT * FROM markets 
            WHERE liquidity >= $1 
            ORDER BY liquidity DESC 
            LIMIT $2
        "#;
        
        let rows = sqlx::query(query)
            .bind(min_liquidity)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        
        let markets = rows.into_iter().map(|row| Market {
            id: row.get("id"),
            address: row.get("address"),
            token_0: row.get("token_0"),
            token_1: row.get("token_1"),
            sqrt_price: row.get("sqrt_price"),
            liquidity: row.get("liquidity"),
            current_tick: row.get("current_tick"),
            tick_spacing: row.get("tick_spacing"),
            fee_bps: row.get("fee_bps"),
            is_paused: row.get("is_paused"),
            phase: row.get("phase"),
            global_lower_tick: row.get("global_lower_tick"),
            global_upper_tick: row.get("global_upper_tick"),
            fee_growth_global_0: row.get("fee_growth_global_0"),
            fee_growth_global_1: row.get("fee_growth_global_1"),
            total_volume_0: row.get("total_volume_0"),
            total_volume_1: row.get("total_volume_1"),
            total_fees_0: row.get("total_fees_0"),
            total_fees_1: row.get("total_fees_1"),
            swap_count: row.get("swap_count"),
            unique_traders: row.get("unique_traders"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_updated_slot: row.get("last_updated_slot"),
        }).collect();
        
        Ok(markets)
    }



    /// Get swaps for a market
    pub async fn get_swaps_for_market(&self, market_id: uuid::Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let query = "SELECT * FROM swaps WHERE market_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3";
        
        let rows = sqlx::query(query)
            .bind(market_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        
        let swaps = rows.into_iter().map(|row| Swap {
            id: row.get("id"),
            signature: row.get("signature"),
            market_id: row.get("market_id"),
            trader: row.get("trader"),
            amount_in: row.get("amount_in"),
            amount_out: row.get("amount_out"),
            token_in: row.get("token_in"),
            token_out: row.get("token_out"),
            sqrt_price_before: row.get("sqrt_price_before"),
            sqrt_price_after: row.get("sqrt_price_after"),
            tick_before: row.get("tick_before"),
            tick_after: row.get("tick_after"),
            liquidity: row.get("liquidity"),
            fee_amount: row.get("fee_amount"),
            timestamp: row.get("timestamp"),
            slot: row.get("slot"),
            block_height: row.get("block_height"),
            price_impact_bps: row.get("price_impact_bps"),
            effective_price: row.get("effective_price"),
        }).collect();
        
        Ok(swaps)
    }

    /// Get swaps by trader
    pub async fn get_swaps_by_trader(&self, trader: &str, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let query = "SELECT * FROM swaps WHERE trader = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3";
        
        let rows = sqlx::query(query)
            .bind(trader)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        
        let swaps = rows.into_iter().map(|row| Swap {
            id: row.get("id"),
            signature: row.get("signature"),
            market_id: row.get("market_id"),
            trader: row.get("trader"),
            amount_in: row.get("amount_in"),
            amount_out: row.get("amount_out"),
            token_in: row.get("token_in"),
            token_out: row.get("token_out"),
            sqrt_price_before: row.get("sqrt_price_before"),
            sqrt_price_after: row.get("sqrt_price_after"),
            tick_before: row.get("tick_before"),
            tick_after: row.get("tick_after"),
            liquidity: row.get("liquidity"),
            fee_amount: row.get("fee_amount"),
            timestamp: row.get("timestamp"),
            slot: row.get("slot"),
            block_height: row.get("block_height"),
            price_impact_bps: row.get("price_impact_bps"),
            effective_price: row.get("effective_price"),
        }).collect();
        
        Ok(swaps)
    }


    /// Insert or update a position
    pub async fn upsert_position(&self, position: &Position) -> Result<()> {
        let query = r#"
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
        "#;
        
        sqlx::query(query)
            .bind(&position.address)
            .bind(position.market_id)
            .bind(&position.owner)
            .bind(position.liquidity)
            .bind(position.tick_lower)
            .bind(position.tick_upper)
            .bind(position.fee_growth_inside_0_last)
            .bind(position.fee_growth_inside_1_last)
            .bind(position.tokens_owed_0)
            .bind(position.tokens_owed_1)
            .bind(position.last_updated_slot)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get positions by owner
    pub async fn get_positions_by_owner(&self, owner: &str) -> Result<Vec<Position>> {
        let query = "SELECT * FROM positions WHERE owner = $1 ORDER BY updated_at DESC";
        
        let rows = sqlx::query(query)
            .bind(owner)
            .fetch_all(&self.pool)
            .await?;
        
        let positions = rows.into_iter().map(|row| Position {
            id: row.get("id"),
            address: row.get("address"),
            market_id: row.get("market_id"),
            owner: row.get("owner"),
            liquidity: row.get("liquidity"),
            tick_lower: row.get("tick_lower"),
            tick_upper: row.get("tick_upper"),
            fee_growth_inside_0_last: row.get("fee_growth_inside_0_last"),
            fee_growth_inside_1_last: row.get("fee_growth_inside_1_last"),
            tokens_owed_0: row.get("tokens_owed_0"),
            tokens_owed_1: row.get("tokens_owed_1"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_updated_slot: row.get("last_updated_slot"),
        }).collect();
        
        Ok(positions)
    }






    /// Insert market snapshot
    pub async fn insert_market_snapshot(&self, snapshot: &MarketSnapshot) -> Result<()> {
        let query = r#"
            INSERT INTO market_snapshots (
                market_id, timestamp, slot, sqrt_price, tick, liquidity,
                volume_0, volume_1, fees_0, fees_1, swap_count,
                tvl_token_0, tvl_token_1, tvl_usd
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
        "#;
        
        sqlx::query(query)
            .bind(snapshot.market_id)
            .bind(snapshot.timestamp)
            .bind(snapshot.slot)
            .bind(snapshot.sqrt_price)
            .bind(snapshot.tick)
            .bind(snapshot.liquidity)
            .bind(snapshot.volume_0)
            .bind(snapshot.volume_1)
            .bind(snapshot.fees_0)
            .bind(snapshot.fees_1)
            .bind(snapshot.swap_count)
            .bind(snapshot.tvl_token_0)
            .bind(snapshot.tvl_token_1)
            .bind(snapshot.tvl_usd)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get market snapshots
    pub async fn get_market_snapshots(&self, market_id: uuid::Uuid, hours: i32) -> Result<Vec<MarketSnapshot>> {
        let query = r#"
            SELECT * FROM market_snapshots 
            WHERE market_id = $1 AND timestamp > NOW() - INTERVAL '$2 hours'
            ORDER BY timestamp DESC
        "#;
        
        let rows = sqlx::query(query)
            .bind(market_id)
            .bind(hours)
            .fetch_all(&self.pool)
            .await?;
        
        let snapshots = rows.into_iter().map(|row| MarketSnapshot {
            id: row.get("id"),
            market_id: row.get("market_id"),
            timestamp: row.get("timestamp"),
            slot: row.get("slot"),
            sqrt_price: row.get("sqrt_price"),
            tick: row.get("tick"),
            liquidity: row.get("liquidity"),
            volume_0: row.get("volume_0"),
            volume_1: row.get("volume_1"),
            fees_0: row.get("fees_0"),
            fees_1: row.get("fees_1"),
            swap_count: row.get("swap_count"),
            tvl_token_0: row.get("tvl_token_0"),
            tvl_token_1: row.get("tvl_token_1"),
            tvl_usd: row.get("tvl_usd"),
        }).collect();
        
        Ok(snapshots)
    }

    /// Get all markets with pagination
    pub async fn get_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        let query = r#"
            SELECT * FROM markets 
            ORDER BY total_volume_0 + total_volume_1 DESC 
            LIMIT $1 OFFSET $2
        "#;
        
        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        let markets: Vec<Market> = rows
            .into_iter()
            .map(|row| Market {
                id: row.get("id"),
                address: row.get("address"),
                token_0: row.get("token_0"),
                token_1: row.get("token_1"),
                sqrt_price: row.get("sqrt_price"),
                liquidity: row.get("liquidity"),
                current_tick: row.get("current_tick"),
                tick_spacing: row.get("tick_spacing"),
                fee_bps: row.get("fee_bps"),
                is_paused: row.get("is_paused"),
                phase: row.get("phase"),
                global_lower_tick: row.get("global_lower_tick"),
                global_upper_tick: row.get("global_upper_tick"),
                fee_growth_global_0: row.get("fee_growth_global_0"),
                fee_growth_global_1: row.get("fee_growth_global_1"),
                total_volume_0: row.get("total_volume_0"),
                total_volume_1: row.get("total_volume_1"),
                total_fees_0: row.get("total_fees_0"),
                total_fees_1: row.get("total_fees_1"),
                swap_count: row.get("swap_count"),
                unique_traders: row.get("unique_traders"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_updated_slot: row.get("last_updated_slot"),
            })
            .collect();
            
        Ok(markets)
    }

    /// Search markets by token address
    pub async fn search_markets(
        &self, 
        token_address: Option<&str>, 
        limit: i64
    ) -> Result<Vec<Market>> {
        let query = if let Some(token) = token_address {
            sqlx::query(
                r#"
                SELECT * FROM markets 
                WHERE token_0 = $1 OR token_1 = $1
                ORDER BY total_volume_0 + total_volume_1 DESC 
                LIMIT $2
                "#
            )
            .bind(token)
            .bind(limit)
        } else {
            sqlx::query(
                r#"
                SELECT * FROM markets 
                ORDER BY total_volume_0 + total_volume_1 DESC 
                LIMIT $1
                "#
            )
            .bind(limit)
        };
        
        let rows = query.fetch_all(&self.pool).await?;
        
        let markets: Vec<Market> = rows
            .into_iter()
            .map(|row| Market {
                id: row.get("id"),
                address: row.get("address"),
                token_0: row.get("token_0"),
                token_1: row.get("token_1"),
                sqrt_price: row.get("sqrt_price"),
                liquidity: row.get("liquidity"),
                current_tick: row.get("current_tick"),
                tick_spacing: row.get("tick_spacing"),
                fee_bps: row.get("fee_bps"),
                is_paused: row.get("is_paused"),
                phase: row.get("phase"),
                global_lower_tick: row.get("global_lower_tick"),
                global_upper_tick: row.get("global_upper_tick"),
                fee_growth_global_0: row.get("fee_growth_global_0"),
                fee_growth_global_1: row.get("fee_growth_global_1"),
                total_volume_0: row.get("total_volume_0"),
                total_volume_1: row.get("total_volume_1"),
                total_fees_0: row.get("total_fees_0"),
                total_fees_1: row.get("total_fees_1"),
                swap_count: row.get("swap_count"),
                unique_traders: row.get("unique_traders"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_updated_slot: row.get("last_updated_slot"),
            })
            .collect();
            
        Ok(markets)
    }

    /// Get positions for a specific user
    pub async fn get_user_positions(
        &self,
        owner: &str,
        limit: i64,
        offset: i64
    ) -> Result<Vec<Position>> {
        let query = r#"
            SELECT * FROM positions 
            WHERE owner = $1
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
        "#;
        
        let rows = sqlx::query(query)
            .bind(owner)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        let positions: Vec<Position> = rows
            .into_iter()
            .map(|row| Position {
                id: row.get("id"),
                address: row.get("address"),
                market_id: row.get("market_id"),
                owner: row.get("owner"),
                liquidity: row.get("liquidity"),
                tick_lower: row.get("tick_lower"),
                tick_upper: row.get("tick_upper"),
                fee_growth_inside_0_last: row.get("fee_growth_inside_0_last"),
                fee_growth_inside_1_last: row.get("fee_growth_inside_1_last"),
                tokens_owed_0: row.get("tokens_owed_0"),
                tokens_owed_1: row.get("tokens_owed_1"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_updated_slot: row.get("last_updated_slot"),
            })
            .collect();
            
        Ok(positions)
    }

    /// Get swaps for a specific market  
    pub async fn get_market_swaps(
        &self,
        market_id: &str,
        limit: i64,
        offset: i64
    ) -> Result<Vec<Swap>> {
        let query = r#"
            SELECT * FROM swaps 
            WHERE market_id = $1::uuid
            ORDER BY timestamp DESC 
            LIMIT $2 OFFSET $3
        "#;
        
        let market_uuid = uuid::Uuid::parse_str(market_id)?;
        
        let rows = sqlx::query(query)
            .bind(market_uuid)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        let swaps: Vec<Swap> = rows
            .into_iter()
            .map(|row| Swap {
                id: row.get("id"),
                signature: row.get("signature"),
                market_id: row.get("market_id"),
                trader: row.get("trader"),
                amount_in: row.get("amount_in"),
                amount_out: row.get("amount_out"),
                token_in: row.get("token_in"),
                token_out: row.get("token_out"),
                sqrt_price_before: row.get("sqrt_price_before"),
                sqrt_price_after: row.get("sqrt_price_after"),
                tick_before: row.get("tick_before"),
                tick_after: row.get("tick_after"),
                liquidity: row.get("liquidity"),
                fee_amount: row.get("fee_amount"),
                timestamp: row.get("timestamp"),
                slot: row.get("slot"),
                block_height: row.get("block_height"),
                price_impact_bps: row.get("price_impact_bps"),
                effective_price: row.get("effective_price"),
            })
            .collect();
            
        Ok(swaps)
    }

    /// Get swaps for a specific trader
    pub async fn get_trader_swaps(
        &self,
        trader: &str,
        limit: i64,
        offset: i64
    ) -> Result<Vec<Swap>> {
        let query = r#"
            SELECT * FROM swaps 
            WHERE trader = $1
            ORDER BY timestamp DESC 
            LIMIT $2 OFFSET $3
        "#;
        
        let rows = sqlx::query(query)
            .bind(trader)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        let swaps: Vec<Swap> = rows
            .into_iter()
            .map(|row| Swap {
                id: row.get("id"),
                signature: row.get("signature"),
                market_id: row.get("market_id"),
                trader: row.get("trader"),
                amount_in: row.get("amount_in"),
                amount_out: row.get("amount_out"),
                token_in: row.get("token_in"),
                token_out: row.get("token_out"),
                sqrt_price_before: row.get("sqrt_price_before"),
                sqrt_price_after: row.get("sqrt_price_after"),
                tick_before: row.get("tick_before"),
                tick_after: row.get("tick_after"),
                liquidity: row.get("liquidity"),
                fee_amount: row.get("fee_amount"),
                timestamp: row.get("timestamp"),
                slot: row.get("slot"),
                block_height: row.get("block_height"),
                price_impact_bps: row.get("price_impact_bps"),
                effective_price: row.get("effective_price"),
            })
            .collect();
            
        Ok(swaps)
    }

    /// Get market analytics for the last 24 hours
    pub async fn get_market_analytics(&self, market_id: &str) -> Result<MarketSnapshot> {
        let query = r#"
            SELECT * FROM market_snapshots
            WHERE market_id = $1::uuid AND timestamp >= NOW() - INTERVAL '24 hours'
            ORDER BY timestamp DESC
            LIMIT 1
        "#;
        
        let market_uuid = uuid::Uuid::parse_str(market_id)?;
        
        let row = sqlx::query(query)
            .bind(market_uuid)
            .fetch_one(&self.pool)
            .await?;
            
        Ok(MarketSnapshot {
            id: row.get("id"),
            market_id: row.get("market_id"),
            timestamp: row.get("timestamp"),
            slot: row.get("slot"),
            sqrt_price: row.get("sqrt_price"),
            tick: row.get("tick"),
            liquidity: row.get("liquidity"),
            volume_0: row.get("volume_0"),
            volume_1: row.get("volume_1"),
            fees_0: row.get("fees_0"),
            fees_1: row.get("fees_1"),
            swap_count: row.get("swap_count"),
            tvl_token_0: row.get("tvl_token_0"),
            tvl_token_1: row.get("tvl_token_1"),
            tvl_usd: row.get("tvl_usd"),
        })
    }
}

#[async_trait]
impl DatabaseOperations for PostgresManager {
    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }
}