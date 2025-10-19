---
title: "Just-In-Time Protocol Liquidity"
description: "Automated market making for newly launched tokens with solvency protection"
category: "Specifications"
order: 202
draft: false
searchable: true
---

# Just-In-Time Protocol Liquidity

This document presents the just-in-time (JIT) liquidity system that provides automated market making for newly launched tokens while protecting the protocol from toxic flow and maintaining strict solvency constraints.

**MVP Implementation**: The protocol launches with JIT v0.5, featuring virtual concentrated liquidity that provides 5-10x revenue improvement through enhanced caps and concentration effects, without the complexity of full position management. This pragmatic approach delivers immediate value while maintaining safety through multiple defensive layers.

## Context

New tokens often launch with thin books. We want to ensure the Feels protocol provides reliable execution and continuous pricing. We must do this without handing toxic flow a free option or risking solvency.

## Decision

Provide bounded, just-in-time (JIT) protocol liquidity inside the swap around a biased, clamped geometric time-weighted average price (GTWAP) anchor, with micro-spread, strict budgets, and place-execute-remove in one instruction. Side selection is contrarian to the taker's intent, never trend-following on lagging windows. Sizing is funded only from τ (fee buffer); asks never sit below the floor; sells come only from inventory. A tiny toxicity EMA throttles size after pick-offs.

## MVP Mode: JIT v0.5 (Virtual Concentrated Liquidity)

To ensure reliable execution at the market price and smooth bootstrapping without external market makers, the MVP ships JIT v0.5 with virtual concentrated liquidity:

### Key Features

- **Virtual Concentration**: Simulates concentrated liquidity without creating positions by scaling liquidity based on distance from current price (up to 10x boost at current tick)
- **Enhanced Caps**: Increases base JIT budget from 1% to 3% of buffer (configurable per market type)
- **Multiple Safety Layers**: Graduated drain protection, slot-based concentration shifts, asymmetric directional caps, tick distance penalties, and circuit breaker
- **Simple Implementation**: ~500 lines of changes, maintaining single-tx place-execute-remove pattern
- **Floor Guard**: JIT ask never below `pool::Floor.get_safe_ask_tick()`

### Virtual Concentration Mechanism

Instead of creating actual positions, v0.5 scales JIT liquidity based on execution distance from current price:

```rust
// Concentration multiplier based on tick distance
let tick_distance = (execution_tick - current_tick).abs();
let concentration_multiplier = match tick_distance {
    0..=10 => 10,      // 10x boost at current price
    11..=50 => 5,      // 5x boost nearby
    51..=100 => 2,     // 2x boost further out
    _ => 1,            // 1x boost far away
};
```

### Enhanced Budget Caps

```rust
// Increased from 1% to 3% default
pub const DEFAULT_JIT_PER_SWAP_BPS: u16 = 300;    // 3% of buffer per swap
pub const DEFAULT_JIT_PER_SLOT_BPS: u16 = 500;    // 5% of buffer per slot
```

## Non-Goals

* No dependency on off-chain keepers, external venues, or 24h volume.
* No persistent passive ranges beyond the narrow JIT bands.
* No use of protected floor reserves $R_{\ast}$.

## Design

### System Overview

The JIT liquidity system is the protocol's facility for executable liquidity. After the initial price discovery phase, a combinatino of JIT liquidity available at the market price + floor liquidity, consitutes the system's steady-state market making strategy.

The JIT liquidity system provides automated, risk-aware market making for newly launched tokens on Feels. At its core, the system acts as a contrarian liquidity provider that places narrow bands of liquidity opposite to incoming trades. While the system is primarily funded by ongoing protocol fee revenue flowing into the pool's buffer account (τ), it is initially capitalized by a small portion of the seed capital from the token's launch phase. This ensures the JIT system is active from the moment the pool enters its steady-state. The system operates entirely within a single swap instruction, placing liquidity just-in-time for the incoming trade and removing any unfilled liquidity immediately after execution.

The pricing anchor combines a geometric time-weighted average price (GTWAP) with a floor price bias, creating a reference point that resists short-term manipulation while respecting the protocol's solvency constraints. Around this anchor, the system places micro-spreads that widen dynamically based on detected toxicity (adverse price movements after fills). The adaptive spread mechanism ensures the protocol is compensated proportionally to the risk it takes, with tighter spreads in calm markets and wider spreads during volatile or adversarial conditions.

Several defensive layers work together to prevent exploitation: directional toxicity tracking with EMA smoothing detects and throttles adverse flow; per-slot budgets and fill limits prevent resource exhaustion; tick-crossing budgets ensure graceful degradation under high volatility; GTWAP slope guards detect and reject manipulation attempts; and inventory cooldowns prevent round-trip attacks. Each component is designed to fail gracefully. This reduces participation rather than stopping entirely, ensuring the system remains useful, even under attack, while bounding worst-case losses to a reasonable and predictable level.

### Safety Mitigations (JIT v0.5)

#### 1. Graduated Drain Protection

Reduces allowance as consumption increases within a rolling window:

```rust
// Reset rolling window every 150 slots (~1 minute)
if current_slot > budget.rolling_window_start + 150 {
    budget.rolling_consumption = 0;
    budget.rolling_window_start = current_slot;
}

// Calculate consumption ratio
let consumption_ratio = budget.rolling_consumption
    .checked_mul(10_000)?
    .checked_div(budget.per_slot_cap_q)
    .unwrap_or(0);

// Graduated throttling
let throttle_factor = match consumption_ratio {
    0..=5000 => 100,      // < 50% used: full allowance
    5001..=7500 => 50,    // 50-75% used: half allowance
    7501..=9000 => 20,    // 75-90% used: 20% allowance
    _ => 10,              // > 90% used: 10% allowance
};
```

#### 2. Slot-Based Concentration Shifts

Prevents attackers from camping optimal ticks by shifting concentration zone every 100 slots:

```rust
// Shift concentration zone every 100 slots (~40 seconds)
let shift_interval = 100u64;
let shift_cycles = current_slot.checked_div(shift_interval).unwrap_or(0);
let shift_amount = ((shift_cycles % 20) as i32).saturating_sub(10);

// Calculate adjusted distance with shift
let adjusted_distance = target_tick
    .saturating_sub(current_tick)
    .saturating_sub(shift_amount)
    .abs() as u32;
```

#### 3. Asymmetric Directional Caps

Reduces caps for crowded trade directions based on recent volume:

```rust
// Calculate recent buy pressure (stored in market state)
let buy_pressure = market.rolling_buy_volume
    .checked_mul(100)
    .and_then(|v| v.checked_div(market.rolling_total_volume))
    .unwrap_or(50) as u16;

// Reduce cap for crowded direction
match (is_buy, buy_pressure) {
    (true, bp) if bp > 70 => base_cap_bps / 2,   // Heavy buy pressure
    (false, bp) if bp < 30 => base_cap_bps / 2,  // Heavy sell pressure
    _ => base_cap_bps,                           // Normal conditions
}
```

#### 4. Tick Distance Impact Penalty

Discourages trades that move price significantly:

```rust
// Graduated penalty for large price movements
let penalty_factor = match tick_movement {
    0..=10 => 100,    // No penalty for small moves
    11..=50 => 70,    // 30% penalty
    51..=100 => 40,   // 60% penalty
    101..=200 => 20,  // 80% penalty
    _ => 10,          // 90% penalty for huge moves
};
```

#### 5. Circuit Breaker

Emergency halt mechanism when buffer health drops below threshold or extreme price movement detected:

```rust
// Check buffer health
let buffer_health_bps = buffer.tau_spot
    .saturating_mul(10_000)
    .checked_div(buffer.initial_tau_spot)
    .unwrap_or(0) as u16;

if buffer_health_bps < params.circuit_breaker_threshold {
    return true;
}

// Check for extreme price movement (>10% in 1 hour)
let price_movement = market.current_tick
    .saturating_sub(market.tick_snapshot_1hr)
    .abs();
    
price_movement > 1000  // ~10% movement
```

### Integration with Unified Architecture

The JIT system integrates cleanly across layers. Terminology: protocol = global systems; pool = per‑market systems.

**Pool Oracle (GTWAP)**: JIT reads the price anchor from `pool::Oracle` (GTWAP). This avoids coupling pool logic to protocol‑level reserves.

**Pool Floor**: All JIT asks respect `pool::Floor`’s safe ask tick. Floor calculation is pool‑local and monotonic.

**FlowSignals**: JIT feeds directional toxicity into the shared `FlowSignals` component; it also consumes combined signals for spread/size throttling.

**SafetyController**: Global `protocol::SafetyController` gates participation, rate limits, and degraded modes.

### JIT v0.5 Parameters (MVP Configuration)

#### Core Parameters
- **base_cap_bps**: 300 (3% of buffer) - configurable per market type
- **concentration_width**: 10 ticks for blue-chip, wider for volatile markets
- **max_multiplier**: 10x for concentrated liquidity effect
- **drain_protection_threshold**: 7000 bps (70% consumption triggers throttling)
- **circuit_breaker_threshold**: 3000 bps (30% buffer health)

#### Safety Parameters
- **rolling_window_slots**: 150 (~1 minute)
- **concentration_shift_interval**: 100 slots (~40 seconds)
- **max_tick_movement_penalty**: 200 ticks (90% penalty beyond this)
- **directional_cap_threshold**: 70% buy/30% sell pressure

#### Recommended Parameters by Market Type

| Market Type | Base Cap | Concentration Width | Max Multiplier | Drain Threshold | Circuit Breaker |
|-------------|----------|-------------------|----------------|-----------------|-----------------| 
| Stablecoin | 500 bps | 5 ticks | 20x | 7500 bps | 2000 bps |
| Blue-chip | 300 bps | 10 ticks | 10x | 7000 bps | 3000 bps |
| Volatile | 100 bps | 20 ticks | 5x | 6000 bps | 4000 bps |
| New Token | 50 bps | 50 ticks | 3x | 5000 bps | 5000 bps |

### Anchor & Placement

* **Anchor**: `R = max(pool_oracle.get_tick()?, pool_floor.get_safe_ask_tick())`. If GTWAP is stale, use `current_tick` for the anchor.
* **Clamp** to current price: `R_c = clamp(R, tick_cur − DEV_CLAMP, tick_cur + DEV_CLAMP)`
* **Micro-spread ranges** (ticks):

  * **Bid**: `[R_c − final_spread − RANGE, R_c − final_spread]`
  * **Ask**: `[R_c + final_spread + edge_offset, R_c + final_spread + edge_offset + RANGE]`
  * Where `final_spread = BASE_SPREAD_TICKS + spread_adjustment` (widens with toxicity)
  * `edge_offset = slot_id & 1` adds deterministic jitter to prevent edge pinning
* **Mode**: **Contrarian** - infer taker intent from params; place **only** the opposite side. Direction inference:
  - Buy direction (+1) if: `(amount_specified_is_input && sqrt_price_limit > price_cur)` OR `(!amount_specified_is_input && sqrt_price_limit > price_cur)`
  - Require `abs(sqrt_price_limit - price_cur) >= L_MIN_TICKS` for contrarian mode
  - If ambiguous, limit too close, or router marked "unknown": use symmetric mode with `BASE_SPREAD_TICKS_SYM >= BASE_SPREAD_TICKS` and `size <= MAX_PER_SWAP_Q / 2`

### Budgets & Sizing (all integers; on-chain verifiable)

* **Per-swap cap**: `size ≤ MAX_PER_SWAP_Q`
* **Per-slot cap**: `slot_budget_used + consumed_jit_quote ≤ MAX_PER_SLOT_Q` (charges only filled amount)
* **Per-slot hit limits**: If `fills_this_slot >= H` or `cum_jit_ticks >= CUM_TICKS`, set `α = TOX_MIN_Q16` for rest of slot
* **Tick-crossing budget**: Only charge if JIT was filled:
  * If JIT hit: `ticks_crossed_this_slot += actual_ticks_crossed * consumed_jit_quote / amount_in_quote`
  * If `ticks_crossed_this_slot + expected_crosses > MAX_TICKS_PER_SLOT`, reduce JIT size proportionally
* **Funding**: `base = min(size_cap, BASE_BPS_OF_TAU * τ / 10_000)` (τ (pool buffer) only)
* **Unified flow signals integration**: 
  * Local toxicity: `α_local = max(TOX_MIN_Q16, 65535 − toxicity_q16)`
  * Global adjustment: `α = (α_local * flow_signals.get_alpha()) >> 16`
  * Final size: `size = (base * α) >> 16`
* **Dynamic spread adjustment**: 
  * Base spread from unified flow signals: `unified_spread = flow_signals.get_spread_adjustment()`
  * Final spread: `final_spread = BASE_SPREAD_TICKS + max(local_spread_adj, unified_spread)`
* **Minimum taker size**: Skip JIT if `amount_in_quote < Q_MIN_FOR_JIT` to prevent dust griefing
* **Proportional size cap**: `size ≤ min(MAX_PER_SWAP_Q, β * min(amount_declared, simulated_fill_at_limit))` where β = 0.25
* **Non-adverse contribution cap**: Per-slot non-adverse toxicity contribution capped at 1× TOX_BASE_Q16_IF_HIT

### Inventory Management (MVP)

In the MVP implementation, JIT uses a simple floor-diversion inventory management approach:

* **Floor Diversion**: All JIT bid fills are diverted to the buffer's fee accounting (instead of burning), where they become available for floor liquidity placement via the POMM system
* **Capital Efficiency**: This approach ensures all JIT quote tokens contribute to market stability through floor liquidity rather than being burned
* **Simplified Accounting**: JIT consumed quote is added to `buffer.fees_token_0` or `buffer.fees_token_1` based on swap direction, making it available for POMM conversion
* **No Complex Inventory**: MVP avoids complex inventory tracking, maturity delays, and rebalancing - all JIT fills flow directly to floor support

#### Future Enhancements (Post-MVP)

* **Initial Inventory**: Protocol receives initial token allocation during the bonding curve phase (details in separate protocol asset allocation document)
* **Dynamic Inventory**: Buys through JIT add to inventory; sells reduce it
* **Path Dependency**: In contrarian mode, the JIT can only place asks if it has sufficient inventory from either:
  - Initial protocol allocation
  - Previous buy-side fills
* **Inventory Deployment Delay**: After a JIT bid is filled, a cooldown period prevents immediate ask placement using the newly acquired inventory
* **Floor-neutral policy**: When selling Δs tokens receiving Δq_actual quote, commit = min(Δq_actual, ceil(P_floor * Δs)) from τ to R_*. Assert P_floor' >= P_floor post-state.
* **Matured inventory**: Only asks from inventory that has aged >= INVENTORY_MATURITY_SLOTS can be used (prevents rapid force-feed-dump cycles)
* **Rebalancing**: Periodic rebalancing mechanism adjusts inventory levels (specified in separate document)

The future system will provide two-sided liquidity from launch, with the bonding curve allocation bootstrapping initial sell-side capacity while preventing inventory manipulation attacks.

### Entry Guards

All must pass before placing JIT:

1. **Global safety check**: `safety_controller.can_execute_jit()?` (respects global pause and degraded states)
2. **Oracle health**: `reserve_oracle_aggregator.health_status == OracleHealth::Healthy`
3. **Oracle freshness**: `reserve_oracle_aggregator.is_fresh(slot)?` (unified freshness check)
4. **Deviation**: `|tick_cur − reserve_oracle_aggregator.gtwap.get_tick()?| ≤ MAX_DEV_TICKS` (skip if breakout)
5. **Cooldown**: `slot ≥ cooldown_until_slot`
6. **Ask cooldown**: If placing an ask, `slot > ask_cooldown_until_slot` (prevents immediate redeployment of inventory from bid fills)
7. **GTWAP slope guard**: `reserve_oracle_aggregator.check_manipulation(MAX_TWAP_SLOPE_TPS)?`
8. **GTWAP divergence duration**: Require deviation check for `D_MIN_SLOTS` consecutive slots
9. **Minimum taker size**: Skip if `amount_in_quote < Q_MIN_FOR_JIT`. After first fill in slot, increase to `Q_MIN_FOR_JIT * 2`
10. **Tick budget check**: If `ticks_crossed_this_slot + expected_crosses > MAX_TICKS_PER_SLOT`, reduce JIT participation proportionally
11. **Per-slot limits**: If `fills_this_slot >= H` or `cum_jit_ticks >= CUM_TICKS`, throttle to minimum
12. **Rate limiting**: `safety_controller.rate_limiter.check_jit_operation(slot)?`
13. **Floor safety**: Never place asks below `floor_manager.current_floor`

### Toxicity Tracking with Unified System

After swap execution, the JIT system updates both local and unified flow signals:

```rust
// Inside the swap, after execution:
let dt = tick_after_swap - tick_before_swap;
let adverse = (jit_ask_hit && dt > 0) || (jit_bid_hit && dt < 0);

let obs_q16 = if adverse {
    // Map ticks to Q16 with bounded ramp
    min(65535, (abs(dt) as u32 * TOX_TICK_Q16).min(65535))
} else if (jit_ask_hit || jit_bid_hit) {
    TOX_BASE_Q16_IF_HIT  // Small floor (~5%) even for non-adverse hits
} else {
    0
};

// Update local toxicity with asymmetric EMA
let shift = if obs_q16 > toxicity_q16 { TOX_SHIFT_UP } else { TOX_SHIFT_DOWN };
toxicity_q16 += ((obs_q16 as i32 - toxicity_q16 as i32) >> shift) as u16;

// Feed into unified flow signals system
flow_signals.update_from_jit(
    obs_q16,
    dt,
    jit_ask_hit || jit_bid_hit,
    slot
)?;

// Get combined signals for sizing and spread
let size_alpha = flow_signals.get_combined_alpha(toxicity_q16)?;
let spread_adjustment = flow_signals.get_spread_adjustment()?;
```

This tracks only directional adversity (price moves against the filled side) while maintaining a small per-hit floor to prevent "slow bleed" attacks through many small non-adverse fills.

Size uses `size_alpha` to reduce on any hit (adverse or not), while spread widens based on `spread_toxicity` (full weight for adverse, half for non-adverse). This dual mechanism prevents both aggressive attacks and slow-bleed exploitation.

### One-Tx Lifecycle with Unified Updates

1. **Pre-execution checks**:
   - Verify global safety: `safety_controller.can_execute_jit()?`
   - Check pool oracle health: `pool_oracle.health_status == Healthy`
   - Validate all entry guards

2. **Compute placement**:
   - Get anchor from pool oracle: `R = max(pool_oracle.get_tick()?, pool_floor.get_safe_ask_tick())`
   - Apply unified flow signals to sizing: `size = base * flow_signals.get_combined_alpha(local_toxicity)?`
   - Adjust spread using unified signals

3. **Execute atomically**:
   - **Place** JIT bands (pre-allocated PDAs)
   - **Execute** swap
   - **Remove** unfilled liquidity

4. **Post-execution updates**:
   - Update local state (budgets, fills, toxicity)
   - Feed signals to unified systems:
     ```rust
     // Update unified flow signals
     flow_signals.update_from_jit(obs_q16, dt, hit, slot)?;
     
     // Update pool oracle if significant move
     if abs(dt) > ORACLE_UPDATE_THRESHOLD {
         pool_oracle.update(slot, tick_after, timestamp)?;
     }
     
     // Update pool floor if ask was filled
     if jit_ask_hit {
         pool_floor.update_after_swap(jitosol_change, slot)?;
     }
     ```
   - Apply cooldowns and reset slot counters as needed

5. **Unified post-swap flow**:
   - All updates flow through the pool's unified post-swap handler
   - Ensures consistent state updates across all subsystems

### State (per pool)

```rust
pub struct JitState {
    // Slot tracking
    pub slot_id: u64,
    pub slot_budget_used_q: u128,
    pub ticks_crossed_this_slot: u32,
    pub fills_this_slot: u8,           // Tracks JIT fills per slot
    pub cum_jit_ticks: u16,           // Cumulative tick movement from JIT fills
    
    // Cooldowns
    pub cooldown_until_slot: u64,
    pub ask_cooldown_until_slot: u64,
    
    // Local toxicity (feeds into unified system)
    pub toxicity_q16: u16,
    
    // Deviation tracking
    pub deviation_ok_slots: u8,       // Consecutive slots where |cur-gtwap| <= MAX_DEV
    
    // Performance tracking
    pub cu_skips_this_slot: u8,       // Track CU-based skips
    pub inventory_by_slot: [u64; 8],  // Ring buffer tracking inventory age
    
    // v0.5 additions for virtual concentration
    pub rolling_consumption: u128,      // Track recent usage
    pub rolling_window_start: u64,      // Window start slot
    pub last_heavy_usage_slot: u64,     // For cooldowns
    pub total_consumed_this_epoch: u128, // Epoch tracking
    
    // Position PDAs
    pub bid_pos_pda: Pubkey,
    pub ask_pos_pda: Pubkey,
}

// Note: GTWAP data accessed via pool::Oracle
// Note: Global flow signals accessed via FlowSignals
// Note: Floor data accessed via pool::Floor
```

### Constants (hierarchical parameter management)

```rust
// Core JIT parameters (protocol-level, rarely changed)
pub struct JitCoreParams {
    pub max_per_swap_q: u128,
    pub max_per_slot_q: u128,
    pub max_ticks_per_slot: u32,
    pub base_bps_of_tau: u16,
}

// Pool-tier parameters (adjustable per pool maturity)
pub struct JitMarketParams {
    pub base_spread_ticks: u8,      // Default: 1
    pub max_spread_ticks: u8,       // Default: 4
    pub range_ticks: u8,            // Default: 1
    pub base_spread_ticks_sym: u8,  // Default: 2
    pub l_min_ticks: u8,            // Default: 5
    pub dev_clamp_ticks: i32,
    pub cooldown_slots: u64,
    pub ask_cooldown_slots: u64,    // Default: 10-20
    pub inventory_maturity_slots: u64, // Default: 5
}

// Toxicity parameters (frequently tuned)
pub struct JitToxicityParams {
    pub tox_tick_q16: u16,          // Default: 6553
    pub tox_shift_up: u8,           // Default: 2
    pub tox_shift_down: u8,         // Default: 4
    pub tox_min_q16: u16,           // Default: 3277 (~5%)
    pub tox_base_q16_if_hit: u16,   // Default: 3277
    pub h_max_fills: u8,            // Default: 3
    pub cum_ticks_limit: u16,       // Default: 6
}

// Oracle parameters (aligned with pool::Oracle and protocol::Oracle)
pub struct JitOracleParams {
    pub max_twap_slope_tps: u16,    // Default: 1
    pub d_min_slots: u8,            // Default: 3
    pub k_cu_free_swaps: u8,        // Default: 5
}
```

### Invariants

* **Solvency**: JIT spends **τ (pool buffer)** only; bid fills are burned (never resold); asks use only initial protocol allocation
* **Floor preservation**: Enforced via pool::Floor: `ask_tick >= pool_floor.current_floor`
* **Atomicity**: place-execute-remove in a single instruction; no lingering JIT
* **Budgets**: per-swap, per-slot, and fill-count caps enforced; only consumed liquidity charged
* **Graceful degradation**: JIT participation reduces proportionally based on unified signals
* **Spread bounds**: `final_spread ∈ [BASE_SPREAD_TICKS, MAX_SPREAD_TICKS]` always
* **Symmetric safety**: In symmetric mode: `size <= MAX_PER_SWAP_Q / 4`, `spread >= BASE_SPREAD_TICKS_SYM`
* **Unified coordination**:
  - Pool price data always from pool::Oracle
  - Toxicity signals shared via FlowSignals
  - Floor calculations from pool::Floor
  - Safety states from SafetyController
* **Tick charging**: `ticks_crossed_this_slot` increments only when `consumed_jit_quote > 0`
* **Non-adverse cap**: Per-slot non-adverse toxicity capped at 1× `TOX_BASE_Q16_IF_HIT`
* **Bounded loss**: worst-case per-tx pick-off bounded by constants:
  ```
  loss_bps ≤ Q_cap * (DEV_CLAMP_TICKS + MAX_SPREAD_TICKS + RANGE_TICKS) − fees(Q_cap)
  ```

## Security Considerations

* **Oracle gaming**: Pool oracle provides manipulation resistance; protocol oracle is independent and conservatively valued
* **Sandwich**: single-tx place-execute-remove
* **DoS via CU**: tick-crossing budget prevents gaming; SafetyController rate limiting provides additional protection
* **Floor breach**: pool::Floor enforces floor constraints across all operations
* **Inventory manipulation**: ask cooldown prevents immediate redeployment
* **Toxicity response**: FlowSignals system provides coordinated response across all subsystems
* **Global safety**: SafetyController can throttle or pause JIT during extreme conditions
* **Cross-system attacks**: Unified architecture prevents exploitation of inconsistencies between subsystems

### Rounding Policy

All calculations use consistent rounding to prevent microscopic leakage:
* **Ceiling** for credits to R_* (conservative for protocol)
* **Floor** for credits to τ
* **Worst-case prices** when computing ask proceeds (lowest tick in band)
* **Integer-only math** throughout with explicit overflow checks

### Parameter Tiering

For newly launched pools:
* **Launch tier** (first 7 days): `MAX_TWAP_SLOPE_TPS = 2`, `L_MIN_TICKS = 3`, `D_MIN_SLOTS = 2`
* **Mature tier** (after 7 days or τ (pool buffer) > threshold): Tighten to standard parameters

This ensures JIT remains active during volatile launch periods while preventing long-term exploitation.

## Deployment Stages

The JIT system deployment follows a progressive rollout:

### Stage 1: Foundation (Week 1) - JIT v0.5 Conservative
- Deploy with conservative parameters (100 bps, 5x multiplier)
- Virtual concentration active but limited
- All safety mechanisms enabled
- Monitor performance and attack attempts

### Stage 2: Activation (Weeks 2-3) - JIT v0.5 Balanced
- Increase to balanced parameters (200 bps, 10x multiplier) if stable
- Fine-tune concentration widths based on market behavior
- Adjust safety thresholds based on observed patterns

### Stage 3: Optimization (Week 4+) - JIT v0.5 Optimized
- Move to aggressive parameters (300 bps, 10x multiplier) for mature markets
- Maintain conservative settings for new/volatile tokens
- Continuous parameter optimization based on data

### Future: JIT v1.0 (Post-MVP)
- Full position management with actual concentrated liquidity
- Advanced inventory tracking and rebalancing
- Multi-tier concentration profiles
- Historical pattern learning

The staged approach allows rapid value capture while maintaining safety.

## Expected Outcomes

### Revenue Impact
- **Conservative** (1.5% cap, 5x concentration): 3-4x revenue vs basic JIT
- **Balanced** (2% cap, 10x concentration): 5-7x revenue vs basic JIT
- **Aggressive** (3% cap, 10x concentration): 7-10x revenue vs basic JIT

### Volume Impact
- Improved execution attracts 2-5x more volume
- Becomes preferred DEX for large trades  
- Aggregators consistently route through protocol

### Risk Profile
- **Drain Risk**: Mitigated by graduated protection and circuit breaker
- **Manipulation Risk**: Mitigated by slot shifts and impact penalties
- **Technical Risk**: Low due to simple implementation (~500 lines)

## Success Metrics

* **Continuity**: fail-to-execute rate at low liquidity < 1%
* **Revenue efficiency**: Revenue per million volume > 5x baseline
* **Bounded exposure**: Realized JIT PnL variance per swap within target
* **Pick-off control**: FlowSignals remain low; appropriate throttling
* **Organic depth**: Stable or improving outside micro-spread bands
* **System coordination**: 
  - Oracle health maintained above 95%
  - Floor never breached by JIT operations
  - Safety interventions rare and appropriate
  - Toxicity signals consistent across subsystems
* **Performance**: CU usage within bounds; no degradation of swap execution

## Monitoring Requirements

Track these metrics for health monitoring:

1. **Buffer Health**: `tau_spot / initial_tau_spot`
2. **Consumption Rate**: `rolling_consumption / per_slot_cap`  
3. **Concentration Hit Rate**: How often trades hit peak concentration zones
4. **Circuit Breaker Activations**: Frequency and causes
5. **Revenue Per Million Volume**: Key efficiency metric
6. **Directional Imbalance**: Buy vs sell pressure over time
7. **Tick Movement Impact**: Average penalty factor applied

## Future Roadmap: JIT v1.0 and Beyond

### JIT v0.6: Volatility-Adaptive Concentration
- Widen concentration in volatile markets
- Tighten in stable conditions
- Dynamic parameter adjustment based on recent price action

### JIT v0.7: Multi-Tier Concentration  
- Different concentration profiles by trade size
- Optimize for both retail and whales
- Size-aware multiplier curves

### JIT v0.8: Historical Pattern Learning
- Track which ticks see most volume
- Bias concentration toward historical hotspots
- Time-of-day and day-of-week patterns

### JIT v1.0: Full Position Management
- Actual concentrated liquidity positions (not virtual)
- Advanced inventory tracking with maturity models
- Rebalancing mechanisms for two-sided liquidity
- Integration with external price feeds
- Cross-market arbitrage awareness

The progression from v0.5 to v1.0 allows the protocol to capture immediate value while building toward a sophisticated market making system that can compete with centralized exchanges on execution quality.
