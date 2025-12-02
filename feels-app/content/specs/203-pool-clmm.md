---
title: "Concentrated Liquidity AMM"
description: "Architecture and mechanics of the Feels Protocol automated market maker"
category: "Specifications"
order: 203
draft: false
searchable: true
---

# Concentrated Liquidity AMM (Market)

This document specifies the architecture and mechanics of the automated market maker used in the Feels Protocol. Feels uses a Uniswap V3-style concentrated liquidity design, utilizing ticks, ranged positions, and discrete liquidity management to achieve high capital efficiency.

**Note**: While this document uses "market" and "pool" interchangeably to refer to a trading pair, the implementation uses the `Market` account structure.

## 1. Core Concepts

### 1.1. Price, Ticks, and Liquidity

- **Price**: The ratio of two tokens in a pool. In the Feels system, all prices are represented as `sqrt_price`, a Q64.64 fixed-point number, which simplifies many mathematical calculations. `sqrt_price = sqrt(price_token1 / price_token0) * 2^64`.
- **Tick**: A discrete price point. The entire price range is divided into discrete ticks. Each tick corresponds to a specific price. The relationship is `price = 1.0001^tick_index`. This means moving one tick changes the price by approximately 0.01% (1 basis point).
- **Tick Spacing**: To prevent griefing with dust liquidity and to manage on-chain data, liquidity can only be added at ticks that are a multiple of `tick_spacing`. A pool can have a `tick_spacing` of, for example, 1, 10, or 100.
- **Liquidity (`L`)**: A virtual quantity representing the depth of the market. In a given price range, the real reserves (`x`, `y`) are related to liquidity by the formulas:
  - `Δx = L * (1/√P_upper - 1/√P_lower)`
  - `Δy = L * (√P_upper - √P_lower)`
- **Active Liquidity**: The total liquidity available at the current pool price. This is the sum of all `liquidity_net` values from ticks below the current tick.

### 1.2. Positions

Instead of providing liquidity across the entire price range (0 to ∞), Liquidity Providers (LPs) can create Positions in specific, finite price ranges defined by a `tick_lower` and `tick_upper`.

- **NFT-Tokenized**: Each position is represented by a unique SPL Token (NFT), which holds the state of the position (liquidity amount, fee growth, etc.).
- **Capital Efficiency**: This allows LPs to concentrate their capital in the price range where they expect most trading to occur, earning more fees for a given amount of capital compared to a full-range AMM.
- **In-Range vs. Out-of-Range**:
  - If the current price is within a position's range, the position consists of both token0 and token1 and earns trading fees.
  - If the current price is below the range, the position consists entirely of token0.
  - If the current price is above the range, the position consists entirely of token1.

### 1.3. Tick Arrays

To manage on-chain data efficiently, individual `Tick` data structures are not stored in their own accounts. Instead, they are grouped into `TickArray` accounts.

- **Fixed Size**: Each `TickArray` stores a fixed number of ticks (e.g., 64).
- **PDA-based**: The address of a `TickArray` is derived from the pool key and the `start_tick_index` of the array, allowing for deterministic lookups.
- **Lazy Initialization**: Ticks within an array are only initialized when liquidity is first added to them.

## 2. Data Structures

### 2.1. `Pool`

The central account for a trading pair.

```rust
// programs/feels/src/state/pool.rs

#[account]
pub struct Pool {
    // Pool status and configuration
    pub is_initialized: bool,
    pub is_paused: bool,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub tick_spacing: u16,
    pub base_fee_bps: u16,

    // Core AMM state
    pub sqrt_price: u128,       // Current Q64.64 sqrt_price
    pub current_tick: i32,      // Current tick index
    pub liquidity: u128,        // Active liquidity at the current_tick

    // Global fee tracking
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,

    // PDA bumps and references
    pub authority: Pubkey,
    pub buffer: Pubkey,
    pub oracle: Pubkey,
    pub pool_authority_bump: u8,
    // ... and other fields
}
```

### 2.2. `TickArray` & `Tick`

Stores the state for a contiguous range of ticks.

```rust
// programs/feels/src/state/tick.rs

#[account(zero_copy)]
#[repr(C)]
pub struct TickArray {
    pub pool: Pubkey,
    pub start_tick_index: i32,
    pub ticks: [Tick; TICK_ARRAY_SIZE],
    // ... and other fields
}

#[zero_copy]
#[repr(C)]
pub struct Tick {
    pub initialized: u8,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
}
```

- **`liquidity_gross`**: The total liquidity that references this tick. It increases whenever a position is opened with this tick as an endpoint.
- **`liquidity_net`**: The *change* in active liquidity when the price crosses this tick.
  - When opening a position from `tick_lower` to `tick_upper`:
    - `liquidity_net` at `tick_lower` is `+liquidity_amount`.
    - `liquidity_net` at `tick_upper` is `-liquidity_amount`.
- **`fee_growth_outside`**: Tracks the total fees earned per unit of liquidity *outside* (i.e., below) this tick. This is crucial for calculating the fees owed to a specific position.

### 2.3. `Position`

Represents a single LP's liquidity contribution.

```rust
// programs/feels/src/state/position.rs

#[account]
pub struct Position {
    pub nft_mint: Pubkey,
    pub market: Pubkey,
    pub owner: Pubkey,
    
    // Range and liquidity
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    
    // Fee tracking snapshot
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    
    // Uncollected tokens
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
    // ... and other fields
}
```

## 3. Price and Liquidity Math

The core math functions are wrappers around the battle-tested `orca-whirlpools-core` library.

### 3.1. Price ↔ Tick Conversion

- `sqrt_price_from_tick(tick: i32) -> u128`: Converts a tick index to a Q64.64 `sqrt_price`.
- `tick_from_sqrt_price(sqrt_price: u128) -> i32`: Converts a `sqrt_price` back to the corresponding tick index (rounding down).

These functions are located in `programs/feels/src/utils/math.rs`.

### 3.2. Liquidity ↔ Amounts Conversion

- **`amounts_from_liquidity`**: Calculates the amounts of `token0` and `token1` that correspond to a given `liquidity` amount and price range. This is used when removing liquidity.
- **`liquidity_from_amounts`**: Calculates the `liquidity` amount that can be created from given amounts of `token0` and `token1` for a specific price range. This is used when adding liquidity.

These functions are located in `programs/feels/src/logic/liquidity_math.rs`.

## 4. Core Instructions

### 4.1. `initialize_pool`

- **Purpose**: Creates a new `Pool` account for a token pair.
- **Process**:
  1. Validates token order, tick spacing, and initial price.
  2. Initializes the `Pool`, `PoolBuffer`, and `PoolOracle` accounts.
  3. Creates the pool's token `vault_0` and `vault_1`.
  4. Revokes mint/freeze authority for protocol-launched tokens to fix their supply.
- **Key Accounts**: `creator`, `token_0`, `token_1`, `pool`, `vault_0`, `vault_1`.

### 4.2. `open_position`

- **Purpose**: Creates a new liquidity position.
- **Process**:
  1. Creates a new `position_mint` (NFT) and `position_token_account` for the user.
  2. Creates the `Position` PDA account to store its state.
  3. Calculates the required `amount_0` and `amount_1` based on the desired `liquidity_amount` and the current pool price.
  4. Transfers the tokens from the user to the pool vaults.
  5. Initializes the `tick_lower` and `tick_upper` in their respective `TickArray` accounts if they are not already initialized.
  6. Updates `liquidity_net` and `liquidity_gross` on both ticks.
  7. Updates the pool's `liquidity` if the new position is active at the current price.
  8. Mints 1 `position_token` to the user.
- **Key Accounts**: `provider`, `pool`, `position_mint`, `position`, `provider_token_0`, `provider_token_1`, `vault_0`, `vault_1`, `lower_tick_array`, `upper_tick_array`.

### 4.3. `close_position`

- **Purpose**: Removes all liquidity from a position and withdraws the underlying tokens and earned fees.
- **Process**:
  1. Verifies the caller owns the position NFT.
  2. Calculates the uncollected fees earned by the position.
  3. Calculates the `amount_0` and `amount_1` corresponding to the position's liquidity at the current price.
  4. Transfers the total tokens (underlying + fees) from the vaults to the user.
  5. Updates the `liquidity_net` and `liquidity_gross` on the position's ticks to remove the liquidity.
  6. Updates the pool's active `liquidity` if the position was in range.
  7. Burns the user's position NFT.
  8. Optionally closes the `Position` and `position_mint` accounts to return rent to the user.
- **Key Accounts**: `owner`, `pool`, `position`, `position_mint`, `owner_token_0`, `owner_token_1`, `vault_0`, `vault_1`, `lower_tick_array`, `upper_tick_array`.

### 4.4. `swap`

- **Purpose**: Executes a trade, swapping one token for another.
- **Process**:
  1. Determines swap direction (`ZeroForOne` or `OneForZero`).
  2. Transfers the input tokens from the user to the appropriate pool vault.
  3. Iterates through initialized ticks in the direction of the swap:
     a. Within each tick segment (the space between two initialized ticks), the swap behaves like a constant product AMM, using the active `liquidity` for that segment.
     b. The amount of input token is consumed, and the corresponding output token is calculated.
     c. Fees are calculated and added to the `fee_growth_global` accumulators for that segment.
     d. When the price crosses an initialized tick:
        i. The active `liquidity` is updated by adding the `liquidity_net` of the crossed tick.
        ii. The `fee_growth_outside` for the crossed tick is flipped to correctly account for fees on either side of it.
  4. The loop continues until the input amount is fully consumed or the price limit is reached.
  5. The total calculated `amount_out` is transferred from the pool vault to the user.
- **Key Accounts**: `user`, `pool`, `vault_0`, `vault_1`, `user_token_in`, `user_token_out`, and a list of `TickArray` accounts (`remaining_accounts`) needed for the swap path.

## 5. Fee Mechanics

- **Global Fee Growth**: The `Pool` account tracks `fee_growth_global_0_x64` and `fee_growth_global_1_x64`. Every time a swap occurs, the fee collected is divided by the active `liquidity` and added to this global accumulator. This represents the total fees earned per unit of liquidity across the entire pool.
- **Outside Fee Growth**: Each `Tick` tracks `fee_growth_outside_x64`. This represents the total fees earned per unit of liquidity *below* that tick.
- **Inside Fee Growth**: The fees earned by a position between `tick_lower` (l) and `tick_upper` (u) can be calculated using the global and outside values. For a given token `i`:
  - `fee_growth_above_l = fee_growth_global_i - fee_growth_outside_l`
  - `fee_growth_below_u = fee_growth_outside_u`
  - `fee_growth_inside = fee_growth_global_i - fee_growth_above_l - fee_growth_below_u`
- **Position Snapshot**: When a position is modified (opened, liquidity added/removed), the current `fee_growth_inside` is snapshotted and stored in the `Position` account. The fees owed to the position are the difference between the current `fee_growth_inside` and the last snapshot, multiplied by the position's `liquidity`.

This "outside" fee accounting mechanism allows for constant-time calculation of fees owed to any position, regardless of how many swaps have occurred.

## External Libraries

The CLMM implementation relies on two key external libraries for its mathematical foundations:

- **`orca-whirlpools-core`**: This library is used for the core pool logic, including price/tick conversions, fixed-point arithmetic (Q64.64 via its `U128` type), and calculating token amount deltas from liquidity.
- **`ethnum`**: Used for `U256` big integer arithmetic. Used in calculations where where intermediate products could exceed the `u128` limit, such as `liquidity_from_amounts`.

## See Also

**Prerequisites (read first)**:
- [GLOSSARY.md](GLOSSARY.md) - Key terms: ticks, liquidity, sqrt_price, zero-copy
- [001-introduction.md](001-introduction.md) - Protocol overview

**Related Systems**:
- [204-pool-oracle.md](204-pool-oracle.md) - GTWAP oracle updated on every swap
- [201-dynamic-fees.md](201-dynamic-fees.md) - Dynamic fee calculation using price impact
- [208-after-swap-pipeline.md](208-after-swap-pipeline.md) - Post-swap processing sequence

**Using CLMM Features**:
- [300-launch-sequence.md](300-launch-sequence.md) - Complete token launch flow using pools
- [207-bonding-curve-feels.md](207-bonding-curve-feels.md) - Bonding curve implementation on CLMM
- [202-jit-liquidity.md](202-jit-liquidity.md) - JIT positions using CLMM ranges

**Configuration**:
- [209-params-and-governance.md](209-params-and-governance.md) - Pool parameters (tick_spacing, base_fee)
- [211-events-and-units.md](211-events-and-units.md) - Event definitions and units
