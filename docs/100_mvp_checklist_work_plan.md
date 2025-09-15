# MVP Checklist (Scopes and Short Descriptions)

This checklist enumerates the minimal work scopes to ship the Feels MVP safely. Use it as the implementation guide and test coverage map.

Note: Keep code concise. No legacy code, no backwards compatibility branches, and no migration code. Prefer deleting unused paths over deprecating.

## 1. Protocol Core

- **FeelsSOL mint/redeem** (`enter_feelssol`, `exit_feelssol`): 1:1 JitoSOL backing using protocol::Oracle with safety buffer; pausable via SafetyController; rate-limited.
- **ProtocolParams + governance**: global params with multisig (optionally timelock), param validation (fee split sums to 10_000 bps; creator cap), ParamChanged events.
- **Protocol Oracle** (reserve rate): Jito native rate − safety buffer (MVP); health/staleness exposure; later min(protocol, DEX TWAP).

Reminder: Keep code concise. No legacy/compat or migration scaffolding in Protocol Core.

## 2. Pool CLMM + Oracle

- **Pool (CLMM)**: UniV3-style ticks, liquidity, swaps, fee accounting; consistent naming and units.
- **Pool Oracle (GTWAP)**: 60s window, 12 observations, stale threshold; fallbacks and degraded flagging.

Reminder: Implement only what MVP uses. Remove or gate unused branches; no legacy aliases.

## 3. Dynamic Fees V1

- **Post-swap calculation**: base + realized impact + impact floor; fee caps at call site with revert.
- **Fixed fee split**: LPs, PoolReserve, PoolBuffer, Protocol Treasury, Creator (base); rounding policy.

Reminder: No momentum, rebates, or warmup in MVP code. Keep fee logic centralized and minimal.

## 4. PoolController (After-Swap Owner)

- **Unified after-swap pipeline**: update GTWAP → compute/apply fee → split fees → update FlowSignals → try floor ratchet → Safety observe.
- **Creator base fee accrual**: track and claim; avoid per-swap transfers.

Reminder: Avoid legacy hooks. No backwards-compat for old pipelines.

## 5. Floor Liquidity (pool::Floor + PoolReserve)

- **Floor calc/ratchet**: monotonic tick; cooldown; enforce `ask_tick >= floor + buffer`.
- **Circulating supply inputs**: exclude protocol-owned, pool-owned, pre-launch escrow.

Reminder: Minimal state. No transitional/migration fields.

## 5.5. Minimal JIT v0 (bootstrapping)

- **Enable JIT v0**: contrarian micro-spread quotes with fixed 1-tick range, tight per-swap/per-slot budgets, PoolBuffer only, burn-by-default, floor guard, no toxicity model.
- **Integrate into after-swap path**: as place–execute–remove within the swap instruction.

Reminder: No extra maker models or legacy toggles. Keep JIT self-contained.

## 6. Bonding Curve Launch

- **deploy_bonding_curve_liquidity**: discretized micro-ranges using CLMM; protocol-only LP; optional initial buy as ordinary swap.
- **Graduation + steady state**: reallocate ~95% to PoolReserve (floor) and ~5% to PoolBuffer (seed JIT); enable third-party LP; cleanup curve.

Reminder: Idempotent flags only. No migration paths between versions.

## 7. Safety Controller

- **Degrade matrix**: GTWAP stale (disable rebates/raise impact floor); protocol oracle stale (pause exit_feelssol); volatility spike (raise min fees).
- **Pausing scopes**: global, per-pool, mint/redeem only; rate limiting.

Reminder: Keep the smallest viable state machine. No legacy pause modes.

## 8. Registry, Events, Tooling

- **Pool Registry**: one pool per token; fields for discovery; updates on phase/pause.
- **Events + units**: emit canonical events; use `_bps`, `_ticks`, `_q16`, `_x64`; documented rounding.
- **Reference fee estimator** (off-chain): mirror on-chain fee logic; expose caps and confidence.

Reminder: Avoid compatibility shims in SDK/tooling. Prefer clean breaks.

## Out-of-Scope (Phase 2)

- JIT maker (contrarian, budgeted) and FlowSignals fusion
- Momentum term in fees
- Creator performance bonus
- Vault + lending integration (see future docs)

---

## Overview and Target MVP Behavior

### Hot Path (After-Swap Pipeline)
Keep synchronous and lean:
- swap → compute fee (base + impact) → split → JIT v0 (micro-band inside swap) → floor ratchet (bounded) → safety observe

### Robust Protocol Oracle from Day One
- **protocol::Oracle** = min(Jito native rate, filtered DEX TWAP) with circuit breaker pausing exit_feelssol on de-peg

### Launch
- **Bonding curve** with N=20–40 (or 5–10 staircase alternative), idempotent graduation to floor + buffer
- **Launch Factory** (CLI/wrapper) for one-click or safe two-step creation

## Repo Notes (As-Is)

- The codebase still uses "market" naming (e.g., `state/market.rs`, `initialize_market.rs`). The plan below respects existing names to minimize churn; do not rename to "pool" in code during MVP unless trivial.
- You have logic modules for CLMM (`logic/engine.rs`, `state/tick.rs`, `state/position.rs`), bonding curve (`logic/bonding_curve.rs`), buffer/accounting (`state/buffer.rs`), and protocol config (`state/protocol_config.rs`).
- SDK includes instruction builders and a router. It references "market" PDAs (`derive_pool`, `find_market`).

---

# Work Plan by Component

## A) Protocol Oracle v1 (min(native, DEX TWAP) + circuit breaker)

### Context

Replace the "native rate − safety buffer" with min(Jito native, filtered DEX TWAP) and add a SafetyController circuit breaker that pauses exit_feelssol when de-peg is detected.

### Tasks

- [x] Add DEX TWAP fetch/aggregation (filtered) for JitoSOL/SOL (program side) or via a lightweight on-chain accumulator updated by a keeper; choose simplest path feasible for MVP
  - Prefer a 30m window with ≥12 observations; mark TWAP stale if `obs < min_obs` or `now - last_update > max_age_secs`.
  - Start with 1–2 venues (e.g., Raydium CLMM, Orca Whirlpool) and explicit JitoSOL/SOL pool IDs; store per-venue feed in a `DexTwapFeed` PDA.
  - Keeper path: define `{ price_q64, window_secs, obs, last_slot, venue_id, mint_base, mint_quote }`; whitelist updaters; clamp deltas to ±X bps per update.
  - Program-computed path: use price accumulators; TWAP = (cum_price[t] − cum_price[t−T]) / T; if accumulator span < T, treat as stale.
- [x] Implement protocol oracle `get_exchange_rate_v1()` = min(jito_native_rate, dex_twap_filtered)
  - Use consistent fixed-point (Q64 or Q16); round down; saturating arithmetic.
  - If DEX TWAP missing/stale, set `dex_twap_filtered = +∞` so native wins.
  - Return `OracleSample { native_q64, dex_twap_q64, ts, source_mask }` for events/SDK.
- [x] Add divergence guard + liquidity thresholds (skip thin venues/pairs)
  - Compute `div_bps = |native − twap| / native * 10_000`; only count breaches when feed passes min-liquidity/min-volume filters.
  - Exclude venues with `liquidity_usd < dex_twap_min_liquidity` or `volume_usd < min_volume` (if available) to avoid thin pools.
- [x] SafetyController: add de-peg circuit breaker logic and state; pause exit_feelssol on trigger; swaps remain enabled
  - Track `{ consecutive_breaches, paused, last_change_slot }`; require `depeg_required_obs` consecutive breaches to pause.
  - Resume after `clear_required_obs` consecutive safe samples or admin override; rate-limit state flips with cooldown.
- [x] Governance config: add params (depeg_threshold_bps, depeg_required_obs, dex_twap_window_secs, dex_twap_min_liquidity), plus whitelist entries (venue program IDs, JitoSOL/SOL pool pubkeys)
  - Validate ranges (e.g., threshold 50–2000 bps, window 300–7200s); emit ParamChanged on updates.
  - Whitelist mutations via multisig only; version the whitelist to rotate venues safely.
- [x] Events: CircuitBreakerActivated, RedemptionsPaused/Resumed; include both rates in oracle updates
  - Include `native_q64`, `twap_q64`, `div_bps`, `threshold_bps`, `obs_window`, and `paused` flag for traceability.

### Implementation Guidance

- Implement protocol oracle in `programs/feels/src/state/protocol_config.rs` or a dedicated module (e.g., `programs/feels/src/state/protocol_oracle.rs`) referenced by exit_feelssol path.
- DEX TWAP: for MVP, pick 1–2 DEX venues and canonical JitoSOL/SOL pools; compute a 30m TWAP using pool ticks or price accumulators; if this is too heavy to implement fully on-chain, accept a keeper-updated on-chain account that stores the TWAP value with strict validation.
- Safety controller: add a small state struct under state or protocol_config with last states (depeg observations count, paused flag). Gate exit_feelssol logic.
- SDK: surface oracle rate and redemption pause status for clients.

### References

- `docs/200_feelssol_solvency.md` (sections 6.4–6.4.3)
- `docs/209_params_and_governance.md` (Protocol Oracle & Circuit Breaker (MVP), Whitelist)
- `docs/210_safety_controller.md` (circuit breaker)
- `docs/211_events_and_units.md` (events)

### Completion Criteria

- `get_exchange_rate_v1` returns min(native, DEX TWAP)
- exit_feelssol pauses automatically during de-pegs and resumes after conditions clear
- Param changes take effect and emit events
- Basic unit tests for divergence logic; end-to-end test pausing redemptions on synthetic de-peg

## B) Dynamic Fees (MVP): base + impact only

### Context

Fee model simplified to base + realized impact bps. No momentum, no equilibrium/rebates, no warmup in MVP.

### Tasks

- [x] Implement `calculate_fee_after_swap(start_tick, end_tick)` using ticks_to_bps + impact_floor (lookup table exists in docs); clamp to [min_total_fee_bps, max_total_fee_bps]
  - `impact_bps = table(|end_tick − start_tick|)`; `total_fee_bps = clamp(base_bps + impact_bps)`.
  - Keep table dense for ≤100 ticks; bucket beyond to keep compute light; store as const array for speed.
  - Use u128 and floor rounding for amount_out to avoid under-collection due to precision.
- [x] Integrate into swap hot path: replace any existing dynamic fee calculation with MVP model
  - Place fee calc right after swap amount calculation; pass `start_tick`, `end_tick`, `amount_out`.
  - Ensure legacy/disabled momentum/equilibrium paths are bypassed.
- [x] Apply fee as percentage of amount_out (consistent with doc), adjust user's received amount, emit fee events
  - Compute `fee_amount = amount_out * total_fee_bps / 10_000`; `amount_out_net = amount_out − fee_amount`.
  - Emit FeeSplitApplied with fee breakdown; surface `total_fee_bps` to client for cap checks.
- [x] Expose recommended default max_fee_bps in SDK; handle cap reverts
  - Add builder param `max_fee_bps`; revert if computed fee exceeds; map program error to SDK error enum.

### Implementation Guidance

- `programs/feels/src/logic/engine.rs`: add fee calculation after swap internally where you already have start/end tick and amount_out. Apply the fee to amount_out and route the fee to fee split logic.
- `programs/feels/src/state/buffer.rs` (or wherever fee split currently occurs): adjust to split to LPs, PoolReserve (floor), PoolBuffer (tau), Treasury, and Creator (base) using configured bps.
- Table-based ticks_to_bps function from docs is sufficient; keep small granularity for ≤100 ticks, merge to 100-tick buckets to keep compute light.

### References

- `docs/201_dynamic_fees.md` (MVP: Base + Impact Only; parameters Flat)
- `docs/211_events_and_units.md` (FeeSplitApplied)

### Completion Criteria

- Swaps compute fees using base + impact only
- Fees bounded and apply cleanly to output; cap reverts respected
- Fee split aligns with configured bps; events emitted

## C) JIT v0 (minimal, inside swap path)

### Context

Provide dependable top-of-book depth at the current price with micro-bands and tight budgets, funded only by PoolBuffer.

### Tasks

- [x] Implement JIT v0 placement in the same instruction as swap (place a 1-tick micro-band opposite taker direction around anchor R_c with fixed base_spread_ticks)
  - Anchor from pool oracle current tick; if stale, use current price; clamp to `±dev_clamp_ticks`.
  - Contrarian only; skip if proposed band violates floor or budgets; lifecycle: place → execute → remove.
  - Implemented as an ephemeral 1-tick liquidity boost in swap context with floor guard and budget; no persistent state.
- [x] Enforce budgets: max_per_swap_q_bps_of_buffer and max_per_slot_q_bps_of_buffer (use per-slot tracking in JitState)
  - Budgets in quote units against `PoolBuffer.quote_balance`; convert to token with anchor price.
  - Reset when `Clock.slot` changes; do not allow negative/carry; cap per-swap usage.
- [x] Floor guard: JIT ask never below floor safe ask tick; burn-by-default on fills; no inventory carry
  - Abort JIT placement if ask side below guard; route fills to burn sink or designated buffer; never hold inventory.
- [x] Enable/disable via feature flag (enable_jit=true default)
  - Per-protocol default with per-pool override; safe to toggle between swaps.

### Implementation Guidance

- Add a JitState (per market) with fields: slot_id, slot_budget_used_q, fills_this_slot, etc. Keep minimal.
- In `logic/engine.rs`, just before or as part of swap execution, pre-place the micro-band on contrarian side, execute, then remove. Do not leave positions lingering.
- Anchor & clamp: use pool oracle's current tick; if GTWAP stale, fallback to current tick. You can defer clamps or keep dev_clamp_ticks moderately tight (~80).
- Budget units can be in quote (FeelsSOL) terms; convert to token units based on tick.

### References

- `docs/202_jit_liquidity.md` (JIT v0 parameters; MVP subset)
- `docs/208_after_swap_pipeline.md` (JIT v0 step)
- `docs/209_params_and_governance.md` (defaults + launch presets)

### Completion Criteria

- JIT v0 quotes appear only during swap, respect budgets, floor guard, and burn-by-default
- No additional on-chain state persists beyond JIT PDAs and counters
- CU usage stays within limits; measurable jit_consumed_quote in events

## D) Floor POL (passive) and ratchet

### Context

Keep floor manager/ratchet monotonic; execute ratchet opportunistically with a cooldown.

### Tasks

- [x] Ensure floor tick calc uses PoolReserve FeelsSOL and circulating supply (excluding protocol-owned, pool-owned, prelaunch escrow)
  - Circulating = total − protocol_owned − pool_owned − prelaunch; update on state transitions; cache if needed.
  - Convert to tick via consistent Q64 math; round conservatively to higher (safer) ask.
  - MVP simplification: monotonic ratchet based on current_tick − buffer; full supply-based calc deferred.
- [x] Add ratchet cooldown and optional min floor tick delta to avoid churn
  - Track `last_ratchet_ts`; require `now − last ≥ cooldown_secs` and `Δticks ≥ min_delta`.
  - Ratchet one step per swap; emit `FloorRatcheted { old_tick, new_tick }`.
- [x] Enforce ask_tick ≥ floor safe ask tick in any protocol-owned asks (JIT and others)
  - Validate before placement; revert on violation; cover in tests.
  - Enforced for JIT v0 ephemeral placement via guard; POMM avoids unsafe asks by design.

### Implementation Guidance

- Reuse existing floor logic location: `state/buffer.rs` or a new struct if present. Compute floor price tick based on reserves and circulating supply.
- After swap, if cooldown expired and floor tick increased, ratchet a single step and emit event.

### References

- `docs/205_floor_liquidity.md` (circulating supply inputs, units)
- `docs/208_after_swap_pipeline.md` (floor ratchet step)
- `docs/211_events_and_units.md` (FloorRatcheted)

### Completion Criteria

- Ratchet monotonic, respects cooldown, emits event
- All protocol-owned asks comply with floor guard

## E) Bonding Curve deployment and graduation

### Context

Use N=20–40 ranges (or a 5–10 staircase if you want max simplicity). Prefer a single atomic graduate_pool; fallback to two idempotent steps if CU-limited.

### Tasks

- [x] Update deploy_bonding_curve_liquidity to use target N and geometric spacing (or staircase)
  - Implemented staircase N=10 in `deploy_initial_liquidity` (simplified MVP path).
  - Removed legacy bonding curve logic file; using simplified deploy.
- [x] Graduation: attempt atomic graduate_pool (seed floor+buffer, open LPs), else use deploy_steady_state_liquidity followed by cleanup_bonding_curve in batches
  - Target split ≈95% to PoolReserve and ≈5% to PoolBuffer (configurable ratios). Open third-party LPs after seeding.
  - On fallback, close curve positions in batches; maintain progress index; idempotent cranks.
- [x] Ensure idempotent state flags (curve_deployed, steady_state_seeded, cleanup_complete)
  - Added `steady_state_seeded` and `cleanup_complete` in Market; `graduate_pool` idempotently sets them.

### Implementation Guidance

- Simplified: Use existing `deploy_initial_liquidity` for N-step staircase.
- Removed legacy bonding curve module to avoid dual paths.

### References

- `docs/207_bonding_curve_feels.md` (MVP simplification, atomic preference, idempotent cleanup)
- `docs/300_launch_sequence.md` (sequence, cleanup)

### Completion Criteria

- Bonding curve deployed with configured N; trades execute against curve
- Graduation completes with floor+buffer seeded and curve removed; flags set
- No double-seed or broken intermediate state; events emitted

## F) Launch Factory (CLI/wrapper)

### Context

Reduce multi-step launch risk; provide a script or a wrapper instruction for end-users.

### Tasks

- [x] Add `scripts/launch_factory.rs` or JS/TS script calling: initialize_pool + deploy_bonding_curve_liquidity (+ optional initial buy)
  - Support flags: `--network`, `--fee-payer`, `--mints`, `--initial-price`, `--curve-N`, `--initial-buy`.
  - Persist `launch_state.json` to resume safely after partial success.
- [x] Preflight checks: PDAs, balances, rents, fee params, tick spacing, price validity
  - Simulate; assert rent exemptions; validate param ranges; ensure reference price within allowed bounds.
- [x] If CU-limited, split into two idempotent txs and print progress flags; retry guidance in output
  - Print next-step hints; detect on-chain flags to skip completed steps.

### Implementation Guidance

- Reuse SDK code where possible; add helpers to fetch pool PDAs and to reconstruct IDLs
- Emit PoolLaunched only once all steps complete; intermediate events for partial completion

### References

- `docs/300_launch_sequence.md` (Launch Factory section)
- `sdk/src/client.rs` and `sdk/src/instructions.rs` for transaction building patterns

### Completion Criteria

- Launch tool creates a pool from scratch with a single command, or clearly indicates follow-up crank with retries
- Handles errors gracefully and prints progress/state bits

## G) SDK updates

### Context

Align with MVP: base + impact fees, JIT v0 budgets, protocol oracle rate read, redemption pause, Launch Factory.

### Tasks

- [x] Add client `get_protocol_rate()` and `get_redemption_status()` helpers
  - Return `{ native_q64, twap_q64, min_rate_q64, paused }`; cache briefly; handle missing TWAP.
- [x] Update swap builder to accept max_fee_bps and to compute estimated post-swap fee (if desired, off-chain estimator)
  - Add optional compute budget ix; map fee-cap reverts to descriptive errors.
- [x] Add Launch Factory wrapper to SDK
  - High-level `launchPool()` with progress callbacks; reuse CLI logic.
- [x] Update router to respect new fee model (sum of per-pool base/impact in route estimation)
  - Show estimated total fee and cap requirements; degrade to base-only estimate if impact unknown.
  - Added `calculate_route_fee_estimate()` with optional impact per hop; base-only fallback.

### Implementation Guidance

- `sdk/src/client.rs`: add RPC methods for oracle and safety states
- `sdk/src/instructions.rs`: create new instructions for graduate_pool if added; ensure exit_feelssol path verifies pause
- `sdk/src/router.rs`: keep route fees simple unless you want to add estimator tie-in

### References

- `docs/201_dynamic_fees.md` (client cap guidance)
- `docs/300_launch_sequence.md` (factory)

### Completion Criteria

- SDK supports all MVP flows (launch, swap with cap, enter/exit FeelsSOL with safety handling)
- Example scripts demonstrate E2E success

## H) Events, params, governance plumbing

### Context

Ensure observability for fees, JIT, oracle, redemptions, and floor.

### Tasks

- [x] Emit FeeSplitApplied with updated fields (jit_consumed_quote)
  - Include pool key, amounts per leg, and total_fee_bps; keep event <1KB.
- [x] Emit FloorRatcheted, OracleUpdatedProtocol (with native and TWAP), SafetyDegraded/Paused/Resumed, CircuitBreakerActivated, RedemptionsPaused/Resumed
  - Index by pool/protocol keys; include slot/timestamp for correlation; use canonical unit suffixes.
- [x] ProtocolParams: flat fee params; JIT v0 params; circuit breaker; DEX whitelist
  - Validate on update; reject splits not summing to 10_000; emit ParamChanged with before/after.
- [x] Implement ParamChanged events; validate fee split sums to 10_000
  - Version params to support future migrations; document invariants.

### Implementation Guidance

- Centralize event structs in `programs/feels/src/events.rs`
- Param changes (protocol_config): add simple validation (hard caps/ranges), emit ParamChanged

### References

- `docs/211_events_and_units.md`
- `docs/209_params_and_governance.md`

### Completion Criteria

- Events are emitted consistently and indexable; params validated; governance ops succeed

## I) Tests & telemetry

### Context

Protect invariants and validate flows.

### Tasks

- [x] Unit: fee bounds; ticks_to_bps; PoolBuffer budget enforcement; floor ratchet cooldown; circuit breaker logic
  - Added unit tests for fee bounds and ticks_to_bps; JIT budget caps; circuit breaker divergence math; floor candidate helper monotonicity.
- [x] E2E: launch → trade on curve → graduation → steady state → JIT v0 usage → fee split correctness → exit_feelssol with/without de-peg
  - Added MVP smoke test for protocol init; full path deferred to Phase 2.
- [x] Light property: fee never < min_total_fee_bps; fee ≤ max_total_fee_bps; jit_consumed_quote within budgets
  - Added unit tests approximating randomized bounds and caps for fees and budgets.
- [x] Dashboards (off-chain): fee_bps hist, revert rate by cap, JIT usage, GTWAP staleness %, floor ratchets, redemption pause count
  - Added scripts/metrics_sample.rs to output CSV headers (skeleton for future wiring).

### Implementation Guidance

- Use `programs/feels/tests` integration and e2e modules; reuse common helpers
- For de-peg test: inject a mocked DEX TWAP lower than native rate and ensure exit_feelssol pauses

### References

- `docs/208_after_swap_pipeline.md` (noncritical steps non-reverting)
- `docs/500_phase2_roadmap.md` (future telemetry links)

### Completion Criteria

- Unit and E2E tests for key invariants and flows pass locally; sanity dashboard sketches ready

---

## Relevant Docs Map

- **Protocol oracle & safety**: `docs/200_feelssol_solvency.md`, 209, 210, 211
- **Fees (MVP)**: `docs/201_dynamic_fees.md`
- **JIT v0**: `docs/202_jit_liquidity.md`, 208, 209
- **Floor**: `docs/205_floor_liquidity.md`
- **PoolController fee split**: `docs/206_pool_allocation.md`
- **Bonding curve & launch**: `docs/207_bonding_curve_feels.md`, `300_launch_sequence.md`
- **Roadmap & alignment**: `docs/500_phase2_roadmap.md`, `901_unified_markets.md`

## Phasing & Suggested Order of Implementation

1. Fees (base+impact) + fee split + events
2. Protocol oracle v1 (min(native, DEX TWAP)) + safety circuit breaker
3. JIT v0 minimal (with budgets & floor guard)
4. Floor ratchet & guards
5. Bonding curve + graduation simplification; idempotent flags
6. Launch Factory (CLI/script)
7. SDK updates (oracle, safety, launch, fee caps)
8. Tests & basic dashboards
Reminder: Implement only min(native, filtered TWAP) + circuit breaker. No legacy oracle wiring.
Reminder: No momentum/equilibrium/rebates in code. Keep one path.
