---
title: "Concept Cards"
description: "Quick reference cards for Feels Protocol components and concepts"
category: "Reference"
order: 0
---

# Concept Cards

3-5 line summaries of key components and concepts. Read these before diving into full documentation.

## Core Components

### market::Oracle (GTWAP)
Per-pool geometric time-weighted average price oracle. Updates on every swap using ring buffer of tick observations. Provides manipulation-resistant price anchor for fees, JIT, and floor calculations. Minimum 60-second averaging window.
- **State**: `OracleState` with 12-slot `observations` ring buffer
- **Key methods**: `update(tick, timestamp)`, `get_twap_tick(seconds_ago)`
- **Full doc**: [204-pool-oracle.md](204-pool-oracle.md)

### market::Floor
Pool-level floor price calculator deriving minimum safe price from pool reserves and token supply. Ratchets up monotonically with cooldown period. Provides safety bounds preventing JIT and fees from operating below floor.
- **State**: `PoolFloor` with `current_floor`, `jitosol_reserves`, `total_feels_supply`
- **Key methods**: `calculate_floor_tick()`, `get_safe_ask_tick()`, `can_ratchet()`
- **Formula**: Floor Price = Pool Allocated Reserves / Circulating Supply
- **Full doc**: [205-floor-liquidity.md](205-floor-liquidity.md)

### PoolController
Per-pool component managing economic incentives and capital allocation. Handles fee distribution across LPs, reserves, buffer, treasury, and creator. Orchestrates phase transitions from bonding curve to steady state.
- **State**: `PoolController` with fee split config, phase tracking, reserve/buffer references
- **Responsibilities**: Fee splits, strategy allocation, lifecycle management
- **Full doc**: [206-pool-allocation.md](206-pool-allocation.md)

### SafetyController
Global protocol component coordinating risk management across all pools. Monitors oracle health, liquidity conditions, and solvency metrics. Implements circuit breakers and graceful degradation modes when thresholds exceeded.
- **Monitors**: Oracle freshness, volatility, solvency ratios, rate limits
- **Actions**: Pause features, throttle operations, emit degradation signals
- **Scopes**: Global (all operations), per-pool, per-feature (JIT, redemptions)
- **Full doc**: [210-safety-controller.md](210-safety-controller.md)

### FlowSignals
Unified component tracking market flow patterns shared across fee model, JIT, and other subsystems. Combines flow EWMA (directional momentum) with JIT toxicity observations to provide coordinated market stress signals.
- **State**: `flow_ewma` (signed), `directional_toxicity`, `combined_signal`
- **Updated by**: Swap execution, JIT fills
- **Consumed by**: Dynamic fees (equilibrium), JIT (sizing/spreads)
- **Full doc**: [201-dynamic-fees.md](201-dynamic-fees.md#state-management-with-unified-components)

### protocol::Oracle
Protocol-level oracle providing conservative FeelsSOL↔JitoSOL exchange rate for global solvency and redemption. Takes minimum of Jito native rate and filtered DEX TWAP with divergence guards. Separate from per-pool GTWAP.
- **Sources**: Jito protocol rate (monotonic), DEX TWAP (30min window, liquidity-filtered)
- **Safety**: Conservative min() composition, 150 bps depeg circuit breaker
- **Uses**: `enter_feelssol`/`exit_feelssol`, protocol treasury accounting
- **Full doc**: [200-feelssol-solvency.md](200-feelssol-solvency.md#oracle-architecture-layered)

## Pool Accounts

### Pool (Market)
Central account for trading pair containing AMM state, configuration, and auxiliary account references. Zero-copy account holding sqrt_price, active liquidity, current tick, and fee accumulators.
- **Key fields**: `sqrt_price`, `liquidity`, `current_tick`, `token_0`, `token_1`, `tick_spacing`
- **PDA seed**: `[b"market", token_0, token_1]`
- **Invariant**: One token must be FeelsSOL (hub-and-spoke)
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#pool)

### PoolBuffer (τ, tau)
Per-pool tactical account funding JIT liquidity and routing creator fees. Accumulates portion of swap fees as working capital for active market making. Symbol τ (tau) in documentation.
- **Funding**: 20% of swap fees (MVP) + 5% of bonding curve capital at graduation
- **Uses**: JIT liquidity budget, creator base fee accruals
- **Budget caps**: Per-swap (10-30 bps), per-slot (30-60 bps), configurable
- **Full doc**: [202-jit-liquidity.md](202-jit-liquidity.md#budgets--sizing)

### PoolReserve
Per-pool strategic capital backing floor liquidity position. Isolated per market ensuring independent solvency. Accumulates swap fees and staking yield allocations for long-term floor support.
- **Funding**: 25% of swap fees (MVP) + 95% of bonding curve capital at graduation
- **Use**: Single-sided floor liquidity position at calculated floor tick
- **Allocation**: Periodically receives share of protocol-wide staking yield
- **Full doc**: [205-floor-liquidity.md](205-floor-liquidity.md), [200-feelssol-solvency.md](200-feelssol-solvency.md#pool-level-solvency)

### Position
User's concentrated liquidity position represented as NFT. Tracks tick range, liquidity amount, and fee growth snapshots for calculating uncollected fees. Each position is unique SPL token mint.
- **Key fields**: `tick_lower`, `tick_upper`, `liquidity`, `fee_growth_inside_*_last_x64`, `tokens_owed_*`
- **NFT**: 1 token minted to LP, burned on close
- **Fee calculation**: Current fee_growth_inside minus last snapshot, multiplied by liquidity
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#positions)

### TickArray
Fixed-size array storing tick data for contiguous 64-tick range. Zero-copy account with lazy initialization of individual ticks. PDA-addressed for deterministic lookup during swaps.
- **Size**: 64 ticks per array (configurable `TICK_ARRAY_SIZE`)
- **PDA seed**: `[b"tick_array", pool, start_tick_index]`
- **Lazy init**: Ticks only initialized when first liquidity added
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#tick-arrays)

### PreLaunchEscrow
Temporary account holding token supply and mint fee before market deployment. Links to market after initialization, cleared when initial liquidity deployed or token expires.
- **Contents**: Full token supply (1B tokens), mint fee (1000 FeelsSOL)
- **Lifecycle**: Created in `mint_token` → linked in `initialize_market` → cleared in `deploy_initial_liquidity`
- **Expiration**: Can be destroyed if no liquidity deployed within expiration window
- **Full doc**: [300-launch-sequence.md](300-launch-sequence.md#step-2-mint-protocol-token)

## Liquidity Strategies

### Floor Liquidity (POMM)
Protocol-owned single-sided position providing guaranteed exit price. Large ask position at floor tick selling project tokens for FeelsSOL. Ratchets up monotonically as reserves grow, never decreases.
- **Type**: Passive, long-term price support
- **Position**: `[global_lower_tick, floor_tick]` range, 100% FeelsSOL (asks)
- **Sizing**: Can absorb entire circulating supply at floor price
- **Full doc**: [205-floor-liquidity.md](205-floor-liquidity.md)

### JIT Liquidity (POMM)
Automated contrarian market-making placing micro-spreads opposite incoming trades. Place-execute-remove in single transaction. MVP v0.5 uses virtual concentration for 5-10x revenue vs basic JIT.
- **Type**: Active, responsive to trading flow
- **Anchor**: GTWAP with floor guard, clamped to current price
- **Placement**: 3-tick spread, 1-tick range, contrarian to taker intent
- **Budgets**: 10-30 bps per swap, 30-60 bps per slot (of PoolBuffer)
- **Safety**: Toxicity throttling, drain protection, circuit breakers
- **Full doc**: [202-jit-liquidity.md](202-jit-liquidity.md)

### Bonding Curve Liquidity
Temporary price discovery mechanism using 10-40 discretized staircase positions approximating x*y=k curve. Active during Phase 1 before graduation. Prevents third-party LPing to ensure fair launch.
- **Type**: Temporary, protocol-only price discovery
- **Implementation**: Micro-ranges with calculated liquidity per tranche
- **Duration**: Until market cap target met or time/volume milestone
- **Transition**: Withdrawn and reallocated to Floor + JIT at graduation
- **Full doc**: [207-bonding-curve-feels.md](207-bonding-curve-feels.md)

## Fee and Economic Concepts

### Dynamic Fees (MVP)
Post-execution fee calculation using base + realized impact. Base fee (30 bps) + impact fee (ticks moved with 10 bps floor). Phase 2 adds momentum and equilibrium adjustments (deferred).
- **Formula**: `total_fee = base_fee + max(ticks_to_bps(ticks_moved), impact_floor)`
- **Bounds**: 20-150 bps total, user can set max cap
- **Anti-gaming**: Impact floor prevents split-trade exploitation
- **Full doc**: [201-dynamic-fees.md](201-dynamic-fees.md#the-model-mvp)

### Fee Split (MVP)
Distribution of collected swap fees among protocol stakeholders. Fixed percentages configured in protocol parameters, applied consistently on every swap.
- **Recipients**: LPs (45%), PoolReserve (25%), PoolBuffer (20%), Treasury (8%), Creator base (2%)
- **Total**: Always sums to 100% (10,000 bps)
- **Governance**: Can adjust split via protocol parameters
- **Full doc**: [206-pool-allocation.md](206-pool-allocation.md#fee-recipients-mvp)

### Impact Floor
Minimum dynamic fee component preventing split-trade gaming. Even tiny price movements pay floor fee, making trade splitting economically unprofitable.
- **Value**: 10 bps (MVP default)
- **Effect**: Split 1000 token trade → 10x trades each paying 10 bps floor = 100 bps total vs 50 bps single trade
- **Rationale**: Simple, robust anti-gaming without additional state
- **Full doc**: [201-dynamic-fees.md](201-dynamic-fees.md#anti-gaming-via-impact-floor)

### Mint Fee
One-time fee to create protocol token, paid in FeelsSOL. Held in escrow until successful deployment, split if token expires unused.
- **Amount**: 1000 FeelsSOL (MVP default, governance-configurable)
- **Escrow**: Held in PreLaunchEscrow until `deploy_initial_liquidity`
- **Success**: 100% to treasury when liquidity deployed
- **Expiration**: 50% to destroyer (bounty), 50% to treasury
- **Full doc**: [300-launch-sequence.md](300-launch-sequence.md#step-2-mint-protocol-token)

## Safety Concepts

### Circuit Breaker
Emergency mechanism halting operations when critical thresholds breached. Can target specific features (JIT only) or entire protocol (all swaps). Activated by SafetyController based on observed metrics.
- **JIT triggers**: Buffer health < 30%, price movement > 10% in 1hr
- **Protocol triggers**: Oracle de-peg > 150 bps for 3+ observations
- **Actions**: Pause JIT, pause redemptions, throttle operations
- **Full doc**: [210-safety-controller.md](210-safety-controller.md#actions-matrix-mvp)

### Degraded Mode
Reduced functionality state when safety conditions suboptimal but not critical. System remains operational with limited features. Preferable to complete shutdown for user experience.
- **GTWAP stale**: Disable rebates, raise impact floor, allow swaps
- **Protocol oracle stale**: Pause redemptions, allow swaps
- **Gradual**: Multiple degradation levels (0=healthy, 1-3=degraded, 4+=critical)
- **Full doc**: [210-safety-controller.md](210-safety-controller.md), [208-after-swap-pipeline.md](208-after-swap-pipeline.md#degraded-mode-rules)

### Toxicity Tracking
JIT measure of adverse price movement after fills. Tracks directional adversity (price moves against filled side) using exponential moving average. Higher toxicity reduces JIT participation and widens spreads.
- **Observation**: Adverse = (bid filled & price down) OR (ask filled & price up)
- **Update**: Asymmetric EMA (faster increase on adverse, slower decay)
- **Response**: Reduce size via α (alpha) factor, widen micro-spreads
- **Full doc**: [202-jit-liquidity.md](202-jit-liquidity.md#toxicity-tracking-with-unified-system)

### Ratcheting
Mechanism allowing only increases, never decreases. Applied to floor price ensuring monotonic upward movement. Cooldown period prevents frequent updates and manipulation attempts.
- **Applied to**: Floor price tick
- **Cooldown**: 1800 slots (~12 minutes at 0.4s/slot)
- **Condition**: New calculated floor > current floor AND cooldown passed
- **Guarantee**: Floor can never decrease, only rise or stay constant
- **Full doc**: [205-floor-liquidity.md](205-floor-liquidity.md#monotonic-ratcheting)

### Solvency Invariants
Mathematical guarantees ensuring protocol can honor obligations. Two-layer model: pool-level (each market) and protocol-level (global reserves).
- **Pool-level**: FeelsSOL in Pool ≥ Reasonable Exit Needs
- **Protocol-level**: JitoSOL Reserves ≥ FeelsSOL Total Supply
- **Floor guarantee**: Each pool's floor backed by isolated PoolReserve
- **Full doc**: [200-feelssol-solvency.md](200-feelssol-solvency.md#solvency-invariants)

## Lifecycle Concepts

### Pool Phases
State machine defining market lifecycle from creation through maturity. Each phase has distinct liquidity strategies and operational characteristics.
- **Phase 0: Uninitialized** - Token minted, no market
- **Phase 1: Price Discovery** - Bonding curve active, third-party LPing disabled
- **Phase 2: Steady State** - Floor + JIT active, open to public LPs
- **Transitions**: `initialize_market` → `deploy_initial_liquidity` → `graduate_pool`
- **Full doc**: [301-market-state-and-lifecycle.md](301-market-state-and-lifecycle.md#the-market-lifecycle-state-machine)

### Graduation
Transition from bonding curve (Phase 1) to steady state (Phase 2). Triggered when market cap target met. Reallocates capital from staircase to Floor (95%) and JIT (5%) without downtime.
- **Trigger**: Permissionless crank after cumulative FeelsSOL raised ≥ target (e.g., 85 SOL)
- **Process**: Deploy Floor + seed JIT buffer BEFORE removing bonding curve positions
- **Zero downtime**: New liquidity active before old liquidity removed
- **Full doc**: [207-bonding-curve-feels.md](207-bonding-curve-feels.md#pool-lifecycle), [300-launch-sequence.md](300-launch-sequence.md#step-6-pool-graduation-future)

### Warmup Period
Initial phase where advanced features disabled to prevent manipulation with early noisy data. Requires BOTH time elapsed AND trading volume before full activation.
- **Duration**: 2400 slots (~20 min) AND 150 trades minimum
- **During warmup**: Rebates disabled, equilibrium bias reduced, base+impact fees only
- **Phase 2 feature**: Not in MVP (fees are base+impact always)
- **Full doc**: [201-dynamic-fees.md](201-dynamic-fees.md#phase-2-deferred-warmup-ramp)

## Technical Concepts

### Zero-Copy
Memory layout optimization allowing direct account data access without deserialization. Reduces compute units and enables efficient handling of large accounts.
- **Applied to**: TickArray, large state accounts
- **Benefit**: O(1) access, lower CU costs, better performance
- **Trade-off**: Fixed size, must plan capacity ahead
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#tick-arrays)

### Tick
Discrete logarithmic price point. Entire price range divided into ticks where `price = 1.0001^tick_index`. Moving one tick changes price by ~1 basis point (0.01%).
- **Spacing**: Liquidity only allowed at multiples of tick_spacing (e.g., 1, 10, 100)
- **Current tick**: Pool's current price position
- **Tick range**: Position's [tick_lower, tick_upper] bounds
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#price-ticks-and-liquidity)

### Sqrt Price
Price representation as Q64.64 fixed-point square root. Simplifies CLMM math for liquidity-to-amount conversions.
- **Format**: `sqrt_price = sqrt(price_token1 / price_token0) * 2^64`
- **Why**: Makes constant product formulas more efficient
- **Conversion**: `tick_from_sqrt_price()`, `sqrt_price_from_tick()`
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#price--tick-conversion)

### Liquidity (L)
Virtual quantity representing market depth in CLMM. Determines amount of tokens in a price range. Higher liquidity = more tokens = less slippage.
- **Units**: Abstract, relates reserves to price range
- **Formulas**: `Δx = L * (1/√P_upper - 1/√P_lower)`, `Δy = L * (√P_upper - √P_lower)`
- **Active liquidity**: Sum of liquidity_net from all crossed ticks
- **Full doc**: [203-pool-clmm.md](203-pool-clmm.md#liquidity--amounts-conversion)

### Hub-and-Spoke Routing
Architecture pattern requiring all trading pairs to include central hub token (FeelsSOL). Eliminates fragmented liquidity and ensures predictable, bounded routing.
- **Max hops**: 2 (TokenA → FeelsSOL → TokenB)
- **Direct trades**: Token ↔ FeelsSOL (1 hop)
- **Benefit**: Concentrated liquidity, simplified routing, better prices
- **Constraint**: token_0 must be FeelsSOL (lower pubkey)
- **Full doc**: [003-hub-and-spoke-architecture.md](003-hub-and-spoke-architecture.md)

## Quick Reference: When to Read What

**Implementing swaps**: 203 §4.4, 201 §4-5, 208
**Launching tokens**: 300 (complete), 207 §2-4
**Working with positions**: 203 §1.2, §4.2-4.3
**Understanding pricing**: 204 (complete), 203 §3.1
**Safety mechanisms**: 210 (complete), 200 §3
**Configuration**: 209 (complete), 211 (units)
**Solvency guarantees**: 200 §2, §5
**Protocol-owned liquidity**: 205 (floor), 202 (JIT)

**See full navigation**: [DOCS-INDEX.md](DOCS-INDEX.md)
**See full glossary**: [GLOSSARY.md](GLOSSARY.md)

