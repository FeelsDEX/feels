# Instantaneous Fees (Future Vision)

> **Note on Phased Implementation:** This document describes the fee model for the future, fully-featured 3D thermodynamic version of the protocol. This "Work-based" model relies on an off-chain provider. The current (Phase 1) implementation uses a simpler, fully on-chain dynamic fee model described in `201_dynamic_fees.md`, which is based on realized price impact and an on-chain GTWAP.

This document explains how trades are priced from the change in potential along the executed path, how that change converts to fee basis points, how rebates are bounded, and how the chain verifies a single provider's updates. The fee structure implements work-based pricing where uphill moves (against equilibrium) pay fees and downhill moves (toward equilibrium) can earn rebates.

## Symbol Table

| Symbol | Description |
|--------|-------------|
| $S, T, L$ | Domain value functions |
| $\hat{w}_s, \hat{w}_t, \hat{w}_l$ | Normalized domain weights (sum to 1) |
| $P$ | Market state summarizing $(S,T,L)$ |
| $V(P)$ | Potential at state $P$ |
| $W$ | Work $= V(P_2)-V(P_1)$ |
| $W_{\uparrow}, W_{\downarrow}$ | Uphill/downhill components of work |
| $\Pi_{in}, \Pi_{out}$ | Marginal price maps (input‑token and output‑token per unit work) |
| $\eta$ | Rebate participation fraction in $[0,1]$ |
| $\kappa$ | Price‑improvement clamp in $(0,1]$ |
| `amount_in` | User pre‑fee input |
| `base_bps` | Base fee in basis points |
| `dyn_bps` | Dynamic surcharge (bps) |
| `MAX_SURCHARGE_BPS`, `MAX_INSTANTANEOUS_FEE` | Policy caps |
| $\tau$ | Pool buffer sourcing rebates and absorbing fees |
| $w_{S,i}, w_{T,d}, w_{L,i}$ | Component weights within spot/time/leverage domains (each set sums to 1) |
| $I_S, I_T, I_L$ | Index sets — $I_S=\{a,b\}$, $I_T$ = duration buckets, $I_L=\{\text{long},\text{short}\}$ |

## Work Computation and Fee Derivation

### Potential Change Calculation

For a state change from $P_1$ to $P_2$, the change in potential (work) is computed exactly as:

$$W = V(P_2) - V(P_1) = -\hat{w}_s \ln\left(\frac{S_2}{S_1}\right) - \hat{w}_t \ln\left(\frac{T_2}{T_1}\right) - \hat{w}_l \ln\left(\frac{L_2}{L_1}\right)$$

where $V(P) = -\hat{w}_s \ln S - \hat{w}_t \ln T - \hat{w}_l \ln L$ is the potential function with $\hat{w}_s, \hat{w}_t, \hat{w}_l$ as normalized domain weights, and $P$ denotes market state summarizing $(S,T,L)$. Positive $W$ indicates uphill movement requiring fee payment, while negative $W$ indicates downhill movement eligible for rebates.

### Marginal Price Mapping

We convert work to prices using marginal maps along the executed path: $\Pi_{in}(P)$ in input‑token units per unit of work (for fees) and $\Pi_{out}(P)$ in output‑token units per unit of work (for rebates). For segmented trades, contributions are computed per segment and split into uphill/downhill parts:

$$\text{Total Work} = \sum_{i} W_i,\; W_i = V(P_{i+1}) - V(P_i),\; W_{\uparrow} = \sum_i \max(W_i,0),\; W_{\downarrow} = -\sum_i \min(W_i,0)$$

## Routing Bound and Segmentation

- Hub‑and‑spoke routing via FeelsSOL bounds token‑to‑token paths to at most two hops ($A \to \text{FeelsSOL} \to B$). Entry/exit is $\text{JitoSOL} \leftrightarrow \text{FeelsSOL}$, and FeelsSOL↔Position is a single hop.
- “Segmented trades” refers to splitting a single hop into size‑based segments along the bonding curve; segmentation is independent of hop count. Large trades may use many segments even with 1–2 hops.

## Pool-Type Base Fees and Dynamic Components

### Base Fee Structure by Pool Type

Base fees vary by pool risk profile to reflect operational costs and market conditions:

```rust
base_fee = match pool_type {
    Stable => 5,    // 0.05% for stable pairs (USDC/USDT)
    Normal => 25,   // 0.25% for standard pairs (ETH/USDC)  
    Volatile => 80, // 0.80% for volatile pairs (new/exotic tokens)
}
```

These base fees represent the minimum cost of trading and cover operational expenses including gas, oracle updates, and protocol maintenance.

### Dynamic Surcharge Calculation

The dynamic surcharge in basis points is computed from the uphill work $W_{\uparrow}$ and $\Pi_{in}$ relative to the user's pre‑fee input `amount_in`. Cap precedence is applied in two steps:

1. **First**: The surcharge is clamped to the range $[0, \texttt{MAX\_SURCHARGE\_BPS}]$
2. **Second**: The total fee is clamped

```rust
// Dynamic fee calculation with caps (fees in input-token bps)
let w_up = W.max(0.0);
let denom = (amount_in as f64).max(1.0); // avoid div-by-zero
let mut dyn_bps = (w_up * Pi_in / denom) * 10_000.0; // dimensionless bps
dyn_bps = dyn_bps.clamp(0.0, MAX_SURCHARGE_BPS as f64);
let fee_bps = (base_bps as f64 + dyn_bps).min(MAX_INSTANTANEOUS_FEE as f64) as u16;
```

## Rebates and κ Clamp

Downhill moves reduce potential and can earn a rebate. The rebate is computed in output‑token units as:

$$\text{rebate} = \eta \cdot W_{\downarrow} \cdot \Pi_{out}$$

Here $\eta \in [0,1]$ is the policy‑set rebate participation fraction and $\Pi_{out}$ maps work units to output‑token units along the executed path.

The rebate is capped by:
- **Per‑transaction limits**: Maximum rebate per single transaction
- **Per‑epoch limits**: Maximum total rebates per epoch
- **τ availability**: Buffer availability constraints  
- **κ clamp**: Bounds the fraction of measured price improvement eligible as a rebate

Price improvement is the output gain relative to a base‑fee‑only baseline for the same `amount_in` and route. The κ clamp (with $\kappa \in (0,1]$) is applied in the same token units as the rebate.

## Large Trades and Segmentation

For large trades, per‑segment work is evaluated within each hop and aggregated into $W_{\uparrow}$ and $W_{\downarrow}$; marginal price maps are evaluated locally (or verified via commitments) and integrated accordingly. Two verification options are available:

### Option A: Deterministic Recomputation
Recomputes the necessary quantities deterministically from posted scalars.

### Option B: Local Approximations  
Uses local quadratic approximations that are:
- Committed off‑chain
- Verified with inclusion proofs
- Bounded by global constraints

## Fallback Behavior

When commitments are stale or invalid, only base fees are applied, rebates are disabled, and an optional fixed spread can be used as a conservative fallback.

The system exits fallback when:
1. A fresh, valid update is accepted
2. The minimum dwell time has elapsed

## Implementation Example

This example computes the dynamic surcharge from uphill work using the input‑token price map, clamps both the surcharge and total fee to policy caps, and converts the resulting basis points into an input‑token fee amount.

```rust
// Fee calculation implementation (bps in input-token units)
fn calculate_instantaneous_fee(
    amount_in: u64,
    base_bps: u16,
    work: f64,          // total work W (positive or negative)
    price_map_in: f64,  // Π_in: input-token per unit work
) -> u64 {
    // Uphill contribution only for surcharge
    let w_up = work.max(0.0);
    if amount_in == 0 { return 0; }

    // Dynamic surcharge calculation with caps
    let dyn_bps_f = ((w_up * price_map_in / amount_in as f64) * 10_000.0)
        .clamp(0.0, MAX_SURCHARGE_BPS as f64);
    let fee_bps = (base_bps as f64 + dyn_bps_f)
        .min(MAX_INSTANTANEOUS_FEE as f64) as u16;

    // Convert to token amount
    (amount_in as u128 * fee_bps as u128 / 10_000) as u64
}
```
