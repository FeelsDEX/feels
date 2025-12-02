# Exchange Pricing Specification

## 1. Overview

The Feels Protocol uses a **Market Physics** model to determine exchange rates and system state. Unlike traditional AMMs that rely on static invariants, Feels employs a **Unified 3D Liquidity Space** where pricing emerges from the fundamental physics of Spot, Time, and Leverage dimensions.

This document details the mathematical framework, the dynamic parameter system, and the hybrid on-chain/off-chain architecture used to maintain efficient pricing.

## 2. Mathematical Framework

### 2.1 The Unified Invariant
A position exists in a 3D state space $\vec{P} = (S, T, L)$. The system preserves the **Trading Invariant**:

$$K_{trade} = S^{\hat{w}_s} \cdot T^{\hat{w}_t} \cdot L^{\hat{w}_l}$$

Where:
*   $S$: Spot liquidity (Token Balances)
*   $T$: Time liquidity (Duration-locked value)
*   $L$: Leverage liquidity (Directional exposure)
*   $\hat{w}_i$: Normalized dimension weights ($\hat{w}_i = \frac{w_i}{1 - w_\tau}$)

### 2.2 Potential Fields
The system seeks to minimize its total potential energy. This "Potential Field" defines the pricing landscape:

$$V_{total} = -\ln(K_{trade}) = -\hat{w}_s \ln(S) - \hat{w}_t \ln(T) - \hat{w}_l \ln(L)$$

*   **Low Potential**: Balanced state (Equilibrium).
*   **Gradient ($\nabla V$)**: The "force" resisting movement. This force determines the instantaneous price impact and fees.

## 3. System Architecture

To balance mathematical sophistication with gas efficiency, pricing calculations use a **Hybrid Verification Model**:

| Component | Responsibility | Context |
|-----------|---------------|---------|
| **Keeper Network** | Calculates complex physics, gradients, and dynamic parameters. | Off-Chain |
| **Smart Contract** | Verifies proofs and updates global state. | On-Chain |
| **Field Commitment** | A compact representation of the current physics state. | On-Chain Storage |

### 3.1 Data Structures

Keepers submit these structures to prove the validity of the pricing model:

```rust
pub struct OptimalityCertificate {
    // Lower bound via convex relaxation
    pub lower_bound: ConvexBound,
    
    // Optimality gap (basis points)
    pub gap_bps: u16, 
    
    // Witness for verification
    pub proof: ConvexProof,
}

pub struct ConvexProof {
    // Sampling points where convex bound is tight
    pub tight_points: Vec<Tick3D>,
    
    // Maximum deviation between actual and convex
    pub max_deviation: FixedPoint,
    
    // Lipschitz constant for gradient bound
    pub lipschitz_constant: FixedPoint,
}
```

### 3.2 Verification Logic
The smart contract performs cheap spot-checks:
1.  **Convex Bound Check**: Verifies the committed lower bound at random points.
2.  **Lipschitz Check**: Verifies the gradient field is smooth.
3.  **Gap Check**: Ensures the solution is within `MAX_GAP_BPS` of optimal.

## 4. Active Physics (Dynamic Parameters)

The system adapts its parameters to market conditions (volatility, risk) to maintain efficiency.

### 4.1 Dynamic Alpha (Funding Sensitivity)
Controls how aggressively the system corrects funding rates based on volatility.

$$ \alpha_{dynamic} = \alpha_{base} \cdot \left( 1 + \max(0, \frac{\sigma_{current}}{\sigma_{baseline}} - 1) \cdot K_{\alpha} \right) $$

*   **Logic**: Higher volatility $\rightarrow$ Higher $\alpha$ $\rightarrow$ Stronger funding pressure $\rightarrow$ Faster rebalancing.
*   **Bounds**: Clamped to $[\alpha_{min}, \alpha_{max}]$.

### 4.2 Dynamic Beta (Lending Sensitivity)
Controls the steepness of interest rate curves based on utilization spread.

$$ \beta_{dynamic} = \beta_{base} \cdot \left( 1 + (U_{max} - U_{min}) \cdot K_{\beta} \right) $$

*   **Logic**: Wider utilization spread $\rightarrow$ Higher $\beta$ $\rightarrow$ Sharper rate changes $\rightarrow$ Stronger incentives to supply/borrow.
*   **Bounds**: Clamped to $[\beta_{min}, \beta_{max}]$.

### 4.3 Dynamic Weights (Risk Management)
Weights shift to protect the system during extreme imbalances.

```rust
if leverage_imbalance > CRITICAL_IMBALANCE {
    let shift = (leverage_imbalance - CRITICAL_IMBALANCE) * WEIGHT_SHIFT_RATE;
    weights.leverage += shift;
    weights.spot -= shift / 2.0;
    weights.time -= shift / 2.0;
}
```

### 4.4 Update Schedule
| Computation | Frequency | Max Staleness | Fallback Strategy |
|---|---|---|---|
| Volatility | 5-10 min | 30 min | Use last known |
| Dynamic $\alpha, \beta$ | 15 min | 1 hour | Compute simple approx |
| Base Rates | 1 hour | 4 hours | Use last known |
| Gradient Tables | 4-6 hours | 24 hours | Linear approximation |

## 5. Reference Constants

These values tune the responsiveness of the physics engine.

```rust
// Sensitivities
const ALPHA_VOL_SENSITIVITY: f64 = 0.5;    // +50% alpha per +100% vol
const BETA_SPREAD_SENSITIVITY: f64 = 2.0;  // +200% beta per +100% spread
const WEIGHT_SHIFT_RATE: f64 = 0.1;        // Weight shift per unit imbalance

// Thresholds
const BASELINE_VOLATILITY: f64 = 0.2;      // 20% annualized vol
const CRITICAL_IMBALANCE: f64 = 0.3;       // 30% imbalance triggers shift

// Safety Bounds
const ALPHA_LIMITS: (f64, f64) = (0.01, 2.0);
const BETA_LIMITS: (f64, f64) = (0.1, 10.0);
const WEIGHT_LIMITS: (f64, f64) = (0.1, 0.6);
```

## 6. Units and Normalization
*   **Time**: Seconds.
*   **Rates**: Decimals (0.05 = 5% APR).
*   **Volatility**: Annualized Standard Deviation (0.2 = 20%).
*   **Weights**: Sum to 1.0 (represented as fixed-point).
