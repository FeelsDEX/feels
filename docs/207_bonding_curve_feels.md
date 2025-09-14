# Feels Bonding Curve (Pool Price Discovery)

This document specifies the design of the refactored Feels Bonding Curve. The system is inspired by the virtual-reserve, constant-product models used by platforms like `pump.fun`, but is implemented using the protocol's native Concentrated Liquidity (CLMM) engine.

## 1. Overview

The goal of the bonding curve phase is to provide a smooth, predictable, and bot-resistant price discovery process for new tokens. Instead of using a true `x*y=k` mathematical formula, we will **approximate a virtual constant-product curve** by deploying a large number of small, contiguous, protocol-owned liquidity positions.

This process functions as **Phase 1** of a pool's life, managed by the `PoolController`. During this phase, the pool is entirely controlled by the protocol to ensure a fair launch.

## 2. Core Principles

-   **Protocol-Only Liquidity**: To ensure the bonding curve's integrity, all third-party LPing is **disabled** during this phase. The protocol is the sole counterparty to all trades.
-   **Simulated `x*y=k` Curve**: From a trader's perspective, the experience will feel like swapping on a classic bonding curve with a smooth, continuous price path. Under the hood, they are swapping against hundreds of discrete CLMM positions programmatically arranged to form a hyperbola.
-   **Graduation Trigger**: The bonding curve phase is finite. It concludes when the market reaches a predefined **target market cap** (e.g., 85 SOL raised), at which point it "graduates".
-   **Seamless Transition to Steady-State**: All capital raised during the bonding phase is automatically rolled over to seed the protocol's long-term, steady-state market-making strategies (Floor and JIT POMM).

## 3. Discretized Liquidity Implementation

This approach replaces the old "staircase" ladder with a much more granular and sophisticated structure that approximates a smooth curve.

### 3.1. Target Curve Definition

First, we define the parameters of the virtual `x*y=k` curve we want to simulate, similar to the `pump.fun` model. This is defined by a set of initial virtual reserves which determine the constant `k`.

-   **Virtual SOL Reserves (`V_S`)**: e.g., 30 SOL
-   **Virtual Token Reserves (`V_T`)**: e.g., 1,073,000,000 tokens
-   **Constant Product (`k`)**: `V_S * V_T`

### 3.2. Algorithm for Curve Discretization

The `deploy_bonding_curve_liquidity` instruction executes the following algorithm to transform the smooth virtual curve into a series of real CLMM positions:

1.  **Define Price Range**: Determine the start and end price for the bonding curve. The start price is derived from the initial virtual reserves. The end price is the price at which the market cap graduation target is met.

2.  **Generate Price Points**: Divide the price range into `N` discrete price points (e.g., `N=200`). These points are spaced **geometrically**, not linearly, to create smaller steps at lower prices and larger steps at higher prices, which naturally maps to the `x*y=k` curve's shape.

3.  **Convert to Ticks**: Each price point `P_i` is converted into a tick index `T_i`. This creates `N-1` small, contiguous tick ranges: `[T_1, T_2]`, `[T_2, T_3]`, etc.

4.  **Calculate Liquidity for Each Tranche**: For each micro-range `[T_i, T_{i+1}]`, the algorithm calculates the precise amount of CLMM liquidity (`L_i`) needed to make that segment behave like the target `x*y=k` curve. This is done by solving the CLMM liquidity formula for `L`:
    -   First, calculate the amount of virtual tokens (`ΔV_T`) that would be sold from the ideal `x*y=k` curve between prices `P_i` and `P_{i+1}`.
    -   Then, using the CLMM formula `ΔTokens = L * (√P_upper - √P_lower)`, solve for `L_i`:
        `L_i = ΔV_T / (√P_{i+1} - √P_i)`

5.  **Deploy Positions**: The instruction loops `N-1` times, creating a protocol-owned CLMM position for each range `[T_i, T_{i+1}]` with the calculated liquidity `L_i` and depositing the required number of tokens from the initial escrow.

The result is a fine-grained approximation of a smooth hyperbola, built from discrete CLMM positions.

## 4. Pool Lifecycle

### 4.1. Phase 1: Bonding Curve Active

-   **Deployment**: The `deploy_bonding_curve_liquidity` instruction creates the discretized hyperbola curve as described above.
-   **LPing Disabled**: The `PoolController` sets a flag on the pool that prevents any user from calling `open_position` or other liquidity-modifying instructions.
-   **Trading**: Users buy the new token with `FeelsSOL`. Swaps are executed by the standard CLMM engine against protocol‑owned liquidity. The standard **Dynamic Fee Model** applies to all trades.
-   **Graduation Check**: After every swap, the contract checks if the total `FeelsSOL` collected has reached the graduation market cap.

### 4.2. Phase 2: Seamless Transition to Steady-State

A critical design goal is to graduate the pool from its bonding curve to an open, steady-state AMM with **zero downtime**. This is achieved by adding the new, permanent liquidity *before* removing the old, temporary bonding curve liquidity.

1.  **Graduation Trigger**: The `swap` transaction that meets or exceeds the graduation cap flips the `PoolController` state from `PriceDiscovery` to `SteadyState`. Trading continues uninterrupted, but internal logic now changes for all subsequent actions.

2.  **Deployment of Steady-State Liquidity**: Once the pool is in the `SteadyState`, a permissionless crank instruction, `deploy_steady_state_liquidity`, can be called. This crucial, one-time transaction performs the initial deployment of the permanent protocol‑owned strategies.

    -   **Source of Capital**: The crank's first step is to calculate the total assets collected during the bonding phase. These assets, held in the pool vaults from the now-inactive bonding curve positions, consist of:
        1.  All the `FeelsSOL` paid by buyers.
        2.  All the remaining, unsold project tokens.

    -   **Capital Allocation**: This recovered capital is then split (e.g., a 95/5 ratio) to fund the two steady-state strategies:

    -   **Floor POL Deployment (~95% of `FeelsSOL` Capital)**:
        -   **Denomination**: The Floor requires `FeelsSOL` to create a buy wall for the project token.
        -   **Price (Tick) Calculation**: The `pool::Floor` calculates the `initial_floor_tick`. This is derived from the `Pool-Level Floor Price Formula`: `Floor Price = (95% of FeelsSOL Raised) / (Number of Tokens Sold)`. 
        -   **Placement**: The crank creates a single, large, one-sided concentrated liquidity position on behalf of the protocol. This position is placed in the range `[global_lower_tick, initial_floor_tick]` and is funded with the allocated `FeelsSOL`. Because the current market price is guaranteed to be above the `initial_floor_tick`, this position consists entirely of `FeelsSOL`, forming a deep, permanent buy wall.

    -   **JIT POL Seeding (~5% of `FeelsSOL` + remaining tokens)**:
        -   **Denomination**: The JIT system needs a mix of assets to operate.
        -   **Placement**: The remaining capital—the ~5% of `FeelsSOL` and all of the unsold project tokens—is transferred into the pool's dedicated **`Pool Buffer (τ)`** account. This provides the initial capitalization required for the JIT system to begin its reactive, on-the-fly market making.

3.  **Pool Opening**: In the same `deploy_steady_state_liquidity` transaction, the `lping_enabled` flag is set to `true`, immediately opening the pool to third-party LPs.

4.  **Cleanup of Bonding Curve Liquidity**: A separate, permissionless crank instruction, `cleanup_bonding_curve`, can now be called multiple times to safely remove the redundant bonding curve positions and reclaim rent, without affecting the now-active market.

## 5. Ungraduated Pools

If a pool never reaches its graduation cap, it remains in the bonding curve phase indefinitely. Users can continue to buy and sell against the curve. The protocol does not intervene, and the token does not graduate to an open AMM.

## 6. Interaction with the Pool Controller

The `PoolController` remains the central controller for this entire process:

-   It manages the state transition from `PriceDiscovery` (the bonding curve phase) to `SteadyState`.
-   It enforces the "no LPing" rule during the `PriceDiscovery` phase.
-   It applies the appropriate fee-splitting logic (typically the "Bootstrap Regime") to all fees collected during the bonding curve phase.
-   It orchestrates the withdrawal and re-allocation of capital upon graduation.

This refactored design provides the user experience and economic properties of an established bonding curve model, while leveraging the power and efficiency of our existing CLMM infrastructure.
