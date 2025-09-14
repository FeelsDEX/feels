# Parameters, Governance, and Pausing

This document defines protocol/pool parameters, governance controls, and pausing/degrade scopes for the MVP.

## ProtocolParams (global)

Core fields (examples; defaults TBD by governance):
- fee_split_bps: { lps, pool_reserve, pool_buffer, protocol_treasury, creator_base }
- min_total_fee_bps, max_total_fee_bps, impact_floor_bps
- floor_buffer_ticks, ratchet_cooldown_slots
- oracle_safety_buffer_bps (protocol oracle)
- feature flags: enable_momentum=false, enable_jit=true (MVP)

Suggested MVP defaults (subject to governance):

```
fee_split_bps = {
  lps: 4500,
  pool_reserve: 2500,
  pool_buffer: 2000,
  protocol_treasury: 800,
  creator_base: 200,
}

min_total_fee_bps = 20
max_total_fee_bps = 150
impact_floor_bps = 10

floor_buffer_ticks = 100
ratchet_cooldown_slots = 1800

oracle_safety_buffer_bps = 50

feature_flags = { enable_momentum: false, enable_jit: true }

// Dynamic fees warmup (both must be satisfied)
S_MIN_SLOTS = 2400      // e.g., ~20 minutes at 0.5s/slot (tune as needed)
MIN_WARMUP_TRADES = 150  // minimum trades before enabling rebates/equilibrium
```

Validation rules (flat, explicit params):
- Sum(fee_split_bps) == 10_000 (hard assert)
 - creator_base <= 500 bps (governance cap)
 - creator_claim_min_amount: u64 (optional) — minimum accrued amount before a creator can claim, to avoid micro‑claims (e.g., ≥ 1e6 lamports)

// JIT v0 (MVP) defaults (conservative)
jit_v0 = {
  base_spread_ticks: 3,
  range_ticks: 1,
  dev_clamp_ticks: 80,
  max_per_swap_q_bps_of_buffer: 10,  // 0.10% of PoolBuffer
  max_per_slot_q_bps_of_buffer: 30,   // 0.30% of PoolBuffer per slot
  cooldown_slots: 5,
  ask_cooldown_slots: 10,
}

// Launch presets (per-pool; optional, time-limited boost)
launch_window_hours = 72
launch_presets = {
  pool_buffer_split_bps: 2200,   // temporary +200 bps to PoolBuffer for bootstrapping
  creator_base_bps: 100,         // temporary reduce creator share during launch
  jit_v0: {
    max_per_swap_q_bps_of_buffer: 20,  // temporarily double budgets
    max_per_slot_q_bps_of_buffer: 60,
    base_spread_ticks: 3,
  }
}

Governance:
- Multisig with optional timelock; ParamChanged events emitted.
- Upgrade policy: program upgrade authority custody requirements documented.

## PoolParams (override)

Optional per-pool overrides (inherit ProtocolParams by default):
- base_fee_bps, tick_spacing
- normal_bias params (fees), floor buffer override
- min_twap_duration, min_cardinality (pool oracle)
- staleness_threshold_slots

Note: Hierarchical “CoreParameters → derived params” is intentionally omitted in MVP for transparency and predictability. In Phase 2, we may introduce optional off‑chain “presets” that generate flat param bundles for governance to approve, preserving clarity while easing tuning.

## Pausing / Degrade Scopes

- Global pause (protocol): disables swap, JIT, mint/redeem.
- Pool pause: disables swap and JIT for a single pool.
- FeelsSOL mint/redeem pause: disables `enter_feelssol`/`exit_feelssol` only.

SafetyController matrix (examples):
- GTWAP stale → disable rebates, raise impact floor, allow swaps; event Degraded(GTWAP).
- Protocol oracle stale → pause `exit_feelssol`, allow swaps; event Degraded(ReserveOracle).
- Volatility spike → temporarily raise min fee bps and/or cap rebates.

## Rate Limiting

- Per-pool per-slot swap limiter (volume/ticks crossed).
- Oracle update limiter (min slots between updates).
- JIT budgets per swap/slot (MVP: JIT v0 enabled by default).

## Future (Vaults & Lending) Parameters

Placeholders to ease future rollout (no effect in MVP unless enabled):
- `enable_vaults: bool` (default false)
- `vault_allocation_cap_bps: u16` (cap of PoolReserve eligible for allocation)
- `min_floor_reserve_ratio_bps: u16` (reserve retained to honor floor)
- `duration_buckets: [u32; N]` (allowed lock durations in seconds)

## Protocol Oracle & Circuit Breaker (MVP)

Add parameters to harden redemptions against de‑pegs:
- `depeg_threshold_bps: u16` (e.g., 100–200 bps)
- `depeg_required_obs: u8` (e.g., 3–5 consecutive TWAP observations)
- `dex_twap_window_secs: u32` (e.g., 1800 for 30 minutes)
- `dex_twap_min_liquidity: u64` (filter thin markets)

Behavior: If DEX TWAP deviates from protocol native rate by more than `depeg_threshold_bps` for `depeg_required_obs`, SafetyController pauses `exit_feelssol`. Swaps remain enabled.

## DEX TWAP Whitelist (MVP)

Governance maintains the list of venues and markets used by the protocol oracle:
- `dex_whitelist`: array of entries with fields:
  - `venue_program: Pubkey` (DEX program ID)
  - `pair`: mints `{ mint_a: Pubkey, mint_b: Pubkey }` (e.g., JitoSOL/SOL)
  - `pool_pubkey: Pubkey` (optional, for concentrated pools)

Rules:
- Only whitelisted venues/pairs contribute to the protocol oracle DEX TWAP.
- Updates are governance‑gated and emit `ParamChanged` events.
