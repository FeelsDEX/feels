# 3D Tick Invariant System Specification

## Overview

The Feels Protocol implements a revolutionary 3D automated market maker (AMM) that extends traditional 2D liquidity concentration to three dimensions: **Rate** (price/interest), **Duration** (time commitment), and **Leverage** (risk multiplier). This creates a unified trading surface where spot, lending, and derivatives markets converge.

## Mathematical Summary

The 3D system is governed by four key mathematical principles:

1. **Invariant Conservation**: $K = R^{w_r} \times D^{w_d} \times L^{w_l} = \text{constant}$
2. **Rebase Constraint**: $\prod_k g_k^{w_k} = 1$ for all value transfers
3. **Position Value**: $V(t) = V_0 \times \prod_d \frac{I_d(t)}{I_d(t_0)}$ where $d \in \{r, d, l\}$
4. **Autocompounding**: $I(t) = I(0) \times e^{\int_0^t r(s) ds}$ for continuous yield accrual

## Mathematical Foundation

### Core Invariant

The protocol maintains a weighted geometric mean invariant across all three dimensions:

$$K = R^{w_r} \times D^{w_d} \times L^{w_l} = \text{constant}$$

Where:
- $R$ = Rate dimension (price or interest rate)
- $D$ = Duration dimension (time commitment)
- $L$ = Leverage dimension (risk multiplier)
- $w_r, w_d, w_l$ = Dimension weights (sum to 1)

### Virtual Rebasing Constraint

All yield and funding flows preserve the invariant through multiplicative rebasing:

$$\prod_k g_k^{w_k} = 1$$

This ensures that value transfers between participants don't change the fundamental market geometry.

## 3D Tick Space

### Tick Representation

Each tick in the 3D space is represented as:

```rust
pub struct Tick3D {
    /// Rate dimension tick (price/interest)
    pub rate_tick: i32,
    
    /// Duration dimension tick (time blocks)
    pub duration_tick: i32,
    
    /// Leverage dimension tick (risk level)
    pub leverage_tick: i32,
    
    /// Combined tick index for efficient lookup
    pub index: u64,
}
```

### Tick Spacing

Different dimensions use different tick spacings for optimal granularity:

- **Rate**: 1 basis point (0.01%) per tick
- **Duration**: Exponential spacing (Flash, Swap, Weekly, Monthly, Quarterly, Annual)
- **Leverage**: 0.1x per tick (1x to 10x range)

### Index Calculation

The combined tick index for efficient storage and retrieval:

$$\text{index} = (\text{rate\_tick} + \text{MAX\_TICK}) \times \text{DURATION\_LEVELS} \times \text{LEVERAGE\_LEVELS} + \text{duration\_tick} \times \text{LEVERAGE\_LEVELS} + \text{leverage\_tick}$$

## Liquidity Concentration

### 3D Liquidity Distribution

Liquidity in the 3D system is distributed across a cube rather than a range:

```rust
pub struct Liquidity3D {
    /// Lower bounds for each dimension
    pub lower_bound: Tick3D,
    
    /// Upper bounds for each dimension
    pub upper_bound: Tick3D,
    
    /// Concentrated liquidity amount
    pub liquidity: u128,
    
    /// Effective liquidity with leverage
    pub effective_liquidity: u128,
}
```

### Effective Liquidity Calculation

Leverage multiplies the effective liquidity available:

$$\text{effective\_liquidity} = \text{base\_liquidity} \times \text{leverage\_factor} \times \text{duration\_multiplier}$$

Where:
- $\text{leverage\_factor}$ = Leverage setting (1x to 10x)
- $\text{duration\_multiplier}$ = Time commitment bonus (1x for Flash, up to 2x for Annual)

## Order Execution

### 3D Path Finding

Orders traverse the 3D space to find optimal execution:

1. **Rate Movement**: Traditional price discovery
2. **Duration Shift**: Time arbitrage opportunities
3. **Leverage Adjustment**: Risk-based routing

### Order Types

```rust
pub enum OrderType3D {
    /// Spot swap (rate dimension only)
    Spot {
        rate_limit: i32,
    },
    
    /// Term loan (rate + duration)
    Loan {
        rate_limit: i32,
        duration: Duration,
    },
    
    /// Leveraged position (all dimensions)
    Leveraged {
        rate_limit: i32,
        duration: Duration,
        leverage: u64,
    },
}
```

### Execution Algorithm

```python
def execute_3d_order(amount, start_tick, target_tick):
    path = find_optimal_path(start_tick, target_tick)
    remaining = amount
    
    for step in path:
        # Get liquidity at current tick
        liquidity = get_tick_liquidity(step.tick)
        
        # Apply dimension weights: L_weighted = L * w_dim
        weighted_liquidity = apply_weights(liquidity, step.dimension)
        
        # Execute partial fill
        filled = min(remaining, weighted_liquidity)
        remaining -= filled
        
        # Update tick state
        update_tick(step.tick, -filled)
        
        if remaining == 0:
            break
    
    return amount - remaining
```

## Dimension Interactions

### Rate-Duration Coupling

Interest rates naturally couple with duration:

$$\text{implied\_rate} = \text{base\_rate} \times (1 + \text{duration\_premium})$$

Longer duration commitments command higher rates, creating a yield curve.

### Leverage-Duration Risk

Leverage risk increases with duration:

$$\text{risk\_factor} = \text{leverage} \times \sqrt{\frac{\text{duration\_blocks}}{\text{BASE\_DURATION}}}$$

This creates natural risk premiums for leveraged long-term positions.

### Cross-Dimensional Arbitrage

The system allows arbitrage across dimensions:

1. **Carry Trade**: Borrow short duration, lend long duration
2. **Risk Arbitrage**: Low leverage long vs high leverage short
3. **Time Arbitrage**: Duration rolling strategies

## Fee Structure

### 3D Fee Calculation

Fees vary by dimension traversal:

$$\text{fee}_{total} = \text{fee}_{rate} + \text{fee}_{duration} + \text{fee}_{leverage}$$

Where:
- $\text{fee}_{rate} = \text{base\_fee} \times \frac{|\Delta r|}{\text{RATE\_SCALE}}$
- $\text{fee}_{duration} = \text{base\_fee} \times \frac{w_d}{100}$
- $\text{fee}_{leverage} = \text{base\_fee} \times \frac{(\Delta l)^2}{\text{LEVERAGE\_SCALE}}$

```rust
pub fn calculate_3d_fee(
    rate_delta: i32,
    duration_delta: i32,
    leverage_delta: i32,
) -> u64 {
    let base_fee = pool.fee_rate;
    
    // Rate dimension fee (traditional swap fee)
    // fee_r = base_fee × |Δr| / RATE_SCALE
    let rate_fee = base_fee * abs(rate_delta) / RATE_SCALE;
    
    // Duration dimension fee (term structure premium)
    // fee_d = base_fee × w_d / 100
    let duration_fee = base_fee * DURATION_WEIGHTS[duration_delta] / 100;
    
    // Leverage dimension fee (risk premium)
    // fee_l = base_fee × Δl² / LEVERAGE_SCALE
    let leverage_fee = base_fee * leverage_delta * leverage_delta / LEVERAGE_SCALE;
    
    rate_fee + duration_fee + leverage_fee
}
```

### Dynamic Fee Adjustment

Fees adjust based on:
- **Volatility**: Higher volatility increases all dimension fees
- **Utilization**: High utilization increases duration fees
- **Imbalance**: Leverage imbalance creates funding fees

## Virtual Rebasing Integration

### Yield Distribution

Yield accrues continuously through virtual rebasing:

```rust
pub struct RebaseIndex3D {
    /// Rate dimension yield index
    pub rate_index: u128,
    
    /// Duration dimension bonus index
    pub duration_index: u128,
    
    /// Leverage funding index
    pub leverage_index: u128,
}
```

### Lazy Evaluation

Positions calculate current value on-demand using multiplicative rebasing:

$$\text{value}_{current} = \text{value}_{base} \times \frac{\text{index}_{rate,current}}{\text{index}_{rate,checkpoint}} \times \frac{\text{index}_{duration,current}}{\text{index}_{duration,checkpoint}} \times \frac{\text{index}_{leverage,current}}{\text{index}_{leverage,checkpoint}}$$

For short positions, the leverage index is inverted:

$$\text{value}_{short} = \text{value}_{base} \times \frac{\text{index}_{rate,current}}{\text{index}_{rate,checkpoint}} \times \frac{\text{index}_{duration,current}}{\text{index}_{duration,checkpoint}} \times \frac{\text{index}_{leverage,checkpoint}}{\text{index}_{leverage,current}}$$

### Autocompounding

The system provides automatic compounding without any user action or gas costs. The rebase indices grow exponentially over time:

$$I(t) = I(t_0) \times e^{r \cdot (t - t_0)}$$

This is approximated discretely as:

$$I_{n+1} = I_n \times (1 + r \cdot \Delta t)$$

Since position values are calculated as:

$$V(t) = V_0 \times \frac{I(t)}{I(t_0)}$$

The yield automatically compounds. For example, with a 10% APY:
- Year 1: $V_1 = V_0 \times 1.10$
- Year 2: $V_2 = V_0 \times 1.10^2 = V_0 \times 1.21$
- Year n: $V_n = V_0 \times 1.10^n$

This matches the continuous compounding formula:
$$V(t) = V_0 \times e^{r \cdot t}$$

```rust
fn get_position_value(position: &Position3D, indices: &RebaseIndex3D) -> (u64, u64) {
    let base_value = position.liquidity;
    
    // Apply rate dimension yield
    let rate_adjusted = base_value * indices.rate_index / position.rate_checkpoint;
    
    // Apply duration bonus
    let duration_adjusted = rate_adjusted * indices.duration_index / position.duration_checkpoint;
    
    // Apply leverage funding
    let final_value = if position.is_long {
        duration_adjusted * indices.leverage_index / position.leverage_checkpoint
    } else {
        duration_adjusted * position.leverage_checkpoint / indices.leverage_index
    };
    
    final_value
}
```

## Risk Management

### Leverage Limits

Dynamic leverage limits based on market conditions:

```rust
pub fn calculate_max_leverage(
    volatility: u64,
    duration: Duration,
    pool_utilization: u64,
) -> u64 {
    let base_max = 10_000_000; // 10x
    
    // Reduce for high volatility
    let volatility_factor = min(100, 10000 / (volatility + 100));
    
    // Reduce for long duration
    let duration_factor = match duration {
        Duration::Flash => 100,
        Duration::Swap => 100,
        Duration::Weekly => 80,
        Duration::Monthly => 60,
        Duration::Quarterly => 40,
        Duration::Annual => 20,
    };
    
    // Reduce for high utilization
    let utilization_factor = 100 - (pool_utilization / 100);
    
    base_max * volatility_factor * duration_factor * utilization_factor / 1_000_000
}
```

### Redenomination

During extreme market stress, positions redenominate proportionally based on risk:

$$\text{loss}_{position} = \text{market\_loss} \times \frac{\text{risk\_weight}_{position} \times \text{value}_{position}}{\sum_i \text{risk\_weight}_i \times \text{value}_i}$$

Where the risk weight is:
$$\text{risk\_weight} = \text{leverage} \times \text{duration\_multiplier}$$

```rust
pub fn redenominate_3d(
    position: &mut Position3D,
    market_loss: u64,
    total_risk_weighted_value: u128,
) -> u64 {
    // Calculate position's risk weight
    let risk_weight = position.leverage * position.duration.risk_multiplier();
    let position_risk_value = position.value * risk_weight;
    
    // Proportional loss allocation
    let position_loss = market_loss * position_risk_value / total_risk_weighted_value;
    
    // Apply loss with protection curve
    let protected_loss = apply_protection_curve(position_loss, position.leverage);
    
    position.value -= protected_loss;
    protected_loss
}
```

## Implementation Architecture

### Account Structure

```rust
// Core 3D pool state
pub struct Pool3D {
    // Current position in 3D space
    pub current_tick_3d: Tick3D,
    
    // Liquidity at current tick
    pub liquidity: u128,
    
    // Dimension weights
    pub weights: DimensionWeights,
    
    // Fee configuration
    pub fee_config: FeeConfig3D,
    
    // Virtual rebasing accumulator
    pub rebase_accumulator: Pubkey,
}

// 3D Tick array for liquidity
pub struct TickArray3D {
    // Start position in 3D space
    pub start_tick: Tick3D,
    
    // Flattened array of ticks
    pub ticks: [Tick3DData; ARRAY_SIZE_3D],
    
    // Bitmap for initialized ticks
    pub initialized_bitmap: u128,
}

// Position with 3D parameters
pub struct Position3D {
    // Position bounds in 3D space
    pub lower_tick: Tick3D,
    pub upper_tick: Tick3D,
    
    // Liquidity and value
    pub liquidity: u128,
    pub value: u64,
    
    // Rebase checkpoints
    pub rebase_checkpoint: RebaseCheckpoint3D,
}
```

### Instruction Flow

1. **Order Placement**
   ```rust
   place_3d_order(
       amount: u64,
       start_dimension: Dimension,
       target_tick: Tick3D,
       max_slippage: u64,
   )
   ```

2. **Liquidity Provision**
   ```rust
   add_3d_liquidity(
       lower_bounds: Tick3D,
       upper_bounds: Tick3D,
       amounts: (u64, u64),
       duration_commitment: Duration,
       leverage_enabled: bool,
   )
   ```

3. **Yield Collection**
   ```rust
   collect_3d_yield(
       position: Pubkey,
       include_fees: bool,
       include_funding: bool,
       include_duration_bonus: bool,
   )
   ```

## Advantages

### Capital Efficiency
- Single pool serves spot, lending, and derivatives
- Liquidity shared across all dimensions
- No fragmentation between markets
- Automatic compounding without gas costs

### Price Discovery
- Unified price across all timeframes
- Natural yield curve emergence
- Integrated funding rates

### Risk Management
- Automatic hedging through dimension coupling
- No liquidations through redenomination
- Smooth risk transitions

### Autocompounding Benefits
Unlike traditional staking systems that require manual claiming and restaking:
- **Zero Gas Costs**: Compounding happens through index updates, not transactions
- **Continuous Accrual**: Yield compounds every block, not just on claim
- **No Loss of Yield**: Users never miss compounding opportunities
- **Simplified UX**: No need to understand or execute compounding strategies

The autocompounding mechanism ensures that:
$$\text{APY} = (1 + \frac{\text{APR}}{n})^n - 1 \approx e^{\text{APR}} - 1$$

Where $n \to \infty$ for continuous compounding, maximizing returns for liquidity providers.
