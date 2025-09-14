# Just-In-Time Protocol Liquidity

This document presents the just-in-time (JIT) liquidity system that provides automated market making for newly launched tokens while protecting the protocol from toxic flow and maintaining strict solvency constraints.

## Context

New tokens often launch with thin books. We want to ensure the Feels protocol provides reliable execution and continuous pricing. We must do this without handing toxic flow a free option or risking solvency.

## Decision

Provide bounded, just-in-time (JIT) protocol liquidity inside the swap around a biased, clamped geometric time-weighted average price (GTWAP) anchor, with micro-spread, strict budgets, and place-execute-remove in one instruction. Side selection is contrarian to the taker's intent, never trend-following on lagging windows. Sizing is funded only from τ (fee buffer); asks never sit below the floor; sells come only from inventory. A tiny toxicity EMA throttles size after pick-offs.

## Non-Goals

* No dependency on off-chain keepers, external venues, or 24h volume.
* No persistent passive ranges beyond the narrow JIT bands.
* No use of protected floor reserves $R_{\ast}$.

## Design

### System Overview

The JIT liquidity system is the protocol's facility for executable liquidity. After the initial price discovery phase, a combinatino of JIT liquidity available at the market price + floor liquidity, consitutes the system's steady-state market making strategy.

The JIT liquidity system provides automated, risk-aware market making for newly launched tokens on Feels. At its core, the system acts as a contrarian liquidity provider that places narrow bands of liquidity opposite to incoming trades. While the system is primarily funded by ongoing protocol fee revenue flowing into the pool's buffer account (τ), it is initially capitalized by a small portion of the seed capital from the token's launch phase. This ensures the JIT system is active from the moment the market enters its steady-state. The system operates entirely within a single swap instruction, placing liquidity just-in-time for the incoming trade and removing any unfilled liquidity immediately after execution.

The pricing anchor combines a geometric time-weighted average price (GTWAP) with a floor price bias, creating a reference point that resists short-term manipulation while respecting the protocol's solvency constraints. Around this anchor, the system places micro-spreads that widen dynamically based on detected toxicity (adverse price movements after fills). The adaptive spread mechanism ensures the protocol is compensated proportionally to the risk it takes, with tighter spreads in calm markets and wider spreads during volatile or adversarial conditions.

Several defensive layers work together to prevent exploitation: directional toxicity tracking with EMA smoothing detects and throttles adverse flow; per-slot budgets and fill limits prevent resource exhaustion; tick-crossing budgets ensure graceful degradation under high volatility; GTWAP slope guards detect and reject manipulation attempts; and inventory cooldowns prevent round-trip attacks. Each component is designed to fail gracefully. This reduces participation rather than stopping entirely, ensuring the system remains useful, even under attack, while bounding worst-case losses to a reasonable and predictable level.

### Integration with Unified Architecture

The JIT system integrates cleanly across layers. Terminology: protocol = global systems; pool = per‑market systems.

**Pool Oracle (GTWAP)**: JIT reads the price anchor from `pool::Oracle` (GTWAP). This avoids coupling pool logic to protocol‑level reserves.

**Pool Floor**: All JIT asks respect `pool::Floor`’s safe ask tick. Floor calculation is pool‑local and monotonic.

**FlowSignals**: JIT feeds directional toxicity into the shared `FlowSignals` component; it also consumes combined signals for spread/size throttling.

**SafetyController**: Global `protocol::SafetyController` gates participation, rate limits, and degraded modes.

### Anchor & Placement

* **Anchor**: `R = max(pool_oracle.get_tick()?, pool_floor.get_safe_ask_tick())`
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

### Inventory Management

The JIT system requires protocol-owned inventory to place ask orders:

* **Initial Inventory**: Protocol receives initial token allocation during the bonding curve phase (details in separate protocol asset allocation document)
* **Dynamic Inventory**: Buys through JIT add to inventory; sells reduce it
* **Path Dependency**: In contrarian mode, the JIT can only place asks if it has sufficient inventory from either:
  - Initial protocol allocation
  - Previous buy-side fills
* **Inventory Deployment Delay**: After a JIT bid is filled, a cooldown period prevents immediate ask placement using the newly acquired inventory
* **Floor-neutral policy**: When selling Δs tokens receiving Δq_actual quote, commit = min(Δq_actual, ceil(P_floor * Δs)) from τ to R_*. Assert P_floor' >= P_floor post-state.
* **Matured inventory**: Only asks from inventory that has aged >= INVENTORY_MATURITY_SLOTS can be used (prevents rapid force-feed-dump cycles)
* **Rebalancing**: Periodic rebalancing mechanism adjusts inventory levels (specified in separate document)

This ensures the JIT can provide two-sided liquidity from launch, with the bonding curve allocation bootstrapping initial sell-side capacity while preventing inventory manipulation attacks.

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
   - All updates flow through the protocol's unified post-swap handler
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

// Market-tier parameters (adjustable per market maturity)
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

## Deployment Phases

The JIT system deployment follows the protocol-wide phased approach:

### Phase 1: Foundation (Weeks 1-2)
- Deploy unified infrastructure (pool::Oracle, pool::Floor, SafetyController, FlowSignals)
- JIT system deployed but inactive
- Collect baseline market data

### Phase 2: Activation (Weeks 3-4)
- Enable JIT with conservative parameters
- Burn-by-default policy for all JIT fills
- Tight budgets: MAX_PER_SWAP_Q = Q_PIVOT / 100
- Wide spreads: BASE_SPREAD_TICKS = 3

### Phase 3: Optimization (Weeks 5-8)
- Monitor flow signal patterns across all subsystems
- Gradually increase budgets based on toxicity levels
- Tighten spreads in stable markets
- Begin testing inventory hold strategies

### Phase 4: Maturity (Week 8+)
- Full JIT functionality with tuned parameters
- Consider matured inventory model after extensive data
- Dynamic parameter adjustment based on market conditions
- Integration with advanced routing strategies

The phased approach ensures all unified components are battle-tested before full JIT activation.

## Success Metrics

* **Continuity**: fail-to-execute rate at low liquidity
* **Bounded exposure**: realized JIT PnL variance per swap within target
* **Pick-off control**: FlowSignals remain low; appropriate throttling
* **Organic depth**: stable or improving outside micro-spread bands
* **System coordination**: 
  - Oracle health maintained above 95%
  - Floor never breached by JIT operations
  - Safety interventions rare and appropriate
  - Toxicity signals consistent across subsystems
* **Performance**: CU usage within bounds; no degradation of swap execution
