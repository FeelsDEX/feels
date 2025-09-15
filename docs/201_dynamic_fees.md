# Dynamic Fee Model (MVP: Base + Impact Only)

This document presents the MVP pool-level dynamic fee model that computes fees entirely on-chain after swap execution. Terminology: “protocol” refers to global systems (treasury, reserve rate oracle, safety), while “pool” refers to per‑market systems (CLMM, GTWAP oracle, fees, JIT, floor).

## Physics Analogy: Markets as Energy Networks

Our goal is to create a sustainable market ecosystem that maintains price stability within a tradeable range while monetizing the volatility necessary for price discovery. For MVP, we prioritize a simple and predictable model (base + impact) while preserving a clear path to richer dynamics (equilibrium bias, momentum) in Phase 2. The physics analogy below provides motivation for future phases.

The system can be analogized to an energy network: user flow adds energy, and liquidity providers offer channels for it to move. Dynamic fees act as the system's impedance, they regulate current so throughput stays healthy. Trades that climb uphill (away from equilibrium) must perform work against resistance and pay a surcharge. Trades that flow downhill (toward equilibrium) harness potential energy and receive a discount. This creates a self-correcting market that remains simple enough to implement and audit fully on-chain.

Liquidity providers are the conductors in this network, they facilitate energy transfer and earn fees proportional to the work performed through their channels. When markets are calm and near equilibrium, LPs earn steady base fees from balanced flow. But when displacement grows or momentum builds, the increased impedance generates higher fees, compensating LPs for the additional risk of facilitating trades during volatile conditions. This dynamic creates a natural feedback loop: higher volatility increases LP returns, attracting more liquidity precisely when the market needs additional capacity to absorb shocks. Conversely, during quiet periods, lower fees prevent excessive liquidity from accumulating where it isn't needed. The system thus encourages LPs to act as dynamic stabilizers, expanding and contracting their positions in response to market stress rather than simply providing static liquidity.

We track displacement magnitude as |current_tick - twap_tick|, representing stored potential energy with larger deviations implying greater instability. Flow momentum captures recent directional bias through a signed EWMA (exponentially weighted moving average), indicating whether the market has kinetic energy pushing it further from balance. The impact floor acts as friction, ensuring even small trades contribute to system maintenance. By computing fees after swap execution, we measure actual work performed: the realized tick movement tells us exactly how much the market was displaced, avoiding prediction errors and gaming opportunities. 

This approach treats fees as impedance that increases with displacement (potential) and flow persistence (momentum), bounded by caps to prevent runaway dynamics. The result optimizes for steady-state operation while monetizing volatility. Fees naturally flow from traders who add instability to those who restore balance, creating a sustainable ecosystem where the market's natural tendency toward equilibrium is reinforced by economic incentives. For LPs, this means their role evolves from passive capital providers to active market stabilizers, with returns that directly reflect their contribution to maintaining market health.

## Core Design Principles

1. **Compute fees after swap execution** using actual realized price impact
2. **Single primary signal** (price impact) with multiplicative modulation
3. **Anti-gaming** via impact floor to prevent split-trade exploitation
4. **Precise integer math** with clear units and bounds
5. **Minimal state** and O(1) computation outside the swap loop

## The Model (MVP)

### Fee Calculation (After Swap)

**Behavior**: This function is called after the swap has executed, using the actual start and end ticks to determine the realized price impact. It adds impact_bps to a base fee to produce a final fee in basis points.

**Intuition**: By calculating fees after the fact, we avoid prediction errors. The fee grows with realized market impact and is independent of oracle state.

**Rationale**: Post-execution calculation ensures we charge based on actual market stress caused, not estimates. This eliminates gaming via slippage manipulation and removes the need for any off-chain computation.

```rust
pub fn calculate_fee_after_swap(start_tick: i32, end_tick: i32) -> u16 {
    let ticks_moved = (end_tick - start_tick).abs();
    let impact_bps = ticks_to_bps(ticks_moved).max(IMPACT_FLOOR_BPS);
    let total = (BASE_FEE_BPS as u32 + impact_bps as u32)
        .clamp(MIN_TOTAL_FEE_BPS as u32, MAX_TOTAL_FEE_BPS as u32);
    total as u16
}
```

## Key Components

### 1. Price Impact Calculation

**Behavior**: Converts the number of ticks the price moved into basis points. For small moves (≤500 ticks), uses linear approximation. For larger moves, consults a precomputed lookup table.

**Intuition**: Each tick represents approximately 1 basis point of price movement (0.01%). This is exact for tick spacing of 1 basis point, and a close approximation for other spacings.

**Rationale**: Ticks are the native unit of price movement in concentrated liquidity. Converting to basis points provides a universal measure that's intuitive for fee calculations and doesn't depend on token decimals or price levels.

```rust
// Precomputed lookup table with enhanced granularity for common trade sizes
const TICK_TO_BPS_TABLE_SMALL: [u16; 11] = [
    0,   // 0 ticks
    10,  // 10 ticks ≈ 10.05 bps
    20,  // 20 ticks ≈ 20.10 bps
    30,  // 30 ticks ≈ 30.15 bps  
    40,  // 40 ticks ≈ 40.20 bps
    50,  // 50 ticks ≈ 50.25 bps
    60,  // 60 ticks ≈ 60.30 bps
    70,  // 70 ticks ≈ 70.35 bps
    81,  // 80 ticks ≈ 80.40 bps
    91,  // 90 ticks ≈ 90.45 bps
    100, // 100 ticks ≈ 100.5 bps
];

const TICK_TO_BPS_TABLE: [u16; 21] = [
    0,    // 0 ticks
    100,  // 100 ticks ≈ 100.5 bps (rounded conservatively)
    201,  // 200 ticks ≈ 202.0 bps
    303,  // 300 ticks ≈ 304.6 bps
    406,  // 400 ticks ≈ 408.1 bps
    510,  // 500 ticks ≈ 512.7 bps
    615,  // 600 ticks
    721,  // 700 ticks
    828,  // 800 ticks
    936,  // 900 ticks
    1046, // 1000 ticks
    1156, // 1100 ticks
    1268, // 1200 ticks
    1381, // 1300 ticks
    1495, // 1400 ticks
    1610, // 1500 ticks
    1726, // 1600 ticks
    1844, // 1700 ticks
    1963, // 1800 ticks
    2083, // 1900 ticks
    2204, // 2000 ticks
];

fn ticks_to_bps(ticks: i32) -> u16 {
    // Enhanced granularity for small moves (most common case)
    if ticks <= 100 {
        let index = (ticks / 10) as usize;
        TICK_TO_BPS_TABLE_SMALL[index]
    } else if ticks <= 2000 {
        // Use standard lookup table
        let index = (ticks / 100).min(20) as usize;
        TICK_TO_BPS_TABLE[index]
    } else {
        // Cap at reasonable maximum
        2500
    }
}
```

**Design Trade-off**: This lookup table approach creates discontinuities at 100-tick boundaries (e.g., 199 ticks = 199 bps, but 200 ticks = 201 bps). This is a conscious choice prioritizing gas efficiency over perfect smoothness. The alternative, an on-chain exponential or polynomial approximation, would be more computationally expensive. For a production system handling high transaction volumes, the gas savings justify the minor pricing discontinuities.

### Phase 2 (Deferred): Momentum Factor (Continuous)

**Behavior**: Produces a multiplicative factor between 80% and 150% based on market momentum. When trades go with momentum, fees increase (up to 1.5x). When trades counter momentum, fees decrease (down to 0.8x).

**Intuition**: Markets with strong directional momentum are more fragile - trades that extend the momentum add to instability and should pay more. Conversely, trades that counter strong momentum help stabilize the market and deserve a discount.

**Rationale**: The hyperbolic curve `adj = MAX * m / (K + m)` provides smooth transitions without arbitrary thresholds. It asymptotically approaches the maximum adjustment, preventing extreme values while maintaining responsiveness to momentum changes.

```rust
// Pre-computed reciprocal for K_MOMENTUM to avoid division
// 1 / 500_000 ≈ 0.000002 in Q32 = 8590 (approximately)
const K_MOMENTUM_RECIPROCAL_Q32: u64 = 8590;

fn calculate_momentum_adjustment(flow_ewma: i64) -> i32 {
    let abs_momentum = flow_ewma.unsigned_abs();
    
    // Using reciprocal multiplication instead of division
    // adj ≈ MAX_ADJ * m / (K + m) 
    // Rewritten as: adj ≈ MAX_ADJ * (1 - K/(K+m))
    // Which becomes: adj ≈ MAX_ADJ * (1 - K*reciprocal/(1 + m*reciprocal))
    let m_scaled = (abs_momentum * K_MOMENTUM_RECIPROCAL_Q32) >> 32;
    let adjustment = (MAX_MOMENTUM_ADJ * m_scaled) / (1 + m_scaled);
    
    // Return raw adjustment - trade direction applied later
    adjustment as i32
}

fn apply_momentum_direction(adjustment: i32, flow_ewma: i64, trade_direction: i8) -> i32 {
    // Check if trade extends or counters momentum
    let with_momentum = (flow_ewma > 0 && trade_direction > 0) || 
                       (flow_ewma < 0 && trade_direction < 0);
    
    if with_momentum {
        100 + adjustment  // Range: 100 to 150
    } else {
        100 - (adjustment * 2 / 5)  // Range: 100 to 80
    }
}
```

### Phase 2 (Deferred): Equilibrium with Two-Part Bias System

**Behavior**: Calculates the equilibrium price target using a two-part system: a gentle capped upward bias during normal conditions, and a hard floor as a failsafe. This combines an "attractive target" that pulls prices up with a "repulsive floor" that prevents dangerous drops.

**Intuition**: The protocol benefits from gentle upward price pressure during normal times, encouraging healthy appreciation. However, this must be capped to prevent unbounded growth. The hard floor ensures safety when markets crash.

**Rationale**: The soft bias provides continuous incentive for upward movement, proportional to distance from the floor but capped at a maximum. The hard floor guarantee ensures the equilibrium never falls into unsafe territory. This synthesis captures the benefits of both approaches without their respective drawbacks.

```rust
fn calculate_biased_equilibrium(
    pool_oracle: &PoolOracle,   // pool::Oracle
    pool_floor: &PoolFloor      // pool::Floor
) -> i32 {
    let twap_tick = pool_oracle.get_twap_tick();
    let floor_tick = pool_floor.current_floor;
    
    // Part 1: Calculate the gentle, capped upward bias for normal conditions
    let distance_from_floor = twap_tick.saturating_sub(floor_tick);
    let capped_distance = distance_from_floor.min(NORMAL_BIAS_CAP_TICKS);
    let soft_bias = (capped_distance as u64 * NORMAL_BIAS_RATE_PCT as u64) / 100;
    let softly_biased_twap = twap_tick + soft_bias as i32;
    
    // Part 2: Use floor manager's safe ask tick as hard floor
    let hard_floor_target = pool_floor.get_safe_ask_tick();
    
    // Part 3: The final equilibrium is the greater of the two
    // This ensures we get the 'soft bias' benefit, but can never fall
    // below the critical safety buffer
    softly_biased_twap.max(hard_floor_target)
}
```

**Example**: With floor at tick 10000, TWAP at 15000:
- Distance from floor: 5000 ticks (capped at cap)
- Soft bias: 5000 × 5% = 250 ticks
- Softly biased TWAP: 15250
- Hard floor: 10000 + 100 = 10100
- Final equilibrium: max(15250, 10100) = 15250

As the market grows and TWAP reaches 20000+, the soft bias caps at NORMAL_BIAS_CAP_TICKS × 5%, preventing unbounded drift.

### Phase 2 (Deferred): Direction Adjustment

**Behavior**: Adds a penalty (positive adjustment) when trades move price away from equilibrium, or a bonus (negative adjustment) when trades restore balance. The bonus scales with how far the market is from equilibrium.

**Intuition**: Markets naturally want to find equilibrium. Trades that push prices further from balance increase systemic stress and should pay more. Trades that restore balance provide a service and deserve lower fees or even rebates.

**Rationale**: The scaling ensures larger incentives when the market needs rebalancing most. The linear scaling with distance (capped at DISTANCE_SCALE) provides predictable incentives without extreme values.

```rust
// Pre-computed reciprocal for DISTANCE_SCALE_TICKS
// 1 / 1000 = 0.001 in Q16 = 66
const DISTANCE_SCALE_RECIPROCAL_Q16: u32 = 66;

fn calculate_direction_adjustment(
    equilibrium_distance: i32,  // current_tick - equilibrium_tick
    trade_direction: i8,        // +1 = buy (tick up), -1 = sell (tick down)
) -> i32 {
    // Combined calculation without separate distance computation
    // equilibrium_distance > 0: price above equilibrium
    // equilibrium_distance < 0: price below equilibrium
    let distance_sign = equilibrium_distance.signum();
    
    // Toward equilibrium when signs oppose
    if distance_sign * (trade_direction as i32) >= 0 {
        // Away from equilibrium
        AWAY_PENALTY_PCT
    } else {
        // Toward equilibrium - use reciprocal for division
        let distance = equilibrium_distance.unsigned_abs()
            .min(DISTANCE_SCALE_TICKS);
        let bonus = ((TOWARD_BONUS_PCT * distance * DISTANCE_SCALE_RECIPROCAL_Q16) >> 16);
        -(bonus as i32)
    }
}
```

### 5. Anti-Gaming via Impact Floor

**Behavior**: Every trade pays a minimum dynamic fee based on the impact floor, regardless of actual price movement. This makes splitting trades inherently unprofitable.

**Intuition**: By ensuring even tiny trades pay meaningful fees (e.g., 10 bps impact floor), we eliminate the economic incentive to split trades. A large trade that would pay 95 bps cannot be profitably split into smaller trades that each pay at least the floor.

**Rationale**: This approach is simple, robust, and requires no additional state. It cannot be gamed by timing trades across slot boundaries or using multiple wallets, and penalizes all small trades equally regardless of origin.

**Example**: With a 10 bps impact floor:
- Single 1000 token trade: 50 bps impact → 95 bps total fee
- Split into 10x 100 token trades: Each pays max(5 bps actual, 10 bps floor) = 10 bps impact → 55 bps per trade → 550 bps total
- The split is strongly unprofitable, achieving our anti-gaming goal with minimal complexity

### Phase 2 (Deferred): Warmup Ramp

**Behavior**: During the initial pool period, gradually enables rebates and equilibrium targeting based on both time elapsed and trading activity. Prevents early noisy data from creating incorrect incentives.

**Intuition**: New pools need time and sufficient trading activity to establish reliable price signals. Early trades might be exploratory or manipulative. The ramp requires both conditions to be met before enabling full dynamic behavior.

**Rationale**: Using both time AND volume prevents gaming. Time alone could be waited out with no trading. Volume alone could be gamed with wash trading. The combination ensures organic market development.

```rust
pub struct WarmupState {
    pub start_slot: u64,          // Deployment slot
    pub warmup_trade_count: u32,  // Trade counter, capped at MIN_WARMUP_TRADES
    pub ramp_q16: u16,            // 0..65535 ≈ [0,1] ramp progress
    pub r0_tick: i32,             // Bootstrap equilibrium: max(twap_seed, floor + buffer)
    pub done: bool,               // True when fully warmed
}

fn update_warmup_ramp_u32(
    warmup: &mut WarmupState,
    amount_in: u64,
    ticks_moved: u32,  // Now u32 for efficiency
    current_slot: u64,
) {
    // 1. Count trades (dust-filtered) using u32 math
    let min_trade_q = Q_PIVOT / 10;  // Dust filter threshold
    if amount_in >= min_trade_q && ticks_moved >= 1 {
        warmup.warmup_trade_count = warmup.warmup_trade_count
            .saturating_add(1)
            .min(MIN_WARMUP_TRADES);
    }
    
    // 2. Compute ramp using u32 arithmetic throughout
    let slots_elapsed = (current_slot.saturating_sub(warmup.start_slot)) as u32;
    let r_slots = ((slots_elapsed.min(S_MIN) * 65535) / S_MIN) as u16;
    let r_trades = ((warmup.warmup_trade_count * 65535) / MIN_WARMUP_TRADES) as u16;
    
    // Ramp is the MINIMUM of time and trade progress
    warmup.ramp_q16 = warmup.ramp_q16.max(r_slots.min(r_trades));
    warmup.done = warmup.ramp_q16 == 65535;
}

// Helper for smooth interpolation
fn lerp_ticks_q16(a: i32, b: i32, t_q16: u16) -> i32 {
    let t = t_q16 as i64;
    let delta = ((b as i64 - a as i64) * t) / 65535;
    (a as i64 + delta) as i32
}

// Cache momentum adjustment (single value per slot, direction-independent)
fn cache_momentum_adjustment(state: &mut PoolState, current_slot: u64) {
    state.momentum_slot = current_slot;
    
    // Calculate raw adjustment once per slot
    // This is the computational bottleneck we're optimizing
    state.cached_momentum_adjustment = calculate_momentum_adjustment(state.flow_ewma);
}
```

### 7. Fee Distribution

**Behavior**: After fees are collected from trades, they are distributed among multiple protocol entities using a configurable split. Governance may adjust the split globally through protocol parameters, but it remains fixed per trade for predictability.

**Intuition**: Different stakeholders contribute to the protocol's success in different ways. The distribution system ensures each entity is compensated appropriately for their role while maintaining incentive alignment.

**Rationale**: A configurable but fixed split keeps the system simple and predictable. Governance can tune splits using protocol parameters, but the mechanism remains simple and auditable.

The collected fees are split between the following entities:
- **The Feels Protocol**: Treasury accumulation for development and governance
- **The Pool's Buffer Account**: Maintains market stability and funds rebates
- **The Pool Creator**: Incentivizes quality pool creation and parameter tuning
- **LPs (Liquidity Providers)**: Rewards capital provision and impermanent loss risk
- **Swappers**: Via rebates for equilibrium-restoring trades

This split is configured via protocol parameters (with optional per‑pool override). Future phases may introduce adaptive policies, but the MVP uses a static configuration.

### 8. Understanding Swapper Rebates (Phase 2)

When advanced fees are enabled, the model can reward swappers for making trades that help stabilize the market. These rewards are delivered in the form of a **rebate**.

**Who gets the rebate?**
The **swapper** executing the trade.

**How does it work?**
A rebate is not a separate payment. It is an **immediate discount on the swap fee**.

1.  The system calculates the final `total_bps` fee, including any negative "bonus" adjustments for helpful trades.
2.  If the trade was helpful, this `total_bps` will be lower than the standard `BASE_FEE_BPS`.
3.  This reduced fee is applied to the swap, meaning the swapper receives **more output tokens** than they would have in a standard swap.

**When is a rebate given?**
A trade is considered "helpful" and eligible for a rebate if it:
-   **Moves the price toward equilibrium:** Pushes the current market price closer to its long-term average (the GTWAP).
-   **Counters market momentum:** For example, buying into a falling market or selling into a rising one.

The **Pool Buffer (τ)** is the accounting source for these rebates. When a rebate is given, the buffer and other fee recipients simply forgo the income they would have otherwise received, effectively using their potential revenue to pay for the swapper's discount.

## State Management with Unified Components (Phase 2)

**Behavior**: The dynamic fee system leverages unified protocol components for consistent state management across all subsystems.

**Intuition**: By sharing core infrastructure like pool::Oracle, pool::Floor, and FlowSignals, the fee system gains access to richer signals while reducing state duplication. The fee split includes LPs, PoolReserve, PoolBuffer, Protocol Treasury, and Creator fees for protocol tokens, with exact percentages governed by protocol params.

**Rationale**: Unified components ensure consistency, reduce gas costs, and enable more sophisticated fee calculations through shared signals.

```rust
// Pool-level state consumed by the fee model
pub struct PoolState {
    // References to unified components (stored as account keys)
    pub pool_oracle: Pubkey,      // pool::Oracle (GTWAP)
    pub pool_floor: Pubkey,       // pool::Floor (calc-only)
    pub safety_controller: Pubkey,
    pub flow_signals: Pubkey,
    
    // Fee-specific state
    pub flow_ewma: i64,
    pub flow_last_update: u64,
    
    // Momentum cache (per-slot, single adjustment value)
    pub momentum_slot: u64,
    pub cached_momentum_adjustment: i32,  // Raw adjustment, direction-independent
    
    // Warmup state
    pub warmup: WarmupState,
}

// Unified flow signals tracking shared with JIT
pub struct FlowSignals {
    pub flow_ewma: i64,              // From dynamic fees
    pub directional_toxicity: u16,    // From JIT
    pub combined_signal: u16,         // Weighted combination
    pub last_update_slot: u64,
}

impl FlowSignals {
    pub fn update_from_swap(&mut self, 
        swap_result: &SwapResult,
        jit_result: &JitResult,
        slot: u64
    ) {
        // Update flow EWMA from swap
        if let Some(signed_flow) = swap_result.signed_flow {
            self.flow_ewma = update_ewma(self.flow_ewma, signed_flow);
        }
        
        // Update directional toxicity from JIT
        if let Some(toxicity) = jit_result.observed_toxicity {
            self.directional_toxicity = update_ewma_u16(
                self.directional_toxicity, 
                toxicity
            );
        }
        
        // Combine signals with weighting
        self.combined_signal = (
            (self.flow_ewma.abs() as u32 * 7 + 
             self.directional_toxicity as u32 * 3) / 10
        ).min(u16::MAX) as u16;
        
        self.last_update_slot = slot;
    }
}

impl PoolState {
    pub fn update_after_swap(&mut self, 
        start_tick: i32,
        end_tick: i32,
        trade_direction: i8,
        amount_in: u64,
        current_slot: u64,
    ) {
        self.current_tick = end_tick;
        
        // Check GTWAP oracle maturity (optimization)
        if self.twap_last_update + MIN_TWAP_MATURITY_SLOTS > current_slot {
            // GTWAP not mature yet - disable GTWAP-dependent features
            self.twap_tick = self.current_tick;  // Fallback to current
        }
        
        // Update warmup ramp (using u32 math for efficiency)
        let ticks_moved = (end_tick - start_tick).unsigned_abs();
        update_warmup_ramp_u32(&mut self.warmup, amount_in, ticks_moved, current_slot);
        
        // Update flow EWMA
        let signed_amount = if trade_direction > 0 { 
            amount_in as i64 
        } else { 
            -(amount_in as i64) 
        };
        self.update_flow_ewma(signed_amount);
        self.flow_last_update = current_slot;
        
        // Cache momentum adjustment for this slot if first trade
        if self.momentum_slot != current_slot {
            cache_momentum_adjustment(self, current_slot);
        }
    }
}
```

## Parameters (Flat, Explicit for MVP)

For transparency and predictable governance, the MVP uses a flat set of explicit parameters:

```rust
pub struct FeeParamsMvp {
    pub base_fee_bps: u16,
    pub impact_floor_bps: u16,
    pub min_total_fee_bps: u16,
    pub max_total_fee_bps: u16,
    pub default_fee_cap_bps: u16,
}
```

These are modified directly via governance (no derived mappings). Phase 2 features (equilibrium, momentum, direction, warmup) introduce additional parameters behind feature flags when enabled.

## Integration with Swap (MVP)

**Behavior**: Shows how the fee model integrates with the existing swap flow. The swap executes first, then fees are calculated based on actual results and applied to the output.

**Intuition**: By calculating fees after execution, we ensure they're based on real market impact, not estimates. This also keeps the swap logic clean and separates concerns.

**Rationale**: This pattern minimizes changes to existing swap code, adds no overhead to the per-tick loop, and ensures fees exactly match the market stress actually created by each trade.

```rust
pub fn execute_swap(
    ctx: Context<Swap>,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit: u128,
    max_fee_bps: Option<u16>,  // Optional user-specified fee cap
) -> Result<()> {
    let start_tick = ctx.accounts.pool.current_tick;
    
    // Execute the CLMM swap (existing logic)
    let swap_result = concentrated_liquidity::swap(
        &mut ctx.accounts.pool,
        amount_in,
        sqrt_price_limit,
    )?;
    
    // Calculate fee based on realized impact (MVP)
    let fee_bps = calculate_fee_after_swap(start_tick, swap_result.end_tick);
    
    // Check against user's fee cap if provided
    if let Some(cap) = max_fee_bps {
        require!(
            fee_bps <= cap,
            ErrorCode::FeeExceedsCap
        );
    }
    
    // Apply fee to output
    // Note: Fee is calculated based on input metrics (ticks moved, amount_in for flow)
    // but applied as a percentage of output. For high-impact trades, this creates
    // a slight mismatch since output value < input value. This is a standard
    // approximation that prioritizes simplicity.
    let fee_amount = (swap_result.amount_out as u128 * fee_bps as u128) / 10_000;
    let final_amount_out = swap_result.amount_out - fee_amount as u64;
    
    require!(
        final_amount_out >= min_amount_out,
        ErrorCode::SlippageExceeded
    );
    
    // Unified post-swap update
    unified_post_swap_update(
        &mut ctx.accounts.pool,
        &swap_result,
        &jit_result,
        fee_bps,
        Clock::get()?.slot,
    )?;
    
    // Transfer tokens
    transfer_tokens_to_user(final_amount_out)?;
    transfer_fees_to_protocol(fee_amount)?;
    
    Ok(())
}

// Unified update function that coordinates all subsystems
pub fn unified_post_swap_update(
    pool: &mut Pool,
    swap_result: &SwapResult,
    jit_result: &Option<JitResult>,
    fee_bps: u16,
    slot: u64,
) -> Result<()> {
    // 1. Update pool oracle (GTWAP)
    pool.pool_oracle.update(
        slot,
        swap_result.end_tick,
        Clock::get()?.unix_timestamp,
    )?;
    
    // 2. Update unified flow signals
    if let Some(jit) = jit_result {
        pool.flow_signals.update_from_swap(
            swap_result,
            jit,
            slot,
        );
    }
    
    // 3. Update floor manager if needed
    if let Some(jitosol_change) = calculate_jitosol_appreciation() {
        pool.pool_floor.update_after_swap(jitosol_change, slot);
    }
    
    // 4. Update rate limiters
    pool.safety_controller.rate_limiter.record_swap(
        swap_result.amount_in,
        fee_bps,
        slot,
    )?;
    
    // 5. Update fee-specific state
    pool.state.update_after_swap(
        swap_result.start_tick,
        swap_result.end_tick,
        swap_result.trade_direction,
        swap_result.amount_in,
        slot,
    );
    
    Ok(())
}
```

### Recommended Fee Caps (Client Guidance)

- Default `max_fee_bps` for casual users: 120 bps (tune via governance/data).
- For sophisticated users/aggregators: set `max_fee_bps` to the observed 95th percentile fee over the last N trades for similar trade sizes, plus a small safety margin (e.g., +20 bps).
- The program emits the effective `fee_bps` via events; indexers can surface recommended caps per pool and trade size.

## Hybrid Model: Balancing UX with Protocol Objectives

While this fully on-chain model achieves robustness and anti-gaming properties, it creates UX challenges:
- Traders cannot know their exact fee until after execution
- This makes transaction planning difficult, especially for aggregators
- The uncertainty may discourage legitimate trading activity

To address this, we introduce a hybrid model with user-specified fee caps:

### User Fee Caps

**Behavior**: Traders can specify a maximum fee they're willing to pay. If the calculated fee exceeds this cap, the transaction reverts.

**Intuition**: This provides transaction certainty while maintaining the benefits of post-execution calculation. Sophisticated traders can set tight caps, while casual users can use higher caps for guaranteed execution.

**Rationale**: The cap acts as a safety valve. It prevents surprise fees while still allowing the protocol to charge appropriate fees based on actual market impact.

```rust
pub fn execute_swap_with_fee_cap(
    ctx: Context<Swap>,
    amount_in: u64,
    min_amount_out: u64,
    sqrt_price_limit: u128,
    max_fee_bps: u16,  // User-specified fee cap
) -> Result<()> {
    // Execute swap and calculate fee as before...
    let fee_bps = calculate_dynamic_fee_after_swap(...);
    
    // Check against user's fee cap
    require!(
        fee_bps <= max_fee_bps,
        ErrorCode::FeeExceedsCap
    );
    
    // Continue with fee application...
}
```

### Implementation Considerations

1. **Default Cap**: Set a reasonable default (e.g., 150 bps) for users who don't specify
2. **UI Guidance**: Show estimated fees based on recent trades to help users set appropriate caps
3. **Two-Tier System**: 
   - Casual users: Higher caps for guaranteed execution
   - Sophisticated traders: Tight caps for cost control

### Benefits of Hybrid Approach

1. **Maintains On-Chain Calculation**: All anti-gaming benefits preserved
2. **Provides Certainty**: Users know maximum possible cost
3. **Trader-Based Discovery**: Sophisticated traders will find optimal cap levels
4. **Graceful Degradation**: System works even if estimation is poor

### Why Revert Instead of Min()

The choice to revert when `fee_bps > max_fee_bps` rather than using `min(fee_bps, max_fee_bps)` is deliberate and critical for protocol health:

**Rationale**: While capping at the minimum would be more user-friendly short-term, it would allow gaming via artificially low caps. The revert approach forces honest engagement with the fee market and incentivizes accurate estimation.

**Benefits**:
- Preserves economic integrity by ensuring correct fees for market stress
- Creates ecosystem pressure for high-quality fee estimators
- Prevents systematic underpayment via low-cap gaming
- Ensures long-term protocol sustainability

## Key Improvements

1. **Fully On-Chain**: No client calculations or verification needed
2. **Actual Impact**: Uses realized tick movement, not estimates
3. **Anti-Gaming**: Impact floor prevents tiny trade spam and split trades
4. (Phase 2) Two-Part Equilibrium: Gentle upward bias with hard floor failsafe
5. **Clear Bounds**: Total fee always positive and capped
6. **Efficient**: O(1) calculations after swap completion
7. **User Control**: Fee caps provide transaction certainty
8. (Phase 2) Warmup Protection: Gradual enablement prevents early pool manipulation

## Testing Scenarios

### Scenario 1: Split Trade Prevention

**Setup**: Trader wants to buy 1000 tokens. Should they trade all at once or split into 10x 100 token trades?

**Behavior**: 
- Single trade: 50 bps impact → 95 bps total fee
- Split trades: Each would have 5 bps actual impact, but floor is 10 bps
- Each split trade: 10 bps impact → 55 bps total fee per trade
- Total for splits: 10 trades × 55 bps = 550 bps

**Result**: Splitting costs 550 bps vs 95 bps for single trade. Gaming is strongly discouraged.

### Scenario 2 (Phase 2): Arbitrage Incentive

**Setup**: Pool price is 2000 ticks above equilibrium. Arbitrageur can move price 500 ticks toward equilibrium.

**Behavior**:
- 500 tick move = 500 bps impact
- Direction bonus at max distance: -80% (multiplier = 20%)
- Dynamic fee: 500 × 0.2 = 100 bps
- Total: 30 + 100 = 130 bps

**Result**: Arbitrageur pays only 130 bps to capture 500 bps of price improvement, creating strong incentive to restore balance.

### Scenario 3 (Phase 2): Early Pool Protection

**Setup**: New pool launches. Attacker tries to manipulate fees via early trades.

**Behavior with warmup**:
- First few trades: ramp ≈ 0, equilibrium stays at bootstrap R0
- No volume after 15 min: ramp still ≈ 0 (needs both time AND volume)
- Rebates heavily scaled down until both conditions met
- Surcharges always apply fully (no gaming opportunity)

**Result**: Pool develops organically without early manipulation affecting long-term behavior.

## Tests

```rust
#[test]
fn test_no_split_advantage() {
    // Single large trade vs multiple splits
    let single_fee = calculate_fee_for_trade(1000_units);
    let split_fees: u16 = (0..10)
        .map(|_| calculate_fee_for_trade(100_units))
        .sum();
    assert!(split_fees >= single_fee, "Splits must not be cheaper");
}

#[test]
fn test_toward_away_symmetry() {
    // Same distance, opposite directions
    let toward_fee = calculate_fee_with_equilibrium(
        500_ticks_impact,
        -1000_equilibrium_distance,
        1_trade_direction  // toward
    );
    let away_fee = calculate_fee_with_equilibrium(
        500_ticks_impact,
        1000_equilibrium_distance,
        1_trade_direction  // away
    );
    assert!(toward_fee <= away_fee, "Toward must be cheaper or equal");
}

#[test]
fn test_momentum_cache_single_adjustment() {
    // Critical test: momentum cache stores single adjustment value
    let mut state = create_test_state();
    state.flow_ewma = 1_000_000;  // Strong positive momentum
    
    // First trade: buy (with momentum)
    let buy_fee = calculate_dynamic_fee_after_swap(
        0, 100, 1, 10000, &state, 1000
    );
    
    // Second trade in same slot: sell (counter momentum)
    let sell_fee = calculate_dynamic_fee_after_swap(
        100, 0, -1, 10000, &state, 1000  // Same slot
    );
    
    // Buy should pay more than sell when there's positive momentum
    assert!(buy_fee > sell_fee, "With-momentum trades must pay more");
    
    // Verify single cached adjustment value
    assert!(state.cached_momentum_adjustment > 0, "Should have positive adjustment");
    assert!(state.cached_momentum_adjustment <= MAX_MOMENTUM_ADJ as i32, "Should be bounded");
    
    // The same adjustment is used differently based on trade direction
    let with_factor = apply_momentum_direction(state.cached_momentum_adjustment, state.flow_ewma, 1);
    let counter_factor = apply_momentum_direction(state.cached_momentum_adjustment, state.flow_ewma, -1);
    assert_eq!(with_factor, 100 + state.cached_momentum_adjustment);
    assert_eq!(counter_factor, 100 - (state.cached_momentum_adjustment * 2 / 5));
}

#[test]
fn test_momentum_staleness() {
    // Stale flow should neutralize momentum
    let mut state = create_test_state();
    state.flow_ewma = 1_000_000;  // Strong momentum
    state.flow_last_update = current_slot - STALE_SLOTS - 1;  // But stale
    
    let fee = calculate_dynamic_fee_after_swap(
        0, 100, 1, 1000, &state, current_slot
    );
    
    // Should only have direction component, no momentum boost
    // The momentum_pct should be neutral (100) due to staleness
    // This is handled in calculate_dynamic_fee_after_swap, not in caching
}

#[test]
fn test_warmup_requirements() {
    // (a) Time alone insufficient
    let mut warmup = WarmupState::new();
    advance_slots(&mut warmup, S_MIN + 100);
    assert!(warmup.ramp_q16 < 65535, "Time alone should not complete warmup");
    
    // (b) Trades alone insufficient
    let mut warmup = WarmupState::new();
    execute_trades(&mut warmup, MIN_WARMUP_TRADES + 10);
    advance_slots(&mut warmup, 100);  // Not enough time
    assert!(warmup.ramp_q16 < 65535, "Trades alone should not complete warmup");
    
    // (c) Both sufficient
    let mut warmup = WarmupState::new();
    execute_trades(&mut warmup, MIN_WARMUP_TRADES);
    advance_slots(&mut warmup, S_MIN);
    assert_eq!(warmup.ramp_q16, 65535, "Both conditions should complete warmup");
}

#[test]
fn test_fee_bounds_fuzz() {
    // Fuzz test all inputs
    for _ in 0..10000 {
        let fee = calculate_dynamic_fee_after_swap(
            random_tick(),
            random_tick(),
            random_direction(),
            random_amount(),
            &random_state(),
            random_slot(),
        );
        assert!(fee >= MIN_TOTAL_FEE_BPS);
        assert!(fee <= MAX_TOTAL_FEE_BPS);
    }
}
```

## Summary

This hybrid design achieves the best of both worlds:
- Single pass fee calculation after swap maintains integrity
- No trust assumptions or client verification required
- Robust against gaming via splits and tiny trades
- Natural incentives for balance restoration through rebates
- Minimal computational overhead with O(1) operations
- Clear, bounded behavior with user control via fee caps
- Transaction certainty for better UX without sacrificing security

The thermodynamic interpretation remains valid: price impact represents dissipation through market resistance, momentum modulates turbulence effects, and equilibrium restoration credits useful work. The fee cap acts as a pressure release valve, allowing the system to maintain its natural dynamics while preventing excessive friction that could inhibit healthy trading activity.

## Forward-Looking Operational Considerations

With the on-chain architecture finalized, critical challenges shift to operational excellence. Two key areas require immediate focus:

### 1. The Critical Off-Chain Fee Estimator

The hybrid model's success depends entirely on accurate client-side fee estimation. Poor estimation leads to failed transactions and user frustration.

**Requirements for the Reference Estimator**:
- **Perfect Logic Mirror**: Must exactly replicate `calculate_dynamic_fee_after_swap`
- **Fast State Access**: Requires reliable RPC nodes for latest `PoolState`
- **Advanced Features**: 
  - Simulate trades against mempool for pending state
  - Account for concurrent trades affecting price impact
  - Provide confidence intervals for estimates

**Implementation Priority**: This estimator should be treated as core protocol infrastructure, not an afterthought. The protocol's UX reputation depends on it.

### 2. Data-Driven Governance Framework

With a dozen governable parameters, the protocol needs sophisticated tooling for informed decision-making.

**Essential Components**:

1. **Public Analytics Dashboards**:
   - Average price impact vs. impact floor utilization
   - Distribution of momentum and direction adjustments
   - Soft bias vs. hard floor activation frequency
   - Fee statistics by trade size and trader type

2. **Digital Twin Simulation Environment**:
   - Replay historical trades against proposed parameter changes
   - Show concrete impact on fees and trader behavior
   - Transform governance from guesswork to data-driven decisions

3. **Phased Parameter Evolution**:
   - Launch with conservative values to encourage adoption
   - Use real data to gradually optimize parameters
   - Example progression:
     - Launch: `IMPACT_FLOOR_BPS = 8`
     - Month 2: Increase to 10 based on split trade analysis
     - Month 3: Tune momentum parameters based on volatility data

**Governance Best Practices**:
- Every parameter change proposal must include simulation results
- Establish clear KPIs: fee revenue, trading volume, price stability
- Regular parameter review cycles based on market conditions
- Emergency parameter adjustment mechanism for black swan events

The protocol's long-term success depends not just on the elegance of its design, but on the sophistication of its operational tooling and governance processes.

## Deployment Phases

The dynamic fee system deployment follows the protocol-wide rollout stages:

### Stage 1: Foundation (Weeks 1-2)
- Deploy unified infrastructure (pool::Oracle, pool::Floor, SafetyController, FlowSignals)
- Launch with conservative fee parameters
- Base fee: 30 bps, Impact floor: 15 bps
- Momentum and direction adjustments disabled

### Stage 2: Activation (Weeks 3-4)
- Enable momentum tracking with conservative bounds
- Enable direction adjustments with small bonuses
- Begin collecting flow signal data

### Stage 3: Optimization (Weeks 5-8)
- Tune parameters based on observed trading patterns
- Gradually reduce impact floor if split trading is minimal
- Increase rebate potential for equilibrium-restoring trades

### Stage 4: Maturity (Week 8+)
- Full dynamic fee model with all features active
- Data-driven parameter adjustments via governance
- Integration with advanced features (e.g., fee tiers by trader type)
### Phase 2 Note: Equilibrium & Rebates

Equilibrium targets and rebates are disabled in MVP. When enabled later, stale GTWAP windows will fall back to `max(current_tick, pool_floor.get_safe_ask_tick())` and disable rebates automatically.
