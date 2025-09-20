-- Feels Protocol Indexer Database Schema
-- Migration 001: Initial schema

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Markets table - Core market information
CREATE TABLE markets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address TEXT NOT NULL UNIQUE,
    token_0 TEXT NOT NULL,
    token_1 TEXT NOT NULL,
    sqrt_price NUMERIC(78, 0) NOT NULL,
    liquidity NUMERIC(78, 0) NOT NULL,
    current_tick INTEGER NOT NULL,
    tick_spacing SMALLINT NOT NULL,
    fee_bps SMALLINT NOT NULL,
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    phase TEXT NOT NULL CHECK (phase IN ('PriceDiscovery', 'SteadyState')),
    global_lower_tick INTEGER NOT NULL,
    global_upper_tick INTEGER NOT NULL,
    fee_growth_global_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fee_growth_global_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    
    -- Analytics fields
    total_volume_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_volume_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_fees_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    total_fees_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    swap_count BIGINT NOT NULL DEFAULT 0,
    unique_traders BIGINT NOT NULL DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_slot BIGINT NOT NULL
);

-- Positions table - User positions in markets
CREATE TABLE positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address TEXT NOT NULL,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    owner TEXT NOT NULL,
    liquidity NUMERIC(78, 0) NOT NULL,
    tick_lower INTEGER NOT NULL,
    tick_upper INTEGER NOT NULL,
    fee_growth_inside_0_last NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fee_growth_inside_1_last NUMERIC(78, 0) NOT NULL DEFAULT 0,
    tokens_owed_0 BIGINT NOT NULL DEFAULT 0,
    tokens_owed_1 BIGINT NOT NULL DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_slot BIGINT NOT NULL,
    
    UNIQUE(address)
);

-- Swaps table - All swap transactions
CREATE TABLE swaps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    signature TEXT NOT NULL UNIQUE,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    trader TEXT NOT NULL,
    amount_in BIGINT NOT NULL,
    amount_out BIGINT NOT NULL,
    token_in TEXT NOT NULL,
    token_out TEXT NOT NULL,
    sqrt_price_before NUMERIC(78, 0) NOT NULL,
    sqrt_price_after NUMERIC(78, 0) NOT NULL,
    tick_before INTEGER NOT NULL,
    tick_after INTEGER NOT NULL,
    liquidity NUMERIC(78, 0) NOT NULL,
    fee_amount BIGINT NOT NULL,
    
    -- Metadata
    timestamp TIMESTAMPTZ NOT NULL,
    slot BIGINT NOT NULL,
    block_height BIGINT,
    
    -- Analytics
    price_impact_bps SMALLINT,
    effective_price NUMERIC(20, 10)
);

-- Market snapshots for time-series data
CREATE TABLE market_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    slot BIGINT NOT NULL,
    
    -- Price data
    sqrt_price NUMERIC(78, 0) NOT NULL,
    tick INTEGER NOT NULL,
    liquidity NUMERIC(78, 0) NOT NULL,
    
    -- Volume data (since last snapshot)
    volume_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    volume_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fees_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fees_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    swap_count INTEGER NOT NULL DEFAULT 0,
    
    -- TVL
    tvl_token_0 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    tvl_token_1 NUMERIC(78, 0) NOT NULL DEFAULT 0,
    tvl_usd NUMERIC(20, 2),
    
    UNIQUE(market_id, timestamp)
);

-- Buffers table - JIT liquidity buffers
CREATE TABLE buffers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address TEXT NOT NULL UNIQUE,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    
    -- Buffer state
    tau_x NUMERIC(78, 0) NOT NULL DEFAULT 0,
    tau_y NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fee_x NUMERIC(78, 0) NOT NULL DEFAULT 0,
    fee_y NUMERIC(78, 0) NOT NULL DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_slot BIGINT NOT NULL
);

-- Floor liquidity table
CREATE TABLE floors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address TEXT NOT NULL UNIQUE,
    market_id UUID NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    
    -- Floor state
    reserves_x NUMERIC(78, 0) NOT NULL DEFAULT 0,
    reserves_y NUMERIC(78, 0) NOT NULL DEFAULT 0,
    supply NUMERIC(78, 0) NOT NULL DEFAULT 0,
    last_ratchet_slot BIGINT NOT NULL DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated_slot BIGINT NOT NULL
);

-- Indexes for performance
CREATE INDEX idx_markets_tokens ON markets(token_0, token_1);
CREATE INDEX idx_markets_phase ON markets(phase);
CREATE INDEX idx_markets_updated ON markets(updated_at);
CREATE INDEX idx_markets_volume ON markets(total_volume_0 + total_volume_1);

CREATE INDEX idx_positions_owner ON positions(owner);
CREATE INDEX idx_positions_market ON positions(market_id);
CREATE INDEX idx_positions_updated ON positions(updated_at);

CREATE INDEX idx_swaps_trader ON swaps(trader);
CREATE INDEX idx_swaps_market ON swaps(market_id);
CREATE INDEX idx_swaps_timestamp ON swaps(timestamp);
CREATE INDEX idx_swaps_signature ON swaps(signature);

CREATE INDEX idx_snapshots_market_time ON market_snapshots(market_id, timestamp);
CREATE INDEX idx_snapshots_timestamp ON market_snapshots(timestamp);

CREATE INDEX idx_buffers_market ON buffers(market_id);
CREATE INDEX idx_floors_market ON floors(market_id);

-- Full-text search indexes
CREATE INDEX idx_markets_tokens_gin ON markets USING gin((token_0 || ' ' || token_1) gin_trgm_ops);

-- Functions for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for automatic timestamp updates
CREATE TRIGGER update_markets_updated_at BEFORE UPDATE ON markets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_positions_updated_at BEFORE UPDATE ON positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_buffers_updated_at BEFORE UPDATE ON buffers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_floors_updated_at BEFORE UPDATE ON floors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
