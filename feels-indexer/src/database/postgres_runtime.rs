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

    /// Get market by address
    pub async fn get_market_by_address(&self, address: &str) -> Result<Option<Market>> {
        let query = "SELECT * FROM markets WHERE address = $1";
        
        let row = sqlx::query(query)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(row) = row {
            let market = Market {
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
            };
            Ok(Some(market))
        } else {
            Ok(None)
        }
    }

    /// Insert a new swap
    pub async fn insert_swap(&self, swap: &Swap) -> Result<()> {
        let query = r#"
            INSERT INTO swaps (
                signature, market_id, trader, amount_in, amount_out,
                token_in, token_out, sqrt_price_before, sqrt_price_after,
                tick_before, tick_after, liquidity, fee_amount,
                timestamp, slot, block_height, price_impact_bps, effective_price
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#;
        
        sqlx::query(query)
            .bind(&swap.signature)
            .bind(swap.market_id)
            .bind(&swap.trader)
            .bind(swap.amount_in)
            .bind(swap.amount_out)
            .bind(&swap.token_in)
            .bind(&swap.token_out)
            .bind(swap.sqrt_price_before)
            .bind(swap.sqrt_price_after)
            .bind(swap.tick_before)
            .bind(swap.tick_after)
            .bind(swap.liquidity)
            .bind(swap.fee_amount)
            .bind(swap.timestamp)
            .bind(swap.slot)
            .bind(swap.block_height)
            .bind(swap.price_impact_bps)
            .bind(swap.effective_price)
            .execute(&self.pool)
            .await?;
            
        Ok(())
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
    
    // Additional methods required by API handlers
    
    /// Insert market (used by stream handler)
    pub async fn insert_market(&self, market: &Market) -> Result<()> {
        self.upsert_market(market).await
    }
    
    /// Insert position (used by stream handler)
    pub async fn insert_position(&self, position: &Position) -> Result<()> {
        self.upsert_position(position).await
    }
    
    /// Get markets paginated
    pub async fn get_markets_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        self.get_markets(limit, offset).await
    }
    
    /// Get markets count
    pub async fn get_markets_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM markets")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get positions count
    pub async fn get_positions_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM positions")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get position by address
    pub async fn get_position_by_address(&self, address: &str) -> Result<Option<Position>> {
        let row = sqlx::query("SELECT * FROM positions WHERE address = $1")
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(row) = row {
            Ok(Some(Position {
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
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get protocol stats for last 24h
    pub async fn get_protocol_stats_24h(&self) -> Result<ProtocolStats24h> {
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(total_volume_0 + total_volume_1), 0) as total_volume_24h,
                COALESCE(SUM(total_fees_0 + total_fees_1), 0) as total_fees_24h,
                COALESCE(SUM(liquidity), 0) as total_liquidity,
                COALESCE(SUM(unique_traders), 0) as active_traders_24h
            FROM markets
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        use rust_decimal::Decimal;
        
        // Get current time minus 24 hours
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);
        
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(DISTINCT m.id) as total_markets,
                COALESCE(SUM(s.amount_in + s.amount_out), 0) as total_volume_24h,
                COALESCE(SUM(s.fee_amount), 0) as total_fees_24h,
                COALESCE(SUM(m.liquidity), 0) as total_liquidity,
                COUNT(DISTINCT s.trader) FILTER (WHERE s.timestamp > $1) as active_traders_24h
            FROM markets m
            LEFT JOIN swaps s ON s.market_id = m.id AND s.timestamp > $1
            "#
        )
        .bind(cutoff_time)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(ProtocolStats24h {
            total_volume_24h: row.try_get("total_volume_24h").unwrap_or(Decimal::ZERO),
            total_fees_24h: row.try_get("total_fees_24h").unwrap_or(Decimal::ZERO),
            total_liquidity: row.try_get("total_liquidity").unwrap_or(Decimal::ZERO),
            active_traders_24h: row.try_get::<i64, _>("active_traders_24h").unwrap_or(0) as u64,
        })
    }
    
    /// Get market stats
    pub async fn get_market_stats(&self, market_id: uuid::Uuid, _start_time: i64, _end_time: i64) -> Result<MarketStatsData> {
        use rust_decimal::Decimal;
        
        // Get current time minus 24 hours
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);
        
        // Get 24h statistics
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(amount_in + amount_out), 0) as volume_24h,
                COALESCE(SUM(fee_amount), 0) as fees_24h,
                COUNT(*) as swaps_24h,
                COUNT(DISTINCT trader) as unique_traders_24h
            FROM swaps
            WHERE market_id = $1
              AND timestamp > $2
            "#
        )
        .bind(market_id)
        .bind(cutoff_time)
        .fetch_one(&self.pool)
        .await?;
        
        // Calculate price change (compare first and last swap in 24h period)
        let price_change_row = sqlx::query(
            r#"
            WITH first_swap AS (
                SELECT sqrt_price_before
                FROM swaps
                WHERE market_id = $1 AND timestamp > $2
                ORDER BY timestamp ASC
                LIMIT 1
            ),
            last_swap AS (
                SELECT sqrt_price_after
                FROM swaps
                WHERE market_id = $1 AND timestamp > $2
                ORDER BY timestamp DESC
                LIMIT 1
            )
            SELECT 
                COALESCE(first_swap.sqrt_price_before, 0) as first_price,
                COALESCE(last_swap.sqrt_price_after, 0) as last_price
            FROM first_swap, last_swap
            "#
        )
        .bind(market_id)
        .bind(cutoff_time)
        .fetch_optional(&self.pool)
        .await?;
        
        let price_change_24h = if let Some(row) = price_change_row {
            let first_price: Decimal = row.try_get("first_price").unwrap_or(Decimal::ZERO);
            let last_price: Decimal = row.try_get("last_price").unwrap_or(Decimal::ZERO);
            
            if first_price > Decimal::ZERO {
                ((last_price - first_price) / first_price) * Decimal::from(100)
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };
        
        Ok(MarketStatsData {
            volume_24h: row.try_get("volume_24h").unwrap_or(Decimal::ZERO),
            fees_24h: row.try_get("fees_24h").unwrap_or(Decimal::ZERO),
            swaps_24h: row.try_get::<i64, _>("swaps_24h").unwrap_or(0),
            unique_traders_24h: row.try_get::<i64, _>("unique_traders_24h").unwrap_or(0),
            price_change_24h,
            liquidity_change_24h: Decimal::ZERO, // TODO: Calculate from historical data
        })
    }
    
    /// Get swaps by user
    pub async fn get_swaps_by_user(&self, user: &str, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        self.get_trader_swaps(user, limit, offset).await
    }
    
    /// Get swaps count by user
    pub async fn get_swaps_count_by_user(&self, user: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM swaps WHERE trader = $1")
            .bind(user)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get positions by user
    pub async fn get_positions_by_user(&self, user: &str, limit: i64, offset: i64) -> Result<Vec<Position>> {
        self.get_user_positions(user, limit, offset).await
    }
    
    /// Get positions count by user
    pub async fn get_positions_count_by_user(&self, user: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM positions WHERE owner = $1")
            .bind(user)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get recent swaps paginated
    pub async fn get_recent_swaps_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let rows = sqlx::query("SELECT * FROM swaps ORDER BY timestamp DESC LIMIT $1 OFFSET $2")
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
    
    /// Get swaps count
    pub async fn get_swaps_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM swaps")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get swap by signature
    pub async fn get_swap_by_signature(&self, signature: &str) -> Result<Option<Swap>> {
        let row = sqlx::query("SELECT * FROM swaps WHERE signature = $1")
            .bind(signature)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(row) = row {
            Ok(Some(Swap {
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
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Get swaps by market ID
    pub async fn get_swaps_by_market_id(&self, market_id: uuid::Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let rows = sqlx::query("SELECT * FROM swaps WHERE market_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3")
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
    
    /// Get swaps count by market ID
    pub async fn get_swaps_count_by_market_id(&self, market_id: uuid::Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM swaps WHERE market_id = $1")
            .bind(market_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get positions paginated
    pub async fn get_positions_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let rows = sqlx::query("SELECT * FROM positions ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
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
    
    /// Get positions by market ID
    pub async fn get_positions_by_market_id(&self, market_id: uuid::Uuid, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let rows = sqlx::query("SELECT * FROM positions WHERE market_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
            .bind(market_id)
            .bind(limit)
            .bind(offset)
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
    
    /// Get positions count by market ID
    pub async fn get_positions_count_by_market_id(&self, market_id: uuid::Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM positions WHERE market_id = $1")
            .bind(market_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
    
    /// Get recent swaps for market
    pub async fn get_recent_swaps(&self, market_id: uuid::Uuid, limit: i64) -> Result<Vec<Swap>> {
        self.get_swaps_by_market_id(market_id, limit, 0).await
    }
    
    /// Get market floor (placeholder)
    pub async fn get_market_floor(&self, _market_id: uuid::Uuid) -> Result<FloorData> {
        Ok(FloorData {
            floor_tick: 0,
            floor_price: rust_decimal::Decimal::ZERO,
            jitosol_reserves: rust_decimal::Decimal::ZERO,
            circulating_supply: rust_decimal::Decimal::ZERO,
            last_update_slot: 0,
        })
    }
    
    /// Get market OHLCV (placeholder)
    pub async fn get_market_ohlcv(&self, market_id: uuid::Uuid, start_time: i64, end_time: i64, interval: &str) -> Result<Vec<OHLCVData>> {
        use rust_decimal::Decimal;
        
        // Convert interval to seconds
        let interval_seconds: i64 = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            _ => 3600, // default to 1 hour
        };
        
        // Convert timestamps to chrono DateTime
        let start_dt = chrono::DateTime::from_timestamp(start_time, 0)
            .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));
        let end_dt = chrono::DateTime::from_timestamp(end_time, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        
        // Query to generate OHLCV candles from swaps
        let rows = sqlx::query(
            r#"
            WITH time_buckets AS (
                SELECT 
                    FLOOR(EXTRACT(EPOCH FROM timestamp) / $2) * $2 as bucket_ts,
                    sqrt_price_after,
                    (amount_in + amount_out) as volume,
                    timestamp,
                    ROW_NUMBER() OVER (PARTITION BY FLOOR(EXTRACT(EPOCH FROM timestamp) / $2) ORDER BY timestamp ASC) as is_first,
                    ROW_NUMBER() OVER (PARTITION BY FLOOR(EXTRACT(EPOCH FROM timestamp) / $2) ORDER BY timestamp DESC) as is_last
                FROM swaps
                WHERE market_id = $1
                  AND timestamp >= $3
                  AND timestamp <= $4
                ORDER BY timestamp ASC
            ),
            candles AS (
                SELECT 
                    bucket_ts as timestamp,
                    MAX(CASE WHEN is_first = 1 THEN sqrt_price_after END) as open,
                    MAX(sqrt_price_after) as high,
                    MIN(sqrt_price_after) as low,
                    MAX(CASE WHEN is_last = 1 THEN sqrt_price_after END) as close,
                    SUM(volume) as volume
                FROM time_buckets
                GROUP BY bucket_ts
                ORDER BY bucket_ts ASC
            )
            SELECT 
                timestamp,
                COALESCE(open, close, 0) as open,
                COALESCE(high, 0) as high,
                COALESCE(low, 0) as low,
                COALESCE(close, 0) as close,
                COALESCE(volume, 0) as volume
            FROM candles
            "#
        )
        .bind(market_id)
        .bind(interval_seconds)
        .bind(start_dt)
        .bind(end_dt)
        .fetch_all(&self.pool)
        .await?;
        
        let candles = rows.iter()
            .map(|row| {
                OHLCVData {
                    timestamp: row.try_get("timestamp").unwrap_or(0),
                    open: row.try_get("open").unwrap_or(Decimal::ZERO),
                    high: row.try_get("high").unwrap_or(Decimal::ZERO),
                    low: row.try_get("low").unwrap_or(Decimal::ZERO),
                    close: row.try_get("close").unwrap_or(Decimal::ZERO),
                    volume: row.try_get("volume").unwrap_or(Decimal::ZERO),
                }
            })
            .collect();
        
        Ok(candles)
    }
    
    /// Get protocol volume history
    pub async fn get_protocol_volume_history(&self, start_time: i64, end_time: i64) -> Result<Vec<VolumeHistoryData>> {
        use rust_decimal::Decimal;
        
        let start_dt = chrono::DateTime::from_timestamp(start_time, 0)
            .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(30));
        let end_dt = chrono::DateTime::from_timestamp(end_time, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        
        let rows = sqlx::query(
            r#"
            SELECT 
                DATE_TRUNC('day', timestamp) as date,
                COALESCE(SUM(amount_in + amount_out), 0) as volume,
                COALESCE(SUM(fee_amount), 0) as fees,
                COUNT(*) as swap_count
            FROM swaps
            WHERE timestamp >= $1
              AND timestamp <= $2
            GROUP BY DATE_TRUNC('day', timestamp)
            ORDER BY date DESC
            "#
        )
        .bind(start_dt)
        .bind(end_dt)
        .fetch_all(&self.pool)
        .await?;
        
        let history = rows.iter()
            .map(|row| {
                let date: chrono::NaiveDateTime = row.try_get("date").unwrap_or_default();
                VolumeHistoryData {
                    date: date.and_utc().timestamp(),
                    volume: row.try_get("volume").unwrap_or(Decimal::ZERO),
                    fees: row.try_get("fees").unwrap_or(Decimal::ZERO),
                    swap_count: row.try_get::<i64, _>("swap_count").unwrap_or(0),
                }
            })
            .collect();
        
        Ok(history)
    }
    
    /// Get protocol markets (alias for get_markets)
    pub async fn get_protocol_markets(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        self.get_markets(limit, offset).await
    }
}

// Additional structs needed by API handlers

#[derive(Debug)]
pub struct ProtocolStats24h {
    pub total_volume_24h: rust_decimal::Decimal,
    pub total_fees_24h: rust_decimal::Decimal,
    pub total_liquidity: rust_decimal::Decimal,
    pub active_traders_24h: u64,
}

#[derive(Debug)]
pub struct MarketStatsData {
    pub volume_24h: rust_decimal::Decimal,
    pub fees_24h: rust_decimal::Decimal,
    pub swaps_24h: i64,
    pub unique_traders_24h: i64,
    pub price_change_24h: rust_decimal::Decimal,
    pub liquidity_change_24h: rust_decimal::Decimal,
}

#[derive(Debug)]
pub struct FloorData {
    pub floor_tick: i32,
    pub floor_price: rust_decimal::Decimal,
    pub jitosol_reserves: rust_decimal::Decimal,
    pub circulating_supply: rust_decimal::Decimal,
    pub last_update_slot: i64,
}

#[derive(Debug)]
pub struct OHLCVData {
    pub timestamp: i64,
    pub open: rust_decimal::Decimal,
    pub high: rust_decimal::Decimal,
    pub low: rust_decimal::Decimal,
    pub close: rust_decimal::Decimal,
    pub volume: rust_decimal::Decimal,
}

#[derive(Debug)]
pub struct VolumeHistoryData {
    pub date: i64,
    pub volume: rust_decimal::Decimal,
    pub fees: rust_decimal::Decimal,
    pub swap_count: i64,
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