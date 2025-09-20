//! Runtime PostgreSQL operations

use super::{Market, Position, Swap};
use super::postgres_runtime::PostgresManager;
use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

impl PostgresManager {
    /// Insert a market
    pub async fn insert_market(&self, market: &Market) -> Result<()> {
        let query = r#"
            INSERT INTO markets (
                id, address, token_0, token_1, sqrt_price, liquidity, current_tick,
                tick_spacing, fee_bps, is_paused, phase, global_lower_tick,
                global_upper_tick, fee_growth_global_0, fee_growth_global_1,
                total_volume_0, total_volume_1, total_fees_0, total_fees_1,
                swap_count, unique_traders, created_at, updated_at, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22, $23
            )
        "#;
        
        sqlx::query(query)
            .bind(market.id)
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
            .bind(market.created_at)
            .bind(market.updated_at)
            .bind(market.last_updated_slot)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Insert a position
    pub async fn insert_position(&self, position: &Position) -> Result<()> {
        let query = r#"
            INSERT INTO positions (
                id, address, market_id, owner, liquidity, tick_lower, tick_upper,
                fee_growth_inside_0_last, fee_growth_inside_1_last,
                tokens_owed_0, tokens_owed_1, created_at, updated_at, last_updated_slot
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
        "#;
        
        sqlx::query(query)
            .bind(position.id)
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
            .bind(position.created_at)
            .bind(position.updated_at)
            .bind(position.last_updated_slot)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Insert a swap
    pub async fn insert_swap(&self, swap: &Swap) -> Result<()> {
        let query = r#"
            INSERT INTO swaps (
                id, signature, market_id, trader, amount_in, amount_out,
                token_in, token_out, sqrt_price_before, sqrt_price_after,
                tick_before, tick_after, liquidity, fee_amount, timestamp,
                slot, block_height, price_impact_bps, effective_price
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19
            )
        "#;
        
        sqlx::query(query)
            .bind(swap.id)
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

    /// Get markets paginated
    pub async fn get_markets_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Market>> {
        let query = "SELECT * FROM markets ORDER BY created_at DESC LIMIT $1 OFFSET $2";
        
        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Market {
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
        }).collect())
    }

    /// Get markets count
    pub async fn get_markets_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM markets")
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("count"))
    }

    /// Get recent swaps
    pub async fn get_recent_swaps(&self, market_id: Uuid, limit: i64) -> Result<Vec<Swap>> {
        let query = "SELECT * FROM swaps WHERE market_id = $1 ORDER BY timestamp DESC LIMIT $2";
        
        let rows = sqlx::query(query)
            .bind(market_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Swap {
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
        }).collect())
    }
    
    /// Get recent swaps across all markets with pagination
    pub async fn get_recent_swaps_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let query = "SELECT * FROM swaps ORDER BY timestamp DESC LIMIT $1 OFFSET $2";
        
        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Swap {
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
        }).collect())
    }
    
    /// Get total count of swaps
    pub async fn get_swaps_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM swaps")
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("count"))
    }
    
    /// Get swap by signature
    pub async fn get_swap_by_signature(&self, signature: &str) -> Result<Option<Swap>> {
        let query = "SELECT * FROM swaps WHERE signature = $1 LIMIT 1";
        
        let result = sqlx::query(query)
            .bind(signature)
            .fetch_optional(&self.pool)
            .await?;
            
        match result {
            Some(row) => Ok(Some(Swap {
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
            })),
            None => Ok(None),
        }
    }
    
    /// Get swaps by market ID with pagination
    pub async fn get_swaps_by_market_id(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Swap>> {
        let query = "SELECT * FROM swaps WHERE market_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3";
        
        let rows = sqlx::query(query)
            .bind(market_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Swap {
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
        }).collect())
    }
    
    /// Get swaps count by market ID
    pub async fn get_swaps_count_by_market_id(&self, market_id: Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM swaps WHERE market_id = $1")
            .bind(market_id)
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("count"))
    }
    
    /// Get all positions with pagination
    pub async fn get_positions_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let query = "SELECT * FROM positions ORDER BY created_at DESC LIMIT $1 OFFSET $2";
        
        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Position {
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
        }).collect())
    }
    
    /// Get total count of positions
    pub async fn get_positions_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM positions")
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("count"))
    }
    
    /// Get position by address
    pub async fn get_position_by_address(&self, address: &str) -> Result<Option<Position>> {
        let query = "SELECT * FROM positions WHERE address = $1 LIMIT 1";
        
        let result = sqlx::query(query)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?;
            
        match result {
            Some(row) => Ok(Some(Position {
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
            })),
            None => Ok(None),
        }
    }
    
    /// Get positions by market ID with pagination
    pub async fn get_positions_by_market_id(&self, market_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Position>> {
        let query = "SELECT * FROM positions WHERE market_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3";
        
        let rows = sqlx::query(query)
            .bind(market_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
        Ok(rows.into_iter().map(|row| Position {
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
        }).collect())
    }
    
    /// Get positions count by market ID
    pub async fn get_positions_count_by_market_id(&self, market_id: Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM positions WHERE market_id = $1")
            .bind(market_id)
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("count"))
    }
    
    /// Get market by address
    pub async fn get_market_by_address(&self, address: &str) -> Result<Option<Market>> {
        let query = "SELECT * FROM markets WHERE address = $1 LIMIT 1";
        
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
    
    /// Get protocol stats for last 24 hours
    pub async fn get_protocol_stats_24h(&self) -> Result<ProtocolStats24h> {
        let now = chrono::Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        
        let stats = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(amount_in), 0) as total_volume_24h,
                COALESCE(SUM(fee_amount), 0) as total_fees_24h,
                COUNT(DISTINCT trader) as active_traders_24h
            FROM swaps
            WHERE timestamp > $1
            "#
        )
        .bind(twenty_four_hours_ago)
        .fetch_one(&self.pool)
        .await?;
        
        // Get total liquidity from markets
        let liquidity_result = sqlx::query(
            r#"
            SELECT COALESCE(SUM(liquidity), 0) as total_liquidity
            FROM markets
            WHERE active = true
            "#
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(ProtocolStats24h {
            total_volume_24h: stats.get("total_volume_24h"),
            total_fees_24h: stats.get("total_fees_24h"),
            total_liquidity: liquidity_result.get("total_liquidity"),
            active_traders_24h: stats.get::<i64, _>("active_traders_24h") as u64,
        })
    }
}

/// Struct for protocol stats
pub struct ProtocolStats24h {
    pub total_volume_24h: rust_decimal::Decimal,
    pub total_fees_24h: rust_decimal::Decimal,
    pub total_liquidity: rust_decimal::Decimal,
    pub active_traders_24h: u64,
}