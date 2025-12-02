---
title: "Floor Liquidity Mechanism"
description: "Protocol-owned liquidity providing price floor support"
category: "Specifications"
order: 205
draft: false
searchable: true
---

# Floor Liquidity (Market Protocol‑Owned Liquidity) Specification

This document specifies the design and implementation of the Floor Liquidity system, the Feels protocol's primary Protocol‑Owned Liquidity (POL) strategy at the market level.

**Implementation Note**: This document describes the complete floor system architecture. Current implementation (MVP) uses simplified floor logic in `Market` struct fields and `logic/floor.rs` helpers. The full `market::Floor` component design is planned for post-MVP.

## 1. Overview

The Floor Liquidity system is the core mechanism by which the Feels protocol converts short-term trading activity and staking yield into a long-term, perpetually rising price floor for a token. Its primary purpose is to provide a guaranteed, on-chain exit price for token holders against protocol-owned liquidity, ensuring permanent liquidity and building long-term value.

This system activates after the initial token launch phase is complete. It replaces the temporary bonding curve liquidity (used for price discovery) and becomes the foundational, passive market-making strategy for the pool, operating in conjunction with the more active Just-in-Time (JIT) liquidity system.

## 2. Core Concepts

### 2.1. Pool-Level Solvency and Pricing

The floor is a pool-level mechanism, meaning each market (e.g., MEME/FeelsSOL) has its own distinct floor price. This design is critical for isolating risk between markets, as described in the [Feels Protocol Solvency](200-feelssol-solvency.md) document.

The floor price for a specific pool is mathematically defined by the reserves allocated to that pool, not the entire protocol's treasury.

At the heart of the system is the Pool-Level Floor Price Formula:

```
Floor Price = Pool's Allocated Reserves (in FeelsSOL) / Circulating Supply of Project Token
```

This calculation ensures that if every token holder were to sell their tokens to the floor liquidity position simultaneously, the *pool* would have enough FeelsSOL reserves to buy every single token. This provides a hard, verifiable backstop for the token's value while maintaining isolation from other markets.

### 2.2. Value Accrual and Allocation

The floor price rises over time through a two-layer process of value accrual at the protocol level and subsequent allocation at the pool level.

1.  **Protocol-Level Accrual**: The main protocol treasury grows from two primary sources:
    *   **Trading Fees**: A portion of fees from *all* markets is routed to the central protocol treasury.
    *   **Staking Yield**: The entire JitoSOL reserve backing all FeelsSOL accrues staking yield, constantly increasing the protocol's total asset value.

2.  **Pool-Level Allocation**: A portion of the globally accrued value is periodically allocated to each individual pool's floor reserves by the Pool Allocation System. This allocation directly increases the `Pool's Allocated Reserves`, which in turn raises the calculated floor price for that specific token.

This architecture ensures that while all pools benefit from the success of the entire protocol, the solvency of each pool's floor remains isolated and independently verifiable.

### 2.3. Monotonic Ratcheting

A key feature of the floor is that it is monotonic: it can only ever rise or stay the same, but it can never decrease. This is enforced by a ratcheting mechanism.

1. The system periodically recalculates the theoretical floor price based on its current reserves.
2. If the newly calculated floor price is higher than the current active floor price, the system "ratchets" the floor up to the new, higher level.
3. A cooldown period (`RATCHET_COOLDOWN`) prevents the floor from being updated too frequently, ensuring smooth and predictable upward movements.

## 3. Architecture and Components

The Floor Liquidity system is managed by a set of unified on-chain components that separate calculation from execution.

### 3.1. The `market::Floor` (calculation)

This component acts as the calculation system for a given market. Each market has its own `market::Floor` instance responsible for calculating that market's specific floor price. Its state is:

```rust
// Specified design (not yet fully implemented)
pub struct MarketFloor {
    pub current_floor: i32,          // The current active floor tick for this pool.
    pub floor_buffer: i32,           // A safety margin (in ticks) above this pool's floor.
    pub last_ratchet_slot: u64,      // Enforces the ratcheting cooldown for this pool.
    pub jitosol_reserves: u128,      // The protocol reserves ALLOCATED to this specific pool's floor.
    pub total_feels_supply: u128,    // The circulating supply of the token in this pool.
}

impl MarketFloor {
    // Calculates the theoretical floor tick based on reserves and supply.
    pub fn calculate_floor_tick(&self) -> i32 { ... }
    
    // Checks if the cooldown period has passed.
    pub fn can_ratchet(&self, current_slot: u64) -> bool { ... }
    
    // Returns the floor + buffer for other systems to use as a safety line.
    pub fn get_safe_ask_tick(&self) -> i32 { ... }
}
```

The `market::Floor` does not directly manage any token accounts or liquidity positions. Its sole job is to serve as the single source of truth for what the floor *should* be.

### 3.2. The Market Controller (execution)

This is a higher-level controller responsible for the *execution* of the `market::Floor` calculations. While `market::Floor` determines the `current_floor` tick, the `MarketController` is the system that actually moves the liquidity on-chain.

**Role**: To manage the protocol's main liquidity `Position` NFT for the floor.

**Process**:
  1. Periodically (e.g., via a crank instruction), it reads the `current_floor` from `market::Floor`.
  2. It compares this to the tick of its currently active floor liquidity position.
  3. If the `current_floor` has been ratcheted up, the Controller executes the necessary transactions to:
     a. Withdraw the liquidity from the old, lower floor tick.
     b. Re-deposit the liquidity as a new, single-sided position at the new, higher `current_floor` tick.

This separation of concerns keeps the core solvency calculation clean and isolated from the complexities of position management.

### 3.3. Circulating Supply Definition (used for Floor)

For floor calculations, a pool’s circulating supply is:

```
circulating_supply = total_token_supply
                     − protocol_owned_tokens
                     − pool_owned_tokens (PoolReserve + PoolBuffer holdings)
                     − prelaunch_escrow_balance (if any)
```

Notes:
- protocol_owned_tokens include any protocol treasury holdings for this token outside the pool.
- pool_owned_tokens include inventory held by PoolController PDAs (e.g., bonded leftovers, JIT inventory if held temporarily in later phases). In MVP, JIT burns on fill; treat JIT as not accumulating inventory.
- prelaunch_escrow_balance should be zero post‑graduation; included for safety.

### 3.4. Minimum Floor Reserve and Allocatable Capital (Future)

To support lending in the future market evolution (Phase 2: Lending), governance may define:
- `min_floor_reserve_ratio_bps`: the minimum proportion of PoolReserve that must remain to satisfy the floor at current circulating supply.
- `vault_allocation_cap_bps`: a ceiling on PoolReserve that can be allocated to the lending/vault subsystem.

PoolController must ensure: `PoolReserve - lending_allocation_q >= floor_min_q` before approving any allocation to lending.

### 3.5. Units and Valuation Notes (MVP)

- PoolReserve and PoolBuffer are denominated in FeelsSOL units at the pool level.
- Floor price/tick is computed from PoolReserve (FeelsSOL) and circulating supply of the pool token; it does not depend on the protocol reserve oracle (JitoSOL rate).
- Protocol mint/redeem paths (FeelsSOL↔JitoSOL) use protocol::Oracle independently; keep these valuations separate to avoid cross‑layer coupling.

## 4. Lifecycle and Mechanics

1.  **Activation**: After the initial launch phase (i.e., after `deploy_bonding_curve_liquidity` is called and the pool graduates), the temporary bonding curve liquidity is removed, and the Floor Liquidity system is activated. The `pool::Floor` calculates the initial floor price based on pool state, and the Pool Controller deploys the first floor position.

2.  **Steady State**: In its normal state, the system maintains a large, single-sided liquidity position at the `current_floor` tick. This position consists entirely of the project token (e.g., MEME) and offers to sell it for FeelsSOL. This creates a perfectly horizontal buy wall at the floor price, capable of absorbing the entire circulating supply of the token.

3.  **Value Accrual & Ratcheting**: As swaps occur, a portion of the fees are routed to the pool’s `PoolReserve`, increasing `jitosol_reserves` allocated to this pool. When `pool::Floor` determines that the floor can be raised and the cooldown has passed, it updates `current_floor`.

4.  **Liquidity Redeployment**: The Pool Allocation system detects the change in the `current_floor` and moves the on-chain liquidity position up to the new tick, cementing the new, higher price floor.

## 5. Interaction with Other Systems

The Floor Liquidity system is a foundational layer that provides stability and guarantees for other protocol components.

-   **Dynamic Fees (Phase 2)**: When advanced fees are enabled, the `pool::Floor`'s `get_safe_ask_tick()` provides the hard floor target for equilibrium calculations, ensuring the fee system never incentivizes trades that would threaten pool solvency. In MVP, fees are base + impact only.

-   **JIT Liquidity**: The Just-in-Time liquidity system queries `pool::Floor` to ensure that its reactive, short-term quotes are never placed below the fundamental price floor. This prevents the active market maker from working against the passive, long-term one.

-   **Launch Sequence**: The Floor system is the designated successor to the bonding curve liquidity deployed in the `deploy_bonding_curve_liquidity` instruction, providing a seamless transition from price discovery to long-term value accrual.

## See Also

**Prerequisites (read first)**:
- [GLOSSARY.md](GLOSSARY.md) - Terms: Floor liquidity, POMM, PoolReserve, ratcheting
- [200-feelssol-solvency.md](200-feelssol-solvency.md) - pool::Floor component (§7)

**Floor Integration**:
- [201-dynamic-fees.md](201-dynamic-fees.md) - Floor provides safety bounds for fees
- [202-jit-liquidity.md](202-jit-liquidity.md) - Floor guard prevents JIT below floor
- [206-pool-allocation.md](206-pool-allocation.md) - PoolReserve funding for floor

**Related Strategies**:
- [202-jit-liquidity.md](202-jit-liquidity.md) - JIT (active) vs Floor (passive) comparison
- [207-bonding-curve-feels.md](207-bonding-curve-feels.md) - Bonding curve transitions to floor at graduation

**Lifecycle**:
- [207-bonding-curve-feels.md](207-bonding-curve-feels.md#pool-lifecycle) - Floor deployment at graduation
- [301-market-state-and-lifecycle.md](301-market-state-and-lifecycle.md) - Market phases

**Solvency**:
- [200-feelssol-solvency.md](200-feelssol-solvency.md#pool-level-solvency) - Pool-level solvency model

**Configuration**:
- [209-params-and-governance.md](209-params-and-governance.md) - Floor buffer, ratchet cooldown parameters
