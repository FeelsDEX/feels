# Concentrated Liquidity Algorithm Documentation

## Overview

This document provides documentation for the concentrated liquidity swap algorithm implemented in the Feels Protocol. The algorithm is based on Uniswap V3's design with Solana-specific optimizations.

## Core Concepts

### 1. Price Representation

The protocol uses **square root price** representation for efficiency:
- **Format**: Q64.96 fixed-point (96 fractional bits) for Uniswap V3 compatibility
- **Internal Conversions**: Some calculations use Q64 for efficiency while maintaining precision
- **Formula**: `sqrt_price_x96 = sqrt(token_b_amount / token_a_amount) * 2^96`
- **Benefits**: Avoids expensive division operations during swaps

```rust
// Example: Token A/Token B at price 2000
// price = 2000 Token B per Token A
// sqrt_price_x96 = sqrt(2000) * 2^96 ≈ 3.54e30
```

### 2. Tick System

Ticks represent discrete price points:
- **Tick spacing**: Varies by fee tier (1, 10, 60, 200)
- **Price formula**: `price = 1.0001^tick`
- **Implementation Range**: -443,636 to +443,636 (enforced bounds)
- **Constants**: MIN_SQRT_PRICE_X96 = 4,295,048,017, MAX_SQRT_PRICE_X96 = 79,226,673,521,066,979,257,578,248,243

```rust
// Get sqrt price from tick (actual implementation)
TickMath::get_sqrt_ratio_at_tick(tick: i32) -> Result<u128>

// Get tick from sqrt price (actual implementation)  
TickMath::get_tick_at_sqrt_ratio(sqrt_price_x96: u128) -> Result<i32>
```

### 3. Liquidity Depth

Liquidity is distributed across price ranges:
- **Virtual liquidity**: `L = sqrt(x * y)`
- **Real reserves**: Calculated based on current price tick position

## Swap Algorithm Deep Dive

### Step 1: Initialization

```rust
fn execute_concentrated_liquidity_swap<'info>(
    swap_state: &mut SwapState,
    pool: &mut Pool,
    sqrt_price_limit: u128,
    zero_for_one: bool,  // true = token_a -> token_b
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<u64>
```

**Key Parameters:**
- `swap_state`: Tracks remaining input, current price, and active liquidity
- `sqrt_price_limit`: Maximum price movement allowed (slippage protection)
- `zero_for_one`: Swap direction indicator (true = sell token A for token B)

### Step 2: Main Swap Loop

The algorithm iterates through price ranges until:
1. All input is consumed, OR
2. Price limit is reached

```rust
while should_continue_swap(swap_state, sqrt_price_limit_adjusted) {
    // Step 1: Compute swap within current tick range
    let step = compute_swap_step(
        swap_state.sqrt_price,
        sqrt_price_limit_adjusted,
        swap_state.liquidity,
        swap_state.amount_remaining,
        pool.fee_rate,
        zero_for_one,
    )?;
    
    // Step 2: Update state
    apply_swap_step(swap_state, &step)?;
    
    // Step 3: Update global fee growth
    update_fee_growth(pool, swap_state.liquidity, step.fee_amount, zero_for_one)?;
    
    // Step 4: Handle tick crossing if we hit a boundary
    handle_tick_crossing(pool, swap_state, &step, zero_for_one, remaining_accounts)?;
}
```

### Step 3: Swap Step Calculation

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

For token A -> token B swaps (zero_for_one = true):
```
Δx = L * (1/√P_new - 1/√P_current)  // Amount token A in
Δy = L * (√P_new - √P_current)      // Amount token B out
```

For token B -> token A swaps (zero_for_one = false):
```
Δx = L * (1/√P_current - 1/√P_new)  // Amount token A out
Δy = L * (√P_current - √P_new)      // Amount token B in
```

**Implementation Details**:
- Uses `ConcentratedLiquidityMath::get_next_sqrt_price_from_input` for exact input swaps
- Internally uses helper functions from `utils::math_amm` for amount calculations
- All calculations use checked arithmetic to prevent overflows

### Step 4: Tick Crossing

When price moves across tick boundaries:

```rust
fn cross_tick<'info>(
    pool: &mut Pool,
    swap_state: &mut SwapState,
    tick_index: i32,
    zero_for_one: bool,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()>
```

**Process:**
1. **Validate tick array accounts** - Check program ownership and data length
2. **Find the correct tick array** containing the target tick
3. **Update active liquidity**: Add/subtract `liquidity_net` based on direction
4. **Sync all pool state immediately** to maintain consistency

### Step 5: Fee Handling

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
   fee_growth_delta = FeeGrowthMath::fee_to_fee_growth(fee_amount, liquidity)
   // Uses 256-bit integers represented as [u64; 4] arrays
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
const MIN_SQRT_PRICE_X96: u128 = 4_295_048_017;  // sqrt(1.0001^-443636) * 2^96
const MAX_SQRT_PRICE_X96: u128 = 79_226_673_521_066_979_257_578_248_243;  // sqrt(1.0001^443636) * 2^96
```

### 2. Overflow Protection
- All arithmetic uses native Rust checked operations (checked_add, checked_sub, etc.)
- Custom 256-bit math for fee growth calculations using [u64; 4] arrays
- Fee growth tracked with 128 bits of precision for accurate distribution

### 3. Slippage Protection
- User specifies `sqrt_price_limit`
- Enforced throughout swap
- Reverts if breached

## Example: Complete Swap Flow

```rust
// User wants to swap 1000 token A for token B
// Current price: 2000 token B per token A

// 1. Initialize swap state
let mut swap_state = SwapState {
    amount_remaining: 1_000_000_000,  // 1000 tokens (6 decimals)
    amount_calculated: 0,
    sqrt_price: pool.current_sqrt_price,
    tick: pool.current_tick,
    fee_amount: 0,
    liquidity: pool.liquidity,
};

// 2. Calculate fee breakdown
let fee_breakdown = pool.calculate_swap_fees(amount_in)?;
swap_state.amount_remaining = amount_in - fee_breakdown.total_fee;

// 3. Execute concentrated liquidity swap
let amount_out = execute_concentrated_liquidity_swap(
    &mut swap_state,
    &mut pool,
    sqrt_price_limit,
    zero_for_one,
    remaining_accounts,
)?;

// 4. Pool state is automatically updated during swap execution
// 5. Transfers handled via CPI with proper authority seeds
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

## Cross-Token Routing

The protocol implements a hub-and-spoke model where all pools pair with FeelsSOL:

### SwapRoute Types
- **Direct**: One token is FeelsSOL (single hop)
- **TwoHop**: Neither token is FeelsSOL (route through FeelsSOL)

### Route Execution
```rust
pub fn execute_routed_swap_handler(
    ctx: Context<ExecuteRoutedSwap>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit_1: u128,
    sqrt_price_limit_2: Option<u128>,
) -> Result<u64>
```

Two-hop swaps execute sequentially:
1. Token A → FeelsSOL (first pool)
2. FeelsSOL → Token B (second pool)

## Oracle Integration

The protocol maintains price observations for TWAP calculations:
- **ObservationState**: Stores up to 128 observations in a circular buffer (Phase 1)
- **EnhancedOracle**: Stores up to 1024 observations with volatility tracking (Phase 2)
- **Cumulative tick tracking**: Enables time-weighted average price calculations
- **Update frequency**: Every swap updates the oracle

## Phase 2 Features (Always Enabled)

1. **Enhanced Oracle**: Extended observation storage and volatility tracking
2. **Position Vault**: Automated liquidity management
3. **Dynamic Fees**: Configurable fee adjustments based on market conditions
4. **Volume Tracking**: Per-pool volume statistics

This algorithm forms the core of the Feels Protocol's efficiency and capital optimization, enabling traders to access deep liquidity with minimal slippage while providing LPs with concentrated exposure to their desired price ranges.