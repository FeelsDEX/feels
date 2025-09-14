# Phase 2 Roadmap

This document lists the deferred features to enable after the MVP demonstrates stability. Each item includes dependencies, a proposed rollout order, and success metrics.

## 1) Advanced Fee Mechanics

- Features: Equilibrium target with floor bias, rebates, direction adjustment, momentum factor, warmup ramp.
- Dependencies: pool::Oracle (GTWAP), pool::Floor, FlowSignals (for momentum cache), added PoolState fields.
- Rollout:
  1. Enable equilibrium (no rebates), anchoring to GTWAP with floor bias.
  2. Enable small rebates for toward‑equilibrium trades; cap magnitude.
  3. Enable momentum factor with conservative bounds and per‑slot cache.
  4. Tune via simulations and dashboard feedback loops.
- Success Metrics: fee_bps distribution stable, revert rate low, improved price stabilization (lower volatility at same volume), no pick‑off spikes.

## 2) JIT v1 Enhancements

- Features: Toxicity EMA, FlowSignals fusion, symmetric fallback mode, limited inventory holds with maturity, momentum‑informed spread.
- Dependencies: FlowSignals module and minimal additional JIT state; SafetyController thresholds.
- Rollout:
  1. Turn on toxicity EMA (no inventory change), keep burn-by-default.
  2. Add symmetric fallback with hard caps for ambiguous direction.
  3. Evaluate limited inventory holds with strict size/time caps.
- Success Metrics: lower fail‑to‑execute rate, acceptable pick‑off loss bounds, stable CU.

## 3) Creator Performance Bonus

- Features: True Net Profit payout from PoolBuffer to creator on epoch schedule.
- Dependencies: PoolBuffer accounting snapshots; JIT PnL tracking; PoolController payout function.
- Rollout: add after stable fee/jit metrics; start with tiny payout_% and increase gradually.
- Success Metrics: predictable creator income, no negative impact on JIT/Buffer solvency.

## 4) Protocol Oracle v2

- Features: Combine Jito native rate and DEX TWAP (min with divergence guard).
- Dependencies: DEX price feeds with liquidity filters; SafetyController divergence thresholds.
- Rollout: start with read‑only monitoring; switch production path after stability.
- Success Metrics: lower redemption premium/discount drift without raising risk.

## 5) FlowSignals and Safety Telemetry

- Features: Unified FlowSignals feeding fees and JIT; dashboards for staleness, caps, fee histograms, JIT usage.
- Dependencies: event indexing; storage for light state.
- Rollout: deploy telemetry first; then connect to fee/JIT gates.
- Success Metrics: actionable dashboards; parameter changes correlate to expected outcomes.

## 6) Bonding Curve Refinements

- Features: Adjust discretization (N) and graduation strategy based on observed CU and UX; consider more granular tranches if needed.
- Dependencies: none; governance only.
- Rollout: parameter change with back‑tests.
- Success Metrics: smooth graduation, minimal CU, stable price discovery.

## 7) Vaults & Lending Integration

- Features: DurationVault (protocol‑level) + PoolController rebalance_hook to allocate a capped portion of PoolReserve to lending buckets; later, borrow flows and liquidation rules.
- Dependencies: see 400_vaults_and_lending_future.md; new governance params: enable_vaults, vault_allocation_cap_bps, min_floor_reserve_ratio_bps, duration_buckets.
- Rollout: enable `enable_vaults=false` first; dry‑run accounting via crank; then small cap and single bucket.
- Success Metrics: floor safety maintained; predictable capacity; no lending‑induced volatility.

---

Recommended Order: (1) Advanced Fees → (2) JIT v1 → (3) Protocol Oracle v2 → (4) Creator Bonus → (5) Signals/Telemetry → (6) Curve refinements → (7) Vaults & Lending.

