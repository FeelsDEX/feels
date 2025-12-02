# 3D Market Dynamics

## Overview
This document outlines the engineering work required to transition the Feels Protocol from a standard CLMM MVP to the **Unified 3D Adaptive AMM** described in `docs/specs/exchange_pricing.md`.

The core shift is moving from a static $x \cdot y = k$ invariant to a dynamic 3D pricing surface $V(S, T, L)$ verified by off-chain keepers.

## 1. State Architecture Changes

### 1.1 New Data Structures (`state/pricing.rs`)
We need to define the structures that Keepers will submit and that the contract will store.

```rust
/// Compressed representation of the pricing surface
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PricingState {
    pub root: [u8; 32],        // Merkle root of the gradient field
    pub timestamp: i64,        // When this state was computed
    pub bounds: [u128; 6],     // Min/Max for S, T, L dimensions
    pub global_params: PricingParameters, // Current alpha, beta, weights
}

/// Proof submitted by Keepers to validate the state
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OptimalityCertificate {
    pub lower_bound: ConvexBound,     // Analytical lower bound function
    pub gap_bps: u16,                 // Optimality gap
    pub proof: ConvexProof,           // ZK/Validity proof of constraints
}

/// Dynamic parameters for the pricing engine
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PricingParameters {
    pub alpha: u64,    // Funding sensitivity (Q64.64)
    pub beta: u64,     // Lending sensitivity (Q64.64)
    pub weights: [u64; 4], // Weights for S, T, L, Buffer (sum to 1.0)
}
```

### 1.2 Market State Updates (`state/market.rs`)
The `Market` account needs to track the 3D state and pricing commitment.

**Additions:**
*   `pricing_state: PricingState`: The latest verified pricing state.
*   `time_liquidity: u128`: Total duration-locked value ($T$).
*   `leverage_liquidity: u128`: Total directional exposure ($L$).
*   `last_pricing_update: i64`: For staleness checks.

**Modifications:**
*   Replace `base_fee_bps` with `PricingParameters` (inside the state).
*   Deprecate `PolicyV1` in favor of dynamic governance.

## 2. Logic Implementation

### 2.1 Pricing Engine (`logic/pricing.rs`)
New module to handle the core pricing math.

*   **Imbalance Calculation**: Implement $V = -\sum w_i \ln(x_i)$.
*   **Gradient Verification**: Logic to verify that a specific swap path respects the committed gradient field.
*   **Unified Invariant**: Check $K_{trade} = S^{\hat{w}_s} \cdot T^{\hat{w}_t} \cdot L^{\hat{w}_l}$.

### 2.2 Swap Engine Updates (`logic/engine.rs`)
Refactor `compute_swap_step` to use the pricing model.

*   **Current**: Uses `orca_whirlpools_core` for $x \cdot y = k$.
*   **New**:
    1.  **Check Staleness**: If `now - last_pricing_update > THRESHOLD`, use Fallback Mode.
    2.  **Fallback Mode**: Use simplified constant-product logic with fixed conservative fees (similar to current implementation).
    3.  **Adaptive Mode**:
        *   Calculate "Cost" incurred against the pricing surface: $C = \nabla V \cdot \Delta P$.
        *   Derive fee from Cost: $Fee = f(C)$.
        *   Verify the step against the `PricingState` (requires inclusion proof from caller).

### 2.3 Verification Logic (`logic/verification.rs`)
Implement the "cheap spot-checks" for Keeper updates.

*   **Convex Bound Check**: Evaluate the committed lower bound at random points to ensure validity.
*   **Lipschitz Check**: Verify the gradient field doesn't change too abruptly (prevent infinite fees).

## 3. New Instructions

### 3.1 `update_pricing`
*   **Caller**: Whitelisted Keepers (initially) or Permissionless (with bond).
*   **Args**: `PricingState`, `OptimalityCertificate`.
*   **Logic**:
    1.  Verify `OptimalityCertificate` (spot checks).
    2.  Update `market.pricing_state`.
    3.  Update `market.last_pricing_update`.

### 3.2 `swap_exact_in_v2`
*   **Args**: Standard swap args + `PricingProof` (merkle path to committed gradient).
*   **Logic**:
    1.  Verify `PricingProof` against `market.pricing_state`.
    2.  Execute swap using the proven gradient.

## 4. Integration Points

### 4.1 `state/market.rs`
*   **Action**: Add `pricing_state` and `last_pricing_update` fields to `Market` struct.
*   **Action**: Add `time_liquidity` and `leverage_liquidity` fields.
*   **Action**: Update `Market::LEN` to account for new fields.

### 4.2 `instructions/swap.rs`
*   **Action**: Update `Swap` struct to include `pricing_proof` (if needed as remaining account or instruction arg).
*   **Action**: In `swap` handler, pass `pricing_state` to `execute_swap_steps`.

### 4.3 `logic/swap_execution.rs`
*   **Action**: Update `execute_swap_steps` to accept `pricing_state`.
*   **Action**: Inside the loop, call `compute_swap_step` with pricing context.

### 4.4 `logic/engine.rs`
*   **Action**: Modify `compute_swap_step` to branch between `Adaptive Mode` and `Fallback Mode`.
*   **Action**: In `Adaptive Mode`, calculate fee based on gradient cost $C$.

## 5. Roadmap

1.  **Phase 1: Data Structures & State**: Update `Market` struct and define `PricingState`. (Breaking change for IDL).
2.  **Phase 2: Keeper Instruction**: Implement `update_pricing` and the verification logic.
3.  **Phase 3: Hybrid Engine**: Modify `compute_swap_step` to support both "Fallback" (Legacy) and "Adaptive" modes.
4.  **Phase 4: Keeper Integration**: Deploy off-chain keepers to start feeding pricing updates.
