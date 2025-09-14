# Future Phase: Vaults and Lending Integration (Design Hooks)

This document outlines the minimal interfaces, constraints, and integration points to add duration vaults and lending in a later phase without refactoring MVP components.

## Objectives

- Introduce a DurationVault where users lock assets for a chosen duration, providing the system with asset duration assurances.
- Extend PoolController to allocate a bounded portion of PoolReserve to lending/liquidity via the vault, while preserving floor solvency.
- Keep MVP intact: no changes to swap path or dynamic fees; all new logic is additive and gated by feature flags.

## Components (Future)

### DurationVault (protocol-level)

- Accepts deposits of FeelsSOL (or pool tokens if designed per‑pool) into duration buckets (e.g., 7d/30d/90d).
- Tracks liabilities by bucket: principal, unlock_time; issues a non‑transferable receipt or ERC‑like position token.
- Exposes available capacity per duration; maintains withdrawal queues.
- Events: VaultDeposit, VaultWithdrawScheduled, VaultWithdrawExecuted, VaultBucketCapacityChanged.

### Lending Reserve (pool-level view)

- PoolController maintains a virtual ledger: `lending_allocation_q` sourced from PoolReserve, bounded by governance caps.
- Lending capacity offered to users (borrow FeelsSOL) against collateral (pool token); pricing deferred to Phase 2.
- Invariants:
  - `PoolReserve >= floor_min_q` at all times
  - `lending_allocation_q <= vault_capacity_q`
  - Per-bucket allocation respects duration matching policies

## Governance Parameters (additions)

Add placeholders in ProtocolParams (see 209_params_and_governance.md):
- `enable_vaults` (bool, default false)
- `vault_allocation_cap_bps` (portion of PoolReserve eligible for lending via vault)
- `min_floor_reserve_ratio_bps` (reserve kept to honor floor before any lending allocation)
- `duration_buckets` (list of allowed durations)

## Integration Points (no‑op in MVP)

- PoolController: add a `rebalance_hook()` invoked by cranks (not on swap) to adjust `lending_allocation_q` within caps.
- Floor guard: during ratchet, compute `floor_min_q` using circulating supply and ensure `PoolReserve - lending_allocation_q >= floor_min_q`.
- SafetyController: monitor `lending_allocation_q` vs `vault_capacity_q`; on anomalies, set `enable_lending=false` and shrink allocations.

## Data and Events (planned)

- Pool events: LendingAllocationChanged { pool, old_q, new_q }, VaultCapacityObserved { pool, capacity_q }.
- Protocol events: VaultParamsChanged, VaultBucketAdded/Removed.

## Rollout Plan (later)

1. Deploy DurationVault (isolated), gated by `enable_vaults=false`.
2. Add PoolController `rebalance_hook()` calling vault view endpoints; dry‑run accounting only.
3. Enable small `vault_allocation_cap_bps` and a single duration bucket; observe.
4. Add borrow flows with strict LTV caps and liquidation rules (outside MVP scope here).

