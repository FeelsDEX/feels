-- Add performance indexes for Feels Protocol Indexer
-- Migration: 0002_add_indexes
-- Date: 2025-11-25

-- Markets indexes
CREATE INDEX IF NOT EXISTS idx_markets_token_0 ON markets(token_0);
CREATE INDEX IF NOT EXISTS idx_markets_token_1 ON markets(token_1);
CREATE INDEX IF NOT EXISTS idx_markets_last_updated ON markets(last_updated_slot DESC);
CREATE INDEX IF NOT EXISTS idx_markets_phase ON markets(phase);
CREATE INDEX IF NOT EXISTS idx_markets_created_at ON markets(created_at DESC);

-- Swaps indexes (critical for performance)
CREATE INDEX IF NOT EXISTS idx_swaps_market_timestamp ON swaps(market_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_trader ON swaps(trader);
CREATE INDEX IF NOT EXISTS idx_swaps_timestamp ON swaps(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_signature ON swaps(signature);
CREATE INDEX IF NOT EXISTS idx_swaps_market_slot ON swaps(market_id, slot);

-- Positions indexes
CREATE INDEX IF NOT EXISTS idx_positions_owner ON positions(owner);
CREATE INDEX IF NOT EXISTS idx_positions_market ON positions(market_id);
CREATE INDEX IF NOT EXISTS idx_positions_updated ON positions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_positions_liquidity ON positions(liquidity) WHERE liquidity > 0;

-- Market snapshots indexes
CREATE INDEX IF NOT EXISTS idx_snapshots_market_timestamp ON market_snapshots(market_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON market_snapshots(timestamp DESC);

-- Composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_swaps_market_trader ON swaps(market_id, trader, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_positions_market_owner ON positions(market_id, owner);

-- Analyze tables after creating indexes
ANALYZE markets;
ANALYZE swaps;
ANALYZE positions;
ANALYZE market_snapshots;

