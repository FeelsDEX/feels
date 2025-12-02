---
title: "After Swap Pipeline"
description: "Post-swap processing and state updates"
category: "Specifications"
order: 208
draft: false
searchable: true
---

# Pool After-Swap Pipeline

This document specifies the atomic, ordered post-swap updates owned by `MarketController` (planned, currently implemented directly in swap logic). It ensures market subsystems update consistently with safety and clear degrade rules.

## Ordering (MVP)

1. Update market GTWAP oracle
   - `market::Oracle.update(end_tick, timestamp)` (impl: `OracleState::update()`)
   - If GTWAP stale (cardinality/time), fall back to current tick and mark degraded.

2. Compute dynamic fee (post-execution)
   - Inputs: start_tick, end_tick, trade_direction, amount_in, PoolState, current_slot
   - Output: `fee_bps` within `[MIN_TOTAL_FEE_BPS, MAX_TOTAL_FEE_BPS]`
   - Apply user-provided `max_fee_bps` cap (revert if exceeded).

3. Split fees (configurable split)
   - Recipients: LP accumulator, PoolReserve, PoolBuffer, Protocol Treasury, Creator (protocol tokens)
   - Apply rounding policy: conservative toward protocol solvency on ties.

4. Update FlowSignals (lightweight; MVP)
   - Update flow EWMA from swap; optionally incorporate JIT metrics when enabled.

5. JIT v0 (optional; MVP)
   - If `enable_jit` is true, place a contrarian micro-band (fixed spread, 1-tick range) funded from PoolBuffer, respecting per‑swap/per‑slot budgets and floor guard; execute within the swap and remove unfilled.

6. Floor maintenance (bounded)
   - If ratchet cooldown passed, recompute floor via `market::Floor.calculate_floor_tick()` and ratchet up if higher.
   - Enforce `ask_tick >= floor_tick + buffer` invariant for any protocol-owned asks.

7. SafetyController observe
   - Record metrics (fee bps distribution, volatility, oracle freshness) and enforce degrade actions if thresholds crossed.

Noncritical steps: Event emission and optional metrics updates are best‑effort; failures in these steps must not revert an otherwise valid swap.

## Degraded Mode Rules

- GTWAP stale: disable rebates (direction bonuses), raise impact floor to `min_floor_bps`, proceed with swaps.
- Protocol oracle stale: swaps OK; pause `exit_feelssol` until healthy.
- Excess volatility: widen fee bounds minimally; do not exceed user fee caps.

## Required Accounts (per swap)

- Market, vaults, market::Oracle (OracleState), MarketController (planned), MarketReserve (planned), Buffer (τ),
- LP fee accumulators, FlowSignals, market::Floor, SafetyController.

## Events (emit)

- FeeSplitApplied, RebateApplied (if any), OracleUpdated (pool),
- FloorRatcheted (if changed), SafetyDegraded/Paused (if actioned).
