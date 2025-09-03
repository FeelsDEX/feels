# System Overview

Feels is a hybrid DeFi protocol that combines spot exchange, lending, and leverage. The market structure draws inspiration from thermodynamic models. This document explains how modeling spot trading, lending, and leverage as dimensions in a unified energy landscape enables coherent cross-domain pricing, natural arbitrage relationships, and value-conserving fee structures. We explore the mathematical foundations, dimensional value functions, and trust-minimized architecture that makes this physics-based approach both theoretically sound and practically implementable on-chain.

## Introduction

The Feels Protocol implements a unified 3D AMM that combines spot exchange, lending (time dimension), and leveraged trading under a single thermodynamic framework. Rather than bolting together three disparate markets, we implement one coherent system grounded in a single potential function and strict conservation laws.

The protocol unifies three domains in one AMM: spot exchange, time (lending), and leverage. The design treats market state as a point on an energy landscape and uses conservation to settle flows without creating or destroying value. This allows instantaneous changes (trades) to be priced by how they move the system on the landscape, while continuous effects (interest and funding) act over time to relax the system toward balance.

## Why Thermodynamics?

Thermodynamics provides the unifying model for Feels: markets tend toward equilibrium like physical systems, so we represent state on an energy landscape and price trades by their change in potential; strict conservation prevents value leakage and enables exact, auditable accounting; and unifying spot, time (lending), and leverage as dimensions of one landscape yields coherent cross‑domain pricing, natural arbitrage relationships, and continuous rebalancing, where uphill moves pay fees and downhill moves can earn capped rebates—implemented efficiently and verifiably on‑chain.

## State and Potential

Market state is represented by three value functions $S$, $T$, and $L$. These are combined into a trading invariant:

$$K_{\text{trade}} = S^{\hat{w}_s} \cdot T^{\hat{w}_t} \cdot L^{\hat{w}_l}$$

and a potential:

$$V = -\ln(K_{\text{trade}}) = -\hat{w}_s \ln S - \hat{w}_t \ln T - \hat{w}_l \ln L$$

Here $S, T, L > 0$ are domain value functions measured in a common numeraire, and $\hat{w}_s, \hat{w}_t, \hat{w}_l \ge 0$ are normalized domain weights with $\hat{w}_s+\hat{w}_t+\hat{w}_l=1$. Lower potential corresponds to a more balanced state. The gradient of $V$ represents local resistance. Moving uphill increases potential; moving downhill reduces it. Instantaneous fees are tied to the change in potential along the executed path. Continuous rebalancing acts like heat flow and is modeled with exact exponentials under conservation. Domain weights $\hat{w}_\bullet$ apply to $(S,T,L)$ in $K_{\text{trade}}$, while component weights $w_{S,i}$, $w_{T,d}$, and $w_{L,i}$ live inside each domain’s value function and each set sums to 1.

## Symbols

- $S, T, L$: Domain value functions in a common numeraire (spot, time, leverage).
- $\hat{w}_s, \hat{w}_t, \hat{w}_l$: Normalized domain weights ($\ge 0$, sum to 1).
- $K_{\text{trade}}$: Trading invariant $S^{\hat{w}_s} T^{\hat{w}_t} L^{\hat{w}_l}$.
- $V$: Potential $-\ln K_{\text{trade}}$ (lower is more balanced).
- Template variables: $D$ (generic domain value), $C_i$ (component capacity), $\rho_i$ (component risk), $w_i$ (component weights in the template, mapping to $w_{S,i}, w_{T,d}, w_{L,i}$ per domain).
- $x_a, x_b$: Pool inventories (units of assets $a$ and $b$).
- $p_a, p_b$: Internal TWAP prices in the common numeraire.
- $w_{S,i}, w_{T,d}, w_{L,i}$: Component weights within spot/time/leverage domains (each set sums to 1).
- $I_S, I_T, I_L$: Index sets — $I_S=\{a,b\}$, $I_T$ = duration buckets, $I_L=\{\text{long},\text{short}\}$; default $w_{L,\text{long}}=w_{L,\text{short}}=1/2$.
- $\rho_S$: Spot risk $= \sigma_{price}\,\sqrt{\Delta t}$ with realized volatility $\sigma_{price}$ over window length $\Delta t$ (years).
- $\rho_T(d)$: Time risk per bucket $= \sigma_{\text{rate}}\,\sqrt{d}$ (dimensionless).
- $\rho_L$: Leverage risk $= \sigma_{\text{leverage}}\,|\text{skew}|$ (dimensionless).
- $d$: Duration bucket (years).
- $T_{\text{lend}}(d), T_{\text{borrow}}(d)$: Lending/borrowing notionals.
- $\sigma_{\text{rate}}$: Annualized interest‑rate volatility (per $\sqrt{\text{year}}$).
- $L_{\text{long}}, L_{\text{short}}$: Long/short position capacities; $\text{skew}\in[-1,1]$; $\sigma_{\text{leverage}}$: annualized position volatility.
- $w_i$ (conservation): Snapshot participation weights; $g_i$: multiplicative growth factors; $\tau$: protocol buffer.

## Dimensional Value Functions and Market Discovery

We express each domain as a weighted geometric mean of component capacities penalized by a dimensionless risk term:

$$D = \prod_{i} \left( \frac{C_i}{\sqrt{1 + \rho_i^2}} \right)^{w_i}, \quad \sum_i w_i = 1$$

where $C_i$ is the component capacity and $\rho_i$ is the component risk in that domain. If all $\rho_i$ in a domain are equal ($\rho_i = \rho$), the penalty can be factored into a single outside term via $\prod_i (1+\rho^2)^{w_i} = (1+\rho^2)$.

### Spot Dimension Value Function

The spot dimension maintains capacity through risk‑scaled inventory values in each leg:

$$S = \prod_{i\in I_S} \left( \frac{x_i p_i}{\sqrt{1 + \rho_S^2}} \right)^{w_{S,i}}, \quad \sum_{i\in I_S} w_{S,i} = 1, \quad \rho_S = \sigma_{price}\,\sqrt{\Delta t}$$

where $x_i$ are pool inventories, $p_i$ are internal TWAP prices in the common numeraire, $w_{S,i}\ge 0$ sum to one, and $\rho_S$ is the spot risk with realized volatility $\sigma_{price}$ over window length $\Delta t$ (years). This is algebraically equivalent to the prior single‑denominator form because $\rho_S$ is common across legs.

**Market Discovery**: Price discovery reveals the marginal exchange rate and the price–quantity curve along the bonding curve, with slippage indicating available liquidity. Active depth concentrates around the current tick while effective depth spans nearby ticks.

### Time Dimension Value Function

The time dimension balances lending and borrowing across durations:

$$T = \prod_{d\in I_T} \left(\frac{\sqrt{T_{\text{lend}}(d)\,T_{\text{borrow}}(d)}}{\sqrt{1 + \rho_T(d)^2}}\right)^{w_{T,d}}, \quad \sum_{d\in I_T} w_{T,d} = 1, \quad \rho_T(d) = \sigma_{\text{rate}}\,\sqrt{d}$$

where $d$ indexes fixed duration buckets measured in years, $w_{T,d} \ge 0$ are bucket weights with $\sum_d w_{T,d} = 1$, $T_{\text{lend}}(d)$ and $T_{\text{borrow}}(d)$ are lending and borrowing notionals in the common numeraire, and $\rho_T(d)$ uses annualized interest‑rate volatility $\sigma_{\text{rate}}$ (per $\sqrt{\text{year}}$) to yield a dimensionless penalty.

**Market Discovery**: Rate discovery reveals the time‑value curve that equilibrates deposits and debt across the term structure. Utilization ratios indicate lending capacity and borrowing demand.

### Leverage Dimension Value Function

The leverage dimension captures directional exposure capacity with symmetric components:

$$L = \prod_{i\in I_L} \left( \frac{L_i}{\sqrt{1 + \rho_L^2}} \right)^{w_{L,i}}, \quad \sum_{i\in I_L} w_{L,i} = 1, \quad \rho_L = \sigma_{\text{leverage}}\,|\text{skew}|$$

where $L_{\text{long}}$ and $L_{\text{short}}$ represent long and short position capacities, $\text{skew} \in [-1,1]$ measures directional imbalance, and $\sigma_{\text{leverage}}$ is annualized position volatility. By default we use equal component weights $w_{L,\text{long}} = w_{L,\text{short}} = 1/2$, which recovers the earlier simplified expression $\sqrt{L_{\text{long}}L_{\text{short}}}/\sqrt{1+\rho_L^2}$, since $\prod L_i^{1/2} = \sqrt{L_{\text{long}}L_{\text{short}}}$ and the common risk factor yields a single $\sqrt{1+\rho_L^2}$ in the denominator.

**Market Discovery**: Funding discovery reveals the rate that rebalances long and short interest and bounds effective leverage capacity. Directional skew drives funding payments and capacity constraints.

## Value Conservation

The system continuously settles the joint value function for each market using multiplicative rebasing with exact exponentials. Each rebase respects a subdomain conservation identity of the form:

$$\sum_i w_i \ln g_i = 0$$

where $w_i$ are snapshot participation weights and $g_i$ are multiplicative growth factors for each participant in the subdomain. The protocol buffer $\tau$ participates as needed and is adjusted to satisfy conservation exactly. This keeps accounting precise and auditable: fees and rebates change who holds value without changing the total.

## On-Chain and Off‑Chain Division of Responsibility

Expensive computation is handled off‑chain by a single provider integrated with the keeper system. The chain verifies freshness and frequency to ensure data is current and updated regularly, applies rate of change caps to limit how much values can change per update, enforces strictly increasing sequence numbers, and maintains fee caps as maximum fee limits.

For instantaneous pricing, the chain either:
1. Recomputes the necessary quantities deterministically from posted scalars, or
2. Verifies small inclusion proofs and global bounds when local approximations are supplied

Accepted updates are minimal, and the chain performs only simple arithmetic to apply fees and rebases.

## Safety Controls

Integrity of the price input is maintained through TWAP windows for time-weighted average price calculations, minimum observation requirements, and statistical confidence thresholds.

Leverage flow is bounded relative to depth over the TWAP window and subject to anti‑ping‑pong clamps. Base fees are adjusted by an off‑chain hysteresis rule; the chain accepts only changes consistent with policy.

## Illustrative Fee Sequence

This sequence computes the instantaneous fee (base plus dynamic surcharge from uphill work), applies it to obtain effective input, executes the swap, then applies a capped rebate from downhill work, yielding the final output.

```rust
// Fee calculation and application
effective_in = amount_in - fee_amount
output = execute_swap(effective_in)
output_with_rebate = output + rebate_capped
```
