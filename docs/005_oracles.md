# Oracle System Specification

This document specifies the oracle system for the Feels Protocol, detailing how market data is aggregated across spot, duration, and volatility dimensions to compute domain value functions and update the system potential.

## Executive Summary

The Feels Protocol implements a hybrid oracle system that combines on-chain geometric TWAP calculations with off-chain volatility metrics. Core price discovery happens entirely on-chain using a tick-based accumulator pattern inspired by Uniswap V3, while more complex calculations like volatility are computed off-chain by keepers.

Unlike traditional DEXs that track only spot prices, our system computes price, liquidity, and volatility metrics for each dimension to feed the domain value functions. The on-chain components ensure trustless price discovery without external dependencies, while off-chain components add sophisticated risk metrics when available.

## Why Three-Dimensional Tracking?

### Domain Value Functions

The protocol's pricing mechanism depends on three domain value functions. The Spot Value Function (S) represents the geometric mean of risk-adjusted pool inventories, capturing the instantaneous exchange rates between token pairs. The Time Value Function (T) represents the geometric mean of risk-adjusted lending and borrowing capacities across different duration buckets, enabling term structure discovery. The Leverage Value Function (L) represents the geometric mean of risk-adjusted long and short position capacities, facilitating directional market exposure.

Each function requires three input metrics:

1. **Price**: Current valuation levels that determine exchange rates and funding costs
2. **Liquidity**: Available capacity for trades, lending, or leverage at each price level
3. **Volatility**: Market risk that affects fee scaling and position limits

### Dimensional Mirroring

The three dimensions mirror each other in structure and behavior. Each dimension performs price discovery to find its own equilibrium values—spot dimensions discover token exchange rates, duration dimensions reveal interest rates across the term structure, and volatility dimensions determine funding rates for directional positions. 

Similarly, liquidity provision operates consistently across dimensions. In the spot dimension, liquidity comes from AMM pool reserves that facilitate token swaps. The duration dimension sources liquidity from lending deposits that enable borrowing across different maturities. The volatility dimension maintains leverage capacity that supports both long and short positions.

Each dimension also tracks its own risk metrics through specialized volatility measures. The spot dimension monitors price volatility (σ_price) to capture exchange rate uncertainty. The duration dimension tracks rate volatility (σ_rate) to measure interest rate risk across the curve. The volatility dimension observes position volatility (σ_leverage) to quantify directional exposure risk. This structural symmetry enables consistent treatment across dimensions while respecting their unique market characteristics.

## Learning from Other Protocols

### Uniswap V3's Tick-Based TWAP

Uniswap V3 pioneered efficient on-chain geometric TWAP calculation through tick accumulation:
- Each pool maintains tickCumulative (sum of log₁.₀₀₀₁(price) over time)
- Updates occur only when price changes, not every block
- Ring buffer stores historical observations for arbitrary lookback
- Geometric TWAP computed as: price = 1.0001^((tickCum[t2] - tickCum[t1])/(t2-t1))

This is an elegant approach because ticks are already logarithmic, making geometric mean calculation a simple arithmetic average of ticks.

### Solana Protocol Comparisons

**Orca's Dynamic Fees**: Tracks 24-hour volatility off-chain and adjusts fees between 0.01%-1% every epoch. Simple but requires trust in fee updates.

**Raydium's Classic TWAP**: Maintains arithmetic price accumulators on-chain. Good for simple averages but expensive for geometric means.

**Meteora's DLMM**: Uses bin-based liquidity as volatility proxy. Sophisticated but complex to implement and verify.

### Key Design Insights

The most successful oracle designs share common patterns. They minimize on-chain computation by using additive accumulators rather than storing individual observations. They leverage logarithmic representations (like ticks) to convert multiplicative operations into additive ones. They piggyback updates on existing transactions rather than requiring separate oracle calls. Finally, they provide bounded storage through ring buffers while still enabling historical queries.

## Oracle Implementation Strategy

### Core Design Principles

The Feels oracle system implements a hybrid approach optimized for both trustlessness and efficiency:

1. **On-Chain Price Discovery**: Geometric TWAPs calculated entirely on-chain using tick-based accumulators
2. **Self-Contained Data**: All metrics derived from protocol's own trading activity  
3. **Efficient Storage**: Ring buffer pattern limits storage while preserving history
4. **Off-Chain Enhancement**: Complex metrics (volatility, correlations) computed by keepers
5. **Graceful Degradation**: System functions with only on-chain data, enhanced by keeper inputs

### On-Chain Tick-Based Oracle Structure

```rust
pub struct MarketOracle {
    // Spot price oracle
    pub spot_oracle: TickOracle,
    
    // Interest rate oracles (per duration bucket)
    pub rate_oracles: [TickOracle; 4], // 1y, 2y, 3y, 5y
    
    // Funding rate oracle
    pub funding_oracle: FundingOracle,
}

pub struct TickOracle {
    // Ring buffer of observations
    pub observations: [Observation; 256],  // Power of 2 for efficient indexing
    pub observation_index: u8,
    pub observation_cardinality: u8,       // Number initialized
    
    // Current tick state
    pub current_tick: i32,                 // log₁.₀₁(price) 
    pub current_liquidity: u128,           // Active liquidity at current tick
    
    // Cumulative values (updated every tick cross)
    pub tick_cumulative: i128,             // Σ(tick_i * time_i)
    pub liquidity_cumulative: u128,        // Σ(liquidity_i * time_i) 
    pub time_cumulative: u64,              // Σ(time_i)
    
    // Last update
    pub last_timestamp: i64,
}

pub struct Observation {
    // Timestamp of this observation
    pub timestamp: i64,
    
    // Cumulative values at this point
    pub tick_cumulative: i128,
    pub liquidity_cumulative: u128,
    pub time_cumulative: u64,
    
    // For volatility calculation (off-chain)
    pub volume_cumulative: u128,
}

pub struct FundingOracle {
    // Similar structure but tracks funding ticks
    pub observations: [FundingObservation; 256],
    pub observation_index: u8,
    
    // Funding can be negative, so we track differently
    pub funding_tick_cumulative: i128,     // Can be negative
    pub open_interest_cumulative: u128,
    pub time_cumulative: u64,
    
    pub last_timestamp: i64,
}
```

### Tick Calculation and Updates

#### Price to Tick Conversion

```rust
// Use base 1.01 for ~1% tick spacing (more efficient than Uniswap's 0.01%)
const TICK_BASE: f64 = 1.01;
const LOG_TICK_BASE: f64 = 0.00995033; // ln(1.01)

// Convert price to tick using bit manipulation (no floating point)
fn price_to_tick(price: u64) -> i32 {
    // For base 1.01: tick ≈ 144.27 * log₂(price)
    // We use a lookup table for the mantissa correction
    let log2_price = 64 - price.leading_zeros();
    let mantissa = (price >> (log2_price - 8)) & 0xFF; // 8-bit precision
    
    let base_tick = (log2_price as i32) * 144;
    let mantissa_adjustment = MANTISSA_LOOKUP[mantissa as usize];
    
    base_tick + mantissa_adjustment
}

// Lookup table for mantissa adjustments (precomputed)
const MANTISSA_LOOKUP: [i32; 256] = [/* ... */];

// Convert tick back to price
fn tick_to_price(tick: i32) -> u64 {
    // Use exponentation by squaring for efficiency
    pow_int(101, tick.abs()) / pow_int(100, tick.abs())
}
```

#### On-Chain Oracle Updates

```rust
impl TickOracle {
    // Called on every trade that crosses a tick
    pub fn update(&mut self, new_tick: i32, liquidity: u128, volume: u128, timestamp: i64) {
        let time_delta = timestamp - self.last_timestamp;
        
        if time_delta > 0 {
            // Update cumulative values
            self.tick_cumulative += (self.current_tick as i128) * (time_delta as i128);
            self.liquidity_cumulative += self.current_liquidity * (time_delta as u128);
            self.time_cumulative += time_delta as u64;
            
            // Write observation if enough time passed
            if time_delta >= MIN_OBSERVATION_INTERVAL {
                self.write_observation(timestamp, volume);
            }
            
            // Update current state
            self.current_tick = new_tick;
            self.current_liquidity = liquidity;
            self.last_timestamp = timestamp;
        }
    }
    
    // Store observation in ring buffer
    fn write_observation(&mut self, timestamp: i64, volume: u128) {
        let index = self.observation_index as usize;
        
        self.observations[index] = Observation {
            timestamp,
            tick_cumulative: self.tick_cumulative,
            liquidity_cumulative: self.liquidity_cumulative,
            time_cumulative: self.time_cumulative,
            volume_cumulative: volume,
        };
        
        self.observation_index = self.observation_index.wrapping_add(1);
        if self.observation_cardinality < 255 {
            self.observation_cardinality += 1;
        }
    }
}
```

### Querying TWAPs On-Chain

```rust
impl TickOracle {
    // Get TWAP over the last `seconds_ago` seconds
    pub fn get_twap(&self, seconds_ago: u32) -> Result<u64> {
        let (old_tick_cum, old_time_cum) = self.observe(seconds_ago)?;
        
        // Calculate average tick
        let tick_delta = self.tick_cumulative - old_tick_cum;
        let time_delta = self.time_cumulative - old_time_cum;
        
        if time_delta == 0 {
            return Ok(tick_to_price(self.current_tick));
        }
        
        let avg_tick = (tick_delta / time_delta as i128) as i32;
        Ok(tick_to_price(avg_tick))
    }
    
    // Get time-weighted average liquidity
    pub fn get_twal(&self, seconds_ago: u32) -> Result<u128> {
        let (old_liq_cum, old_time_cum) = self.observe_liquidity(seconds_ago)?;
        
        let liquidity_delta = self.liquidity_cumulative - old_liq_cum;
        let time_delta = self.time_cumulative - old_time_cum;
        
        if time_delta == 0 {
            return Ok(self.current_liquidity);
        }
        
        Ok(liquidity_delta / time_delta as u128)
    }
    
    // Binary search through ring buffer for historical observation
    fn observe(&self, seconds_ago: u32) -> Result<(i128, u64)> {
        let target_timestamp = self.last_timestamp - seconds_ago as i64;
        
        // Binary search for closest observation
        let obs = self.find_observation(target_timestamp)?;
        
        // Linear interpolation if needed
        if obs.timestamp != target_timestamp {
            let next_obs = self.get_next_observation(&obs)?;
            let weight = (target_timestamp - obs.timestamp) as u128;
            let total = (next_obs.timestamp - obs.timestamp) as u128;
            
            let tick_cum = obs.tick_cumulative + 
                (next_obs.tick_cumulative - obs.tick_cumulative) * weight / total;
            let time_cum = obs.time_cumulative + 
                (next_obs.time_cumulative - obs.time_cumulative) * weight / total;
                
            Ok((tick_cum, time_cum))
        } else {
            Ok((obs.tick_cumulative, obs.time_cumulative))
        }
    }
}
```

### Off-Chain Volatility Calculation

Keepers compute volatility metrics that would be expensive on-chain:

```rust
// Keeper computes realized volatility from on-chain observations
pub fn compute_volatility(oracle: &TickOracle, window_hours: u32) -> f64 {
    let observations = oracle.get_observations(window_hours * 3600);
    
    // Convert ticks to log returns
    let mut returns = Vec::new();
    for i in 1..observations.len() {
        let tick_change = observations[i].tick - observations[i-1].tick;
        let time_delta = observations[i].timestamp - observations[i-1].timestamp;
        
        // Annualized return
        let return_rate = (tick_change as f64 * LOG_TICK_BASE) / 
                         (time_delta as f64 / SECONDS_PER_YEAR);
        returns.push(return_rate);
    }
    
    // Standard deviation of returns
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / returns.len() as f64;
        
    variance.sqrt()
}
```

### Hybrid System Benefits

The tick-based on-chain oracle provides several key advantages:

1. **Trustless Core**: Geometric TWAPs calculated entirely on-chain without external dependencies
2. **Gas Efficient**: Only ~2-3k compute units per trade (tick updates are O(1))
3. **Historical Queries**: Ring buffer enables lookback over configurable time windows
4. **Graceful Fallback**: System functions with only on-chain data if keepers fail

Off-chain keepers enhance the system with:

1. **Volatility Metrics**: Complex calculations that would be expensive on-chain
2. **Cross-Domain Correlations**: Analyze relationships between spot/rate/funding
3. **Advanced Risk Metrics**: Value-at-Risk, conditional volatility, etc.
4. **Proactive Alerts**: Monitor for unusual market conditions

## Implementation Roadmap

### Phase 1: On-Chain Tick Oracle
1. Implement tick conversion functions with lookup tables
2. Deploy ring buffer storage for observations
3. Add tick accumulator updates to trade flow
4. Enable TWAP queries for spot prices

### Phase 2: Multi-Dimensional Extension
1. Extend tick system to interest rates (duration dimension)
2. Add funding rate tracking (leverage dimension)
3. Implement time-weighted liquidity tracking
4. Test cross-dimensional consistency

### Phase 3: Keeper Infrastructure
1. Build off-chain observation indexer
2. Implement volatility calculation service
3. Add correlation analysis between dimensions
4. Create monitoring dashboard

### Phase 4: Production Optimization
1. Optimize tick spacing for gas efficiency
2. Tune ring buffer size based on usage patterns
3. Add caching for frequently queried windows
4. Implement emergency fallback parameters

## Security Considerations

### On-Chain Oracle Security

The tick-based oracle design provides strong security guarantees:

1. **Manipulation Resistance**: Attacks require sustained volume over time windows
   - 30-minute TWAP requires manipulating price for entire duration
   - Cost scales linearly with manipulation time
   - Ring buffer preserves evidence of manipulation attempts

2. **No External Dependencies**: All data comes from protocol's own trades
   - No trust in external price feeds
   - No oracle extractable value (OEV)
   - Atomic composability with other on-chain protocols

3. **Bounded Computation**: Fixed gas costs regardless of market conditions
   - O(1) updates per trade
   - O(log n) historical queries via binary search
   - No unbounded loops or storage growth

### Off-Chain Enhancement Security

Keeper-provided volatility metrics include safety mechanisms:

1. **Purely Additive**: Volatility only increases fees, never decreases them
2. **Capped Impact**: Maximum volatility contribution to fees is bounded
3. **Fallback Values**: Conservative defaults when keeper data unavailable
4. **No Critical Dependency**: System fully functional without keeper input

## Conclusion

The Feels Protocol's hybrid oracle system combines the best of on-chain and off-chain computation. By implementing tick-based accumulators for core price discovery, we achieve trustless geometric TWAPs with minimal gas overhead—adding only 2-3k compute units per trade. This approach, inspired by Uniswap V3 but adapted for Solana's architecture, ensures the protocol can operate independently without external dependencies.

The three-dimensional extension—tracking prices, rates, and funding across spot, duration, and leverage dimensions—provides comprehensive market state for domain value calculations. Each dimension uses the same tick-based pattern, ensuring consistency and code reuse.

Off-chain keepers enhance the system with sophisticated volatility metrics and cross-domain analysis, but these remain strictly optional. The protocol degrades gracefully when keeper data is unavailable, using conservative defaults that protect users while maintaining full functionality.

This design achieves our goals of trustlessness, efficiency, and extensibility while learning from the successes of existing DeFi protocols.