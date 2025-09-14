# Safety Controller (MVP)

The Safety Controller coordinates degrade/pause behavior across protocol and pools to prevent cascading failures and protect solvency.

## Responsibilities

- Track health for: protocol oracle, pool oracles (GTWAP), liquidity/volatility, solvency ratios.
- Enforce global/pool/feature pauses and degraded behavior.
- Rate limit sensitive actions (oracle updates, after-swap path, JIT ops).

## Health Model

HealthStatus per component:
- is_healthy (bool), last_healthy_slot, error_count, degradation_level (0=ok, 1–3=degraded, 4+=critical)

## Actions Matrix (MVP)

- GTWAP stale or insufficient cardinality:
  - Disable rebates (direction bonus), increase impact floor to `impact_floor_bps`
  - Allow swaps; emit Degraded(GTWAP)

- Protocol oracle stale/unhealthy:
  - Pause `exit_feelssol` redemptions; allow swaps
  - Emit Degraded(ReserveOracle)

- Protocol depeg detected (JitoSOL/SOL market rate deviates > threshold):
  - Immediate circuit breaker: pause `exit_feelssol` redemptions; allow swaps
  - Condition: DEX TWAP deviation > `depeg_threshold_bps` for `depeg_required_obs` consecutive observations
  - Emit SafetyPaused(scope=Redemptions) and Degraded(ReserveOracle)

- Volatility spike (ticks per second / price impact > threshold):
  - Raise min_total_fee_bps temporarily, cap max rebate magnitude

### Indicative thresholds (MVP)

- GTWAP stale if `now_slot - last_observation_slot > staleness_threshold_slots` (e.g., 150 slots)
- Insufficient cardinality if `observation_cardinality < MIN_CARDINALITY` (e.g., 8 of 12)
- Volatility spike if `ticks_moved / seconds > V_TPS_THRESHOLD` (conservative value; governance‑tuned)

- Critical invariant breach (e.g., floor ask below safe tick):
  - Pool pause (disable swaps/JIT) until resolved; emit Paused(Pool)

## Integration Points

- PoolController::after_swap calls `safety.observe(...)` and reads current degrade flags.
- Protocol mint/redeem paths (`enter_feelssol`,`exit_feelssol`) query safety before proceeding.

## Events

- SafetyDegraded(component, level), SafetyPaused(scope), SafetyResumed(scope)

## Cool-Off Behavior

To prevent flapping between degraded and healthy states, apply a cool‑off timer before clearing degraded flags (e.g., remain degraded for at least 60–120 seconds after the last unhealthy signal), tunable via governance.
