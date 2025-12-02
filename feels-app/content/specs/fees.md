# Fee System Specification

## 1. Overview

In the Feels Protocol, fees are not arbitrary percentages. They are emergent properties of the system's physics:
*   **Instantaneous Fees** (Trading) emerge from the **Gradient** of the potential field (resistance to movement).
*   **Continuous Fees** (Funding/Yield) emerge from **Conservation Laws** (cost of maintaining position over time).

This document details the fee mechanisms, calculation formulas, and safety protocols.

## 2. Instantaneous Fees (Trading)

Instantaneous fees apply to any action that changes the system's state (e.g., Swaps, Opening Positions). They represent the work done against the potential field.

### 2.1 Calculation Formula
The base fee is the path integral of the gradient along the trade vector:

$$W = \nabla V(P) \cdot \Delta \vec P$$

The actual fee charged to the user is calculated as:

1.  **Surcharge**:
    $$ \text{dyn\_bps} = \min\left( \frac{W \cdot \Pi}{\text{amount\_in}} \cdot 10^4, \text{MAX\_SURCHARGE} \right) $$
    
2.  **Total Fee**:
    $$ \text{fee\_bps} = \text{base\_bps} + \text{dyn\_bps} $$

### 2.2 Rebates (Negative Fees)
If a trade moves the system "downhill" (improving balance, $W < 0$), the user receives a rebate instead of paying a dynamic surcharge.

$$ R = \min(\eta |W| \Pi, \text{cap}_{tx}, \text{cap}_{epoch}) $$

*   $\eta$: Rebate participation factor (0.0 to 1.0).
*   **Caps**: Rebates are strictly limited by the protocol's accumulated buffer to ensure solvency.

## 3. Continuous Fees (Yield & Funding)

Continuous fees represent the cost of time and risk. They are delivered via **Multiplicative Rebasing**, meaning user balances change smoothly over time.

### 3.1 Mechanism
Balances are updated using an exponential growth factor:
$$ g = e^{r \Delta t} $$

*   **Lenders**: See balances grow ($r > 0$).
*   **Borrowers**: See debt grow ($r > 0$).
*   **Traders**: See positions rebase based on funding rates ($r$ can be positive or negative).

### 3.2 Conservation Laws (Solvency)
To guarantee that value is never created from thin air, every rebase epoch must satisfy the **Sub-domain Log-Sum Conservation Law**:

$$ \sum_{i \in \text{domain}} w_i \ln(g_i) = 0 $$

**Example: Lending Domain**
$$ w_{deposit} \ln(g_{deposit}) + w_{debt} \ln(g_{debt}) + w_{buffer} \ln(g_{buffer}) = 0 $$

This ensures that the yield paid to lenders comes exactly from the interest paid by borrowers (plus/minus the protocol buffer).

## 4. Safety & Resilience

### 4.1 Staleness & Fallbacks
The system relies on Keepers to update the physics parameters that determine fees. If Keepers go offline, the system degrades gracefully.

1.  **Grace Period**: For short outages (< 30 min), the system uses the last known good physics parameters.
2.  **Fallback Mode**: If `staleness > threshold`, the system switches to **Bounded-Spread Routing**.
    *   Complex gradient calculations are disabled.
    *   Fees revert to a simple, conservative percentage model (e.g., fixed 0.3%).
    *   Rebates are disabled to prevent gaming stale state.

### 4.2 Testing Matrix
To ensure fee correctness and solvency:

*   **Loop-Work Theorem**: $\oint \nabla V \cdot d\vec P \ge 0$. Proves that no sequence of trades can extract infinite value (no perpetual motion).
*   **Rebase Conservation**: Verifies that $\sum w_i \ln(g_i) = 0$ holds exactly for every epoch.
*   **Stress Tests**: Verifies fee behavior under extreme volatility and utilization.

## 5. User Experience
*   **Transparency**: Instantaneous fees are shown upfront. Continuous fees are visible as real-time balance changes.
*   **No Claiming**: Yield and funding are auto-compounding; users never need to manually "claim" rewards.
*   **Liquidation-Free**: Positions compress via rebasing rather than facing sudden liquidation events.
