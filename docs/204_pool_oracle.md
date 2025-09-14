# Pool Oracle (GTWAP) Specification

This document specifies the design and implementation of the on-chain Geometric Time-Weighted Average Price (GTWAP) oracle used by each Feels market pool.

## 1. Overview

Each Feels pool requires a robust, on-chain, and manipulation-resistant price feed for its internal operations, such as for the dynamic fee model and JIT liquidity system. The protocol cannot rely on external oracles, especially for newly launched tokens that have no other trading venues.

To solve this, each pool implements its own oracle that calculates a Geometric Time-Weighted Average Price (GTWAP). This oracle is updated with every swap, providing a continuously refreshed, manipulation-resistant price source derived directly from trading activity within the pool.

## 2. Core Concepts

### 2.1. Time-Weighted Average Price (TWAP)

A TWAP is an average price calculated over a period of time. Unlike a simple moving average, a TWAP is weighted by the amount of time each price point was active. This makes it significantly more resistant to short-term price manipulation (e.g., via flash loans) because an attacker must hold the price at an artificial level for a sustained period to have a meaningful impact on the average, which is economically costly.

### 2.2. Geometric Mean and Ticks

The Feels oracle calculates a geometric mean of the price, not an arithmetic one. This is achieved by averaging the tick index over time.

- **Ticks are Logarithmic**: As described in the CLMM specification, the price at a given tick is `price = 1.0001^tick_index`. This is a logarithmic relationship.
- **Averaging Logs**: The average of logarithms is the logarithm of the geometric mean: `(log(a) + log(b)) / 2 = log(sqrt(a*b))`.

By calculating the time-weighted average of the tick index, the oracle is implicitly and efficiently calculating the geometric mean of the price, which is better suited for representing proportional price changes in financial markets.

### 2.3. Cumulative Tick Value

To calculate the average tick efficiently, the oracle does not store a long history of individual price points. Instead, it stores a cumulative tick value at discrete time intervals.

- `tick_cumulative = Σ (tick_i * time_delta_i)`

This value represents the integral of the tick index over time. By taking two `Observation`s, the average tick over that period can be calculated with a single subtraction and division:

- `avg_tick = (tick_cumulative_new - tick_cumulative_old) / (timestamp_new - timestamp_old)`

## 3. Data Structures

The oracle state is stored in a dedicated `OracleState` account, linked to a specific `Pool`.

### 3.1. `OracleState` Account

This account holds a ring buffer of observations.

```rust
// programs/feels/src/state/pool_oracle.rs

#[account]
pub struct OracleState {
    pub pool_id: Pubkey,
    pub observation_index: u16,
    pub observation_cardinality: u16,
    pub observations: [Observation; MAX_OBSERVATIONS],
    // ... and other fields
}
```

- **`observations`**: A fixed-size array (currently `MAX_OBSERVATIONS = 12`) that acts as a ring buffer.
- **`observation_index`**: The index of the most recently written observation in the array.
- **`observation_cardinality`**: The number of initialized observations in the buffer. This grows from 1 to `MAX_OBSERVATIONS` as the oracle records more data.

### 3.2. `Observation` Struct

Represents a single data point recorded by the oracle.

```rust
// programs/feels/src/state/oracle.rs

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct Observation {
    pub block_timestamp: i64,
    pub tick_cumulative: i128,
    pub initialized: bool,
}
```

## 4. How It Works

### 4.1. Initialization

- When a new pool is created via the `initialize_pool` instruction, a corresponding `OracleState` account is also created.
- The `initialize` method is called, which populates the first slot (`index = 0`) of the `observations` array with the current block timestamp and a `tick_cumulative` of 0.
- The `observation_cardinality` is set to 1.

### 4.2. Updating Observations

- The `update` method on the `OracleState` account is called at the end of every swap.
- The method performs the following steps:
  1. It retrieves the last observation from the ring buffer.
  2. It only proceeds if the current block timestamp is greater than the last observation's timestamp.
  3. It calculates the time delta since the last observation.
  4. It calculates the new `tick_cumulative` by adding `(last_tick * time_delta)` to the previous `tick_cumulative`.
  5. It advances the `observation_index` to the next slot in the ring buffer.
  6. It writes a new `Observation` with the current timestamp and the new cumulative value into this new slot.
  7. If the ring buffer is not yet full, it may increment the `observation_cardinality`.

### 4.3. Calculating the GTWAP

- Any on-chain program can calculate the GTWAP over a desired period by calling `get_twap_tick(seconds_ago)`.
- This function:
  1. Determines the target timestamp (`now - seconds_ago`).
  2. Searches the `observations` array to find the initialized observation whose timestamp is closest to, but not after, the target timestamp.
  3. It retrieves this old observation and the most recent observation.
  4. It calculates the average tick as `(tick_cumulative_new - tick_cumulative_old) / (timestamp_new - timestamp_old)`.

## 5. Security and Manipulation Resistance

The GTWAP oracle design is inherently resistant to several forms of attack:

- **Flash Loan Resistance**: An attacker cannot manipulate the TWAP with a single-transaction flash loan. To influence the average, they must hold the market at an artificial price for a duration, incurring significant capital costs and risk.
- **Minimum Duration**: The `get_twap_tick` function enforces a `MIN_TWAP_DURATION` (currently 60 seconds). This prevents queries over extremely short and easily manipulated time windows, ensuring that any calculated TWAP is based on a meaningful period of trading activity.
- **Timestamp Dependency**: While the oracle relies on block timestamps, which can have minor variance, the time-weighting mechanism averages out small fluctuations. Significant timestamp manipulation is difficult for validators to perform without being slashed.

## 6. Integration within the System

The GTWAP is a foundational pool component used by other systems within a pool (dynamic fees, JIT, floor).

### 6.1. Relationship to Protocol Oracle

The pool GTWAP oracle is separate from the protocol‑level reserve rate oracle. The protocol oracle provides a conservative FeelsSOL↔JitoSOL exchange rate used for global solvency checks and treasury accounting. Consumers should fetch GTWAP from `pool::Oracle` and the reserve rate from `protocol::Oracle` as needed; do not conflate the two.

### 6.2. Consumers of GTWAP

- **Dynamic Fee Model**: The dynamic fee calculation uses the GTWAP as the "equilibrium" price. Trades moving the price away from the GTWAP pay a surcharge, while trades moving the price toward it can receive a discount.
- **JIT Liquidity**: The Just-In-Time liquidity provider uses the GTWAP as its primary anchor for placing quotes, ensuring its operations are centered around a stable, manipulation-resistant price point.
