---
title: "Glossary"
description: "Terms, abbreviations, and component reference for Feels Protocol"
category: "Reference"
order: 0
---

# Glossary

Quick reference for terms, abbreviations, and components used throughout the Feels Protocol documentation.

## Core Terms

### FeelsSOL
The hub token in the protocol's hub-and-spoke architecture. Backed 1:1 by JitoSOL reserves held in protocol vaults. All trading pairs must include FeelsSOL.
- **Minting**: Deposit JitoSOL via `enter_feelssol` to mint FeelsSOL 1:1
- **Redemption**: Burn FeelsSOL via `exit_feelssol` to redeem JitoSOL
- **See**: [003-hub-and-spoke-architecture.md](003-hub-and-spoke-architecture.md), [200-feelssol-solvency.md](200-feelssol-solvency.md)

### JitoSOL
Liquid staking token that serves as the backing asset for all FeelsSOL. Earns staking rewards over time, which accrue to protocol reserves.
- **See**: [200-feelssol-solvency.md](200-feelssol-solvency.md#backing-invariant)

### GTWAP (Geometric Time-Weighted Average Price)
A manipulation-resistant price oracle that calculates the geometric mean of prices over time by averaging tick indices. Each pool maintains its own GTWAP.
- **Implementation**: Ring buffer of tick observations updated on every swap
- **Minimum window**: 60 seconds
- **See**: [204-pool-oracle.md](204-pool-oracle.md)

### CLMM (Concentrated Liquidity Market Maker)
Uniswap V3-style AMM where liquidity providers can concentrate capital in specific price ranges for improved capital efficiency.
- **Key concepts**: Ticks, tick arrays, positions, sqrt_price
- **See**: [203-pool-clmm.md](203-pool-clmm.md)

### Hub-and-Spoke Architecture
Design pattern where all tokens trade through a central hub (FeelsSOL), eliminating fragmented liquidity and ensuring maximum 2-hop routes.
- **Direct trade**: Token ↔ FeelsSOL (1 hop)
- **Cross trade**: TokenA → FeelsSOL → TokenB (2 hops)
- **See**: [003-hub-and-spoke-architecture.md](003-hub-and-spoke-architecture.md)

## Liquidity Concepts

### Floor Liquidity
Protocol-owned, single-sided liquidity position that provides a guaranteed price floor. Funded by pool reserves, ratchets up monotonically over time.
- **Formula**: Floor Price = Pool Allocated Reserves / Circulating Supply
- **Characteristic**: Can only rise or stay constant, never falls
- **See**: [205-floor-liquidity.md](205-floor-liquidity.md)

### JIT Liquidity (Just-In-Time)
Automated market-making strategy that places contrarian liquidity opposite to incoming trades, funded by pool buffer (τ).
- **Placement**: Around GTWAP anchor with micro-spreads
- **Execution**: Place-execute-remove in single transaction
- **MVP**: JIT v0.5 with virtual concentration
- **See**: [202-jit-liquidity.md](202-jit-liquidity.md)

### POMM (Protocol-Owned Market Making)
General term for automated liquidity strategies owned and operated by the protocol, including Floor and JIT.
- **See**: [205-floor-liquidity.md](205-floor-liquidity.md), [202-jit-liquidity.md](202-jit-liquidity.md)

### Bonding Curve
Initial price discovery mechanism using discretized staircase liquidity pattern. Active during pool Phase 1 before graduation.
- **Implementation**: 10-40 micro-ranges approximating x*y=k curve
- **Graduation**: Transitions to steady-state (Floor + JIT) after market cap target
- **See**: [207-bonding-curve-feels.md](207-bonding-curve-feels.md)

## Protocol Components

### market::Oracle
Per-pool GTWAP oracle component. Calculates geometric time-weighted average price using ring buffer of tick observations.
- **State account**: `OracleState`
- **Key methods**: `update()`, `get_twap_tick(seconds_ago)`
- **Ring buffer**: 12 observations (configurable)
- **See**: [204-pool-oracle.md](204-pool-oracle.md)

### market::Floor
Pool-level floor price calculator. Determines minimum safe price based on pool reserves and circulating supply.
- **State**: `PoolFloor` struct
- **Key methods**: `calculate_floor_tick()`, `can_ratchet()`, `get_safe_ask_tick()`
- **Ratcheting**: Monotonic increases with cooldown period
- **See**: [205-floor-liquidity.md](205-floor-liquidity.md#the-poolfloor)

### PoolController
Per-pool component managing fee distribution, strategy allocation, and lifecycle phases.
- **Responsibilities**: Fee splits, capital allocation, phase transitions
- **State account**: `PoolController`
- **See**: [206-pool-allocation.md](206-pool-allocation.md)

### SafetyController
Global protocol component coordinating risk management and degraded operation modes.
- **Monitors**: Oracle health, liquidity conditions, solvency metrics
- **Actions**: Circuit breakers, rate limiting, feature degradation
- **See**: [210-safety-controller.md](210-safety-controller.md), [200-feelssol-solvency.md](200-feelssol-solvency.md#protocol-safety-controller)

### FlowSignals
Unified component tracking market flow patterns for coordination across fee model, JIT, and other subsystems.
- **Signals**: Flow EWMA, directional toxicity, combined signal
- **Consumers**: Dynamic fees, JIT sizing, spread adjustments
- **See**: [201-dynamic-fees.md](201-dynamic-fees.md#state-management-with-unified-components)

### protocol::Oracle
Protocol-level oracle providing conservative FeelsSOL↔JitoSOL exchange rate for global solvency.
- **Sources**: Jito native rate + filtered DEX TWAP
- **Safety**: Takes minimum of available rates with divergence guards
- **Distinct from**: market::Oracle (per-pool GTWAP)
- **See**: [200-feelssol-solvency.md](200-feelssol-solvency.md#oracle-architecture-layered)

## Accounts and State

### Pool
Central account for a trading pair. Contains AMM state, configuration, and references to auxiliary accounts.
- **Key fields**: `sqrt_price`, `liquidity`, `current_tick`, `token_0`, `token_1`
- **PDA seed**: `[b"market", token_0, token_1]` (also called "market")
- **See**: [203-pool-clmm.md](203-pool-clmm.md#pool)

### PoolBuffer (τ, tau)
Per-pool tactical account funding JIT liquidity and routing creator fees. Accumulates trading fees.
- **Symbol**: τ (tau) in documentation
- **Funding**: Portion of swap fees + initial seed from bonding curve
- **Uses**: JIT budget, creator base fees (MVP)
- **See**: [202-jit-liquidity.md](202-jit-liquidity.md#budgets--sizing), [206-pool-allocation.md](206-pool-allocation.md)

### PoolReserve
Per-pool strategic capital backing floor liquidity. Isolated per market for solvency.
- **Funding**: Portion of swap fees + staking yield allocation
- **Use**: Floor liquidity position
- **Isolation**: Each pool's reserve independent
- **See**: [205-floor-liquidity.md](205-floor-liquidity.md), [200-feelssol-solvency.md](200-feelssol-solvency.md#pool-level-solvency)

### Position
User's concentrated liquidity position, represented as NFT. Tracks range, liquidity, and uncollected fees.
- **State account**: `Position`
- **Key fields**: `tick_lower`, `tick_upper`, `liquidity`, `fee_growth_inside_*`
- **NFT**: Each position is unique SPL token
- **See**: [203-pool-clmm.md](203-pool-clmm.md#positions)

### TickArray
Fixed-size array storing tick data for a contiguous range. Uses zero-copy for efficiency.
- **Size**: 64 ticks per array (configurable)
- **Lazy init**: Ticks initialized only when liquidity added
- **PDA**: Derived from pool and start tick index
- **See**: [203-pool-clmm.md](203-pool-clmm.md#tick-arrays)

### PreLaunchEscrow
Temporary account holding tokens and fees before market deployment. Links to market after initialization.
- **Contents**: All token supply, mint fee in FeelsSOL
- **Lifecycle**: Created in `mint_token`, linked in `initialize_market`, cleared in `deploy_initial_liquidity`
- **See**: [300-launch-sequence.md](300-launch-sequence.md)

## Abbreviations and Units

### General Abbreviations
- **AMM**: Automated Market Maker
- **TWAP**: Time-Weighted Average Price
- **GTWAP**: Geometric TWAP (averages log prices / tick indices)
- **LP**: Liquidity Provider
- **JIT**: Just-In-Time (liquidity)
- **POMM**: Protocol-Owned Market Making
- **POL**: Protocol-Owned Liquidity
- **PDA**: Program Derived Address (Solana)
- **CU**: Compute Units (Solana transaction cost)
- **MVP**: Minimum Viable Product (initial release scope)

### Unit Suffixes
- **`_bps`**: Basis points (1/10,000). Example: 30 bps = 0.30%
- **`_ticks`**: Price ticks on logarithmic scale. 1 tick ≈ 0.01% price change
- **`_q`**: Quote token amount (FeelsSOL in most contexts)
- **`_q16`**: Fixed-point Q16 format (16 fractional bits)
- **`_q32`**: Fixed-point Q32 format (32 fractional bits)
- **`_x64`**: Q64.64 format (64.64 fixed point, used for prices)
- **`_pct`**: Percentage (0-100)

### Special Symbols
- **τ (tau)**: PoolBuffer account (tactical working capital)
- **R_***: Floor reserves (strategic floor capital)
- **α (alpha)**: Size adjustment factor in JIT (toxicity-based throttling)
- **L**: Liquidity amount in CLMM formulas

## Pool Lifecycle Phases

### Phase 0: Uninitialized
Token minted but market not yet created.
- **Next**: `initialize_market` → Phase 1

### Phase 1: Price Discovery (Bonding Curve)
Initial liquidity deployed in staircase pattern. Third-party LPing disabled.
- **Duration**: Fixed time or volume milestone
- **Liquidity**: 10-40 discretized ranges approximating bonding curve
- **Next**: `graduate_pool` → Phase 2

### Phase 2: Steady State
Mature market with Floor + JIT liquidity. Open to third-party LPs.
- **Floor**: Large single-sided position at calculated floor
- **JIT**: Active contrarian market making
- **LPs**: Public can open positions

## Fee and Economic Terms

### Base Fee
Static component of trading fee, set per pool. MVP default 30 bps.
- **See**: [201-dynamic-fees.md](201-dynamic-fees.md#the-model-mvp)

### Impact Fee
Dynamic component based on realized tick movement. Calculated post-swap.
- **Formula**: `ticks_moved` with lookup table and floor
- **Floor**: Minimum 10 bps to prevent split-trade gaming
- **See**: [201-dynamic-fees.md](201-dynamic-fees.md#price-impact-calculation)

### Fee Split
Distribution of collected fees among stakeholders.
- **MVP recipients**: LPs (45%), PoolReserve (25%), PoolBuffer (20%), Protocol Treasury (8%), Creator base (2%)
- **Configurable**: Via protocol parameters
- **See**: [206-pool-allocation.md](206-pool-allocation.md#fee-recipients-mvp)

### Rebates (Phase 2, Deferred)
Fee discounts for equilibrium-restoring trades. Not in MVP.
- **Mechanism**: Negative adjustments to total fee calculation
- **Funded by**: PoolBuffer foregone revenue
- **See**: [201-dynamic-fees.md](201-dynamic-fees.md#understanding-swapper-rebates-phase-2)

### Mint Fee
One-time fee paid in FeelsSOL to create new protocol token. Held in escrow until deployment.
- **Default**: 1000 FeelsSOL (configurable)
- **Destination**: 100% to treasury after successful deployment; 50/50 split if expired
- **See**: [300-launch-sequence.md](300-launch-sequence.md#step-2-mint-protocol-token)

## Safety and Risk Terms

### Solvency Ratio
Measure of protocol's ability to honor redemptions.
- **Protocol-level**: JitoSOL Reserves ≥ FeelsSOL Total Supply
- **Pool-level**: FeelsSOL in Pool ≥ Liquidity Exit Needs
- **See**: [200-feelssol-solvency.md](200-feelssol-solvency.md#two-layer-solvency-model)

### Circuit Breaker
Emergency mechanism halting operations when thresholds breached.
- **Triggers**: Buffer depletion, extreme price movement, oracle de-peg
- **Scope**: Can be global (all swaps) or targeted (JIT only, redemptions only)
- **See**: [202-jit-liquidity.md](202-jit-liquidity.md#circuit-breaker), [210-safety-controller.md](210-safety-controller.md)

### Degraded Mode
Reduced functionality state when safety conditions not optimal but not critical.
- **Example**: GTWAP stale → disable rebates but allow swaps
- **Graceful**: System remains usable with reduced features
- **See**: [210-safety-controller.md](210-safety-controller.md#actions-matrix-mvp)

### Rate Limiting
Throttling mechanism to prevent resource exhaustion.
- **Applies to**: Swap volume per slot, JIT budget consumption, oracle updates
- **Granularity**: Per-pool per-slot typically
- **See**: [209-params-and-governance.md](209-params-and-governance.md#rate-limiting)

### Toxicity
Measure of adverse price movement after JIT fills, used to throttle future participation.
- **Tracking**: Exponential moving average (EMA) with asymmetric update rates
- **Response**: Reduce JIT size, widen spreads
- **See**: [202-jit-liquidity.md](202-jit-liquidity.md#toxicity-tracking-with-unified-system)

### Staleness
Condition when oracle data is outdated beyond acceptable threshold.
- **Threshold**: Typically 150 slots (~60 seconds at 0.4s/slot)
- **Response**: Degrade to fallback behavior (e.g., use current price)
- **See**: [204-pool-oracle.md](204-pool-oracle.md#parameters--staleness)

## Implementation Terms

### Zero-Copy
Memory layout optimization allowing direct account data access without deserialization.
- **Used for**: TickArray, large state accounts
- **Benefit**: Reduced compute units, better performance
- **See**: [203-pool-clmm.md](203-pool-clmm.md#tick-arrays)

### Ratcheting
Mechanism allowing monotonic increases only, never decreases.
- **Applied to**: Floor price
- **Cooldown**: Prevents frequent updates (e.g., 1800 slots)
- **See**: [205-floor-liquidity.md](205-floor-liquidity.md#monotonic-ratcheting)

### Warmup Period
Initial phase where certain features disabled to prevent manipulation with noisy early data.
- **Duration**: Requires BOTH time (e.g., 2400 slots) AND volume (e.g., 150 trades)
- **During warmup**: Rebates disabled, equilibrium bias reduced
- **See**: [201-dynamic-fees.md](201-dynamic-fees.md#phase-2-deferred-warmup-ramp)

### Unified Components
Shared infrastructure used consistently across subsystems to reduce duplication.
- **Examples**: market::Oracle, market::Floor, FlowSignals, SafetyController
- **Benefit**: Consistency, reduced state, coordinated responses
- **See**: [200-feelssol-solvency.md](200-feelssol-solvency.md#component-integration)

## Cross-References

### By Topic

**Token Launches**: 
- [300-launch-sequence.md](300-launch-sequence.md), [207-bonding-curve-feels.md](207-bonding-curve-feels.md), [301-market-state-and-lifecycle.md](301-market-state-and-lifecycle.md)

**Trading & Swaps**: 
- [203-pool-clmm.md](203-pool-clmm.md), [201-dynamic-fees.md](201-dynamic-fees.md), [208-after-swap-pipeline.md](208-after-swap-pipeline.md)

**Oracles & Pricing**: 
- [204-pool-oracle.md](204-pool-oracle.md), [200-feelssol-solvency.md](200-feelssol-solvency.md#oracle-architecture-layered)

**Liquidity & Market Making**: 
- [202-jit-liquidity.md](202-jit-liquidity.md), [205-floor-liquidity.md](205-floor-liquidity.md), [206-pool-allocation.md](206-pool-allocation.md)

**Safety & Governance**: 
- [210-safety-controller.md](210-safety-controller.md), [209-params-and-governance.md](209-params-and-governance.md)

**Protocol Architecture**: 
- [003-hub-and-spoke-architecture.md](003-hub-and-spoke-architecture.md), [200-feelssol-solvency.md](200-feelssol-solvency.md)

