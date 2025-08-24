# Concentrated Liquidity Algorithm Documentation

## Overview

This document provides documentation for the concentrated liquidity swap algorithm implemented in the Feels Protocol. The algorithm is based on Uniswap V3's design with Solana-specific optimizations.

## Core Concepts

### 1. Price Representation

The protocol uses **square root price** representation for efficiency:
- **Format**: Q96 fixed-point for external APIs (Uniswap V3 compatible)
- **Internal**: Q64 fixed-point for calculations (Orca Whirlpools compatible)
- **Formula**: `sqrt_price_x96 = sqrt(token1_amount / token0_amount) * 2^96`
- **Benefits**: Avoids expensive division operations during swaps

```rust
// Example: FeelsSOL/FEELS at $2000
// price = 2000 FEELS/FeelsSOL
// sqrt_price_x96 = sqrt(2000) * 2^96 ≈ 3.54e30
```

### 2. Tick System

Ticks represent discrete price points:
- **Tick spacing**: Varies by fee tier (1, 10, 60, 200)
- **Price formula**: `price = 1.0001^tick`
- **Implementation Range**: -443636 to +443636 (practical limits)
- **Theoretical Range**: -887272 to +887272

```rust
// Tick to price conversion
fn tick_to_price(tick: i32) -> f64 {
    1.0001_f64.powi(tick)
}

// Price to tick conversion (logarithmic)
fn price_to_tick(price: f64) -> i32 {
    (price.ln() / 1.0001_f64.ln()).floor() as i32
}
```

### 3. Liquidity Depth

Liquidity is distributed across price ranges:
- **Virtual liquidity**: `L = sqrt(x * y)`
- **Real reserves**: Calculated based on current price tick position

## Swap Algorithm Deep Dive

### Phase 1: Initialization

```rust
fn execute_concentrated_liquidity_swap(
    swap_state: &mut SwapState,
    pool: &mut Pool,
    sqrt_price_limit: u128,
    zero_for_one: bool,  // true = token0 -> token1
    remaining_accounts: &[AccountInfo],
) -> Result<u64>
```

**Key Parameters:**
- `swap_state`: Tracks remaining input, current price, and active liquidity
- `sqrt_price_limit`: Maximum price movement allowed (slippage protection)
- `zero_for_one`: Swap direction indicator

### Phase 2: Main Swap Loop

The algorithm iterates through price ranges until:
1. All input is consumed, OR
2. Price limit is reached

```rust
while swap_state.amount_remaining > 0 && swap_state.sqrt_price != sqrt_price_limit {
    // Step 1: Compute swap within current tick range
    let step = compute_swap_step(...)?;
    
    // Step 2: Update state
    swap_state.sqrt_price = step.sqrt_price_next;
    swap_state.amount_remaining -= step.amount_in;
    swap_state.amount_calculated += step.amount_out;
    
    // Step 3: Update global fee growth
    if swap_state.liquidity > 0 {
        update_fee_growth(pool, step.fee_amount, swap_state.liquidity)?;
    }
    
    // Step 4: Cross tick if necessary
    if step.sqrt_price_next == step.sqrt_price_target {
        cross_tick(pool, swap_state, step.tick_next, zero_for_one)?;
    }
}
```

### Phase 3: Swap Step Calculation

Each step computes the swap within a single tick range:

```rust
fn compute_swap_step(
    sqrt_price_current: u128,
    sqrt_price_target: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u16,
    zero_for_one: bool,
) -> Result<SwapStep>
```

**Algorithm:**
1. **Calculate maximum price movement** given available liquidity
2. **Determine actual price target** (min of calculated and limit)
3. **Calculate token amounts** based on price movement
4. **Apply fees** to input amount

**Mathematical Formulas:**

For token0 -> token1 swaps (zero_for_one = true):
```
Δx = L * (1/√P_new - 1/√P_current)  // Amount token0 in
Δy = L * (√P_new - √P_current)      // Amount token1 out
```

For token1 -> token0 swaps (zero_for_one = false):
```
Δx = L * (1/√P_current - 1/√P_new)  // Amount token0 out
Δy = L * (√P_current - √P_new)      // Amount token1 in
```

**Note**: External API uses Q96 fixed-point arithmetic (2^96 scaling), while internal calculations use Q64 (2^64 scaling) for efficiency on Solana.

### Phase 4: Tick Crossing

When price moves across tick boundaries:

```rust
fn cross_tick(
    pool: &mut Pool,
    swap_state: &mut SwapState,
    tick_index: i32,
    zero_for_one: bool,
) -> Result<()>
```

**Process:**
1. **Load tick data** from tick array account
2. **Update active liquidity**: Add/subtract `liquidity_net`
3. **Update tick state**: Mark as initialized if needed
4. **Track fee growth**: Record fees earned outside tick

### Phase 5: Fee Handling

Fees are handled in multiple layers:

1. **Swap fees**: Deducted from input amount
   ```rust
   fee_amount = (amount_in * fee_rate) / 10000
   ```

2. **Protocol fees**: Percentage of swap fees
   ```rust
   protocol_fee = (fee_amount * protocol_fee_rate) / 10000
   ```

3. **Fee growth tracking**: For liquidity providers
   ```rust
   fee_growth_delta = (fee_amount << 128) / liquidity
   ```

## Gas Optimizations

### 1. Tick Array Loading
- Arrays passed via `remaining_accounts`
- Only loaded when needed
- Pre-calculated off-chain

### 2. Batched Updates
- Accumulate tick updates
- Apply in single transaction
- Use TransientTickUpdates

### 3. Zero-Copy Deserialization
- Direct memory access
- No allocation overhead
- Fixed-size structures

## Safety Mechanisms

### 1. Price Bounds
```rust
const MIN_SQRT_PRICE_X96: u128 = 18447090763469684736;  // sqrt(1.0001^-443636) * 2^96
const MAX_SQRT_PRICE_X96: u128 = 340_275_971_719_517_849_884_101_479_037_289_023_427;  // sqrt(1.0001^443636) * 2^96
```

### 2. Overflow Protection
- All arithmetic uses native Rust checked operations (checked_add, checked_sub, etc.)
- U256/U512 from ruint library for large number operations
- Fee growth tracked in Q128.128 format using 256-bit integers

### 3. Slippage Protection
- User specifies `sqrt_price_limit`
- Enforced throughout swap
- Reverts if breached

## Example: Complete Swap Flow

```rust
// User wants to swap 1000 FEELS for FeelsSOL
// Current price: 2000 FEELS/FeelsSOL (sqrt_price ≈ 8.26e20)

// 1. Initialize swap state
let mut swap_state = SwapState {
    amount_remaining: 1000_000_000,  // 1000 FEELS (6 decimals)
    amount_calculated: 0,
    sqrt_price: 3_543_191_142_285_914_205_922_034_323_338_780,  // sqrt(2000) * 2^96
    tick: 69081,  // Current tick
    fee_amount: 0,
    liquidity: 50_000_000_000_000u128,  // Active liquidity
};

// 2. Calculate fee (0.3%)
let fee = 1000_000_000 * 30 / 10000;  // 3,000,000 = 3 FEELS
swap_state.amount_remaining = 997_000_000;  // After fee

// 3. Compute swap step
// Price moves slightly due to liquidity depth
// Output: ~0.498 FeelsSOL

// 4. Update pool state
pool.current_sqrt_price = 3_541_000_000_000_000_000_000_000_000_000_000;  // New price after swap
pool.current_tick = 69060;

// 5. Transfer tokens
// User sends: 1000 FEELS
// User receives: 0.498 FeelsSOL
```

## Common Pitfalls and Solutions

### 1. Rounding Errors
- **Problem**: Cumulative rounding in multi-step swaps
- **Solution**: Track exact amounts, round only on final output

### 2. Liquidity Gaps
- **Problem**: No liquidity in price range
- **Solution**: Revert with InsufficientLiquidity error

### 3. Tick Array Boundaries
- **Problem**: Swap spans multiple arrays
- **Solution**: Pre-load all required arrays

### 4. Price Impact
- **Problem**: Large swaps move price significantly
- **Solution**: Use price limit and consider splitting trades

## Testing Considerations

1. **Edge Cases**:
   - Swaps at price boundaries
   - Zero liquidity ranges
   - Maximum/minimum prices
   - Single tick swaps

2. **Precision Tests**:
   - Compare outputs with reference implementation
   - Verify fee calculations
   - Check rounding consistency

3. **Gas Benchmarks**:
   - Measure CU usage for various swap sizes
   - Compare single vs multi-hop swaps
   - Profile tick array loading

## Future Optimizations (Phase 2+)

1. **Dynamic fees**: Adjust based on volatility
2. **MEV protection**: Just-in-time liquidity defenses  
3. **Oracle integration**: TWAP for price feeds
4. **Cross-program composability**: Flash swaps

This algorithm forms the core of the Feels Protocol's efficiency and capital optimization, enabling traders to access deep liquidity with minimal slippage while providing LPs with concentrated exposure to their desired price ranges.