# Verification and Policy

This document specifies the trust‑minimized verification rules for a single provider and the policy controls the chain enforces for updates, fees, and safety limits. The verification system implements cryptographic proofs and economic bounds to maintain security while enabling efficient off-chain computation of complex market physics.

## Symbols

- $S, T, L$: Domain value functions; $\tau$: protocol buffer.
- $w_{S,i}, w_{T,d}, w_{L,i}$: Component weights within domains (each set sums to 1).
- $I_S, I_T, I_L$: Index sets — $I_S=\{a,b\}$, $I_T$ = duration buckets, $I_L=\{\text{long},\text{short}\}$.
- $\kappa$: Price‑improvement clamp; $\eta$: rebate participation fraction.
- $\alpha$: Safety coefficient for leverage bounds.
- $D_{\text{TWAP}}$: Depth over TWAP window; $W_{\text{length}}$: window length factor; $L_{\text{notional}}$: leverage notional.
- $\sigma_{\min}, \sigma_{\max}$: Volatility bounds (annualized).
- Policy caps: `MAX_SURCHARGE_BPS`, `MAX_INSTANTANEOUS_FEE`, `REBATE_CAP`, `MAX_FEE` (global fee bound in commitments).
- Sequence numbers: strictly increasing per update; timestamps subject to freshness and cadence limits.

## Trust-Minimized Provider Architecture

### Single Provider with Cryptographic Constraints

The system employs one designated provider for efficiency while maintaining security through comprehensive verification. The provider computes complex market physics off-chain and posts compact updates that are cryptographically verified on-chain.

### Mandatory Validation Conditions

The chain accepts an update only if it satisfies *all* of the following conditions:

1. **Temporal freshness**: Within configured staleness window
2. **Update cadence**: Respects minimum interval between updates
3. **Bounded volatility**: Rate-of-change caps prevent manipulation
4. **Monotonic sequencing**: Strictly increasing sequence numbers
5. **Economic limits**: Fee and rebate caps enforced
6. **Cryptographic integrity**: Valid signatures and proofs
7. **Conservation compliance**: Updates respect thermodynamic constraints

Atomic rejection: Updates failing *any* single check are immediately rejected with no partial application.

## Freshness and Frequency

### Update Requirements
Each update includes a timestamp. The chain enforces maximum age limits to prevent stale data from affecting pricing and minimum intervals to avoid excessive parameter churn between updates.

### Validation Logic
Checks for staleness versus now and enforces minimum interval since the last accepted update.
```rust
fn validate_timing(
    update_timestamp: i64,
    current_time: i64,
    last_update_time: i64,
    max_staleness: i64,
    min_interval: i64,
) -> bool {
    let age = current_time - update_timestamp;              // staleness vs now
    let interval = update_timestamp - last_update_time;     // cadence vs last
    
    age <= max_staleness && interval >= min_interval
}
```

## Rate‑of‑Change and Sequence

### Change Limits
Per‑update change in posted quantities is capped to prevent market manipulation through excessive parameter jumps, system instability from rapid oscillations, and economic attacks via coordinated parameter abuse.

### Sequence Enforcement
Sequence numbers must increase strictly to prevent replay attacks through resubmission of old updates, out‑of‑order application via non-monotonic updates, and double spending from multiple applications of the same update.

Any violation results in immediate rejection.

## Fee and Rebate Caps

### Instantaneous Fee Limits
- **Surcharge cap**: Maximum dynamic surcharge per transaction
- **Total fee cap**: Combined base fee and surcharge limit

### Rebate Constraints
Rebates are limited by:
- **Per‑transaction limits**: Maximum rebate per single transaction
- **Per‑epoch limits**: Total rebate budget per epoch
- **Buffer availability**: Available $\tau$ buffer funds
- **Price improvement fraction**: Never exceed fraction $\kappa$ of measured improvement

### Funding Limits
- **Daily funding cap**: Hard limit on maximum daily funding rate
- **Anti‑manipulation**: Prevents excessive funding rate manipulation

## Cryptographic Commitment Architecture

The system supports dual verification modes balancing computation cost with flexibility:

### Mode A: Deterministic Recomputation
Full transparency approach where the chain recomputes work values directly:

- **Input data**: Posted scalars $(S, T, L)$, domain weights, trade parameters
- **On-chain calculation**: Direct application of potential formulas
- **Verification**: Deterministic computation with identical results
- **Advantages**: Maximum transparency, no trust assumptions
- **Trade-offs**: Higher gas costs for complex multi-segment trades

### Mode B: Cryptographic Commitments
Scalable verification using cryptographic proofs for complex calculations:

#### Commitment Structure
1. **Local approximations**: Quadratic approximations for each market segment
2. **Merkle tree construction**: Segments organized in authenticated data structure
3. **Root commitment**: Merkle root posted on-chain as commitment
4. **Inclusion proofs**: Cryptographic proofs for segments used in pricing

#### Verification Process
For each trade, the system verifies:
1. **Inclusion validity**: Merkle proofs authenticate used segments
2. **Bounded accuracy**: Global envelopes ensure conservative safety
3. **Economic constraints**: Fee and rebate limits enforced regardless of local calculation

### Mathematical Verification Framework

For Mode B commitments, the chain enforces:
$$\text{fee} \leq f_{\text{global}}(\text{bounds}, \text{proof}) \leq \text{MAX\_FEE}$$
$$\text{rebate} \leq r_{\text{global}}(\text{bounds}, \text{proof}) \leq \text{REBATE\_CAP}$$

Where global bound functions provide conservative upper limits that are always safe even if local approximations contain errors.

### Commitment Lifecycle Management
Validates commitment updates via signature, freshness, approximation bounds, and state consistency checks.
```rust
// Commitment update verification
struct CommitmentUpdate {
    merkle_root: [u8; 32],
    segment_count: u16,
    max_approximation_error: u64,
    valid_until: i64,
    signature: [u8; 64],
}

fn verify_commitment_update(
    update: &CommitmentUpdate,
    provider: &Pubkey,
    current_state: &MarketState,
) -> Result<()> {
    // 1. Signature verification
    verify_provider_signature(&update, provider)?;
    
    // 2. Freshness check
    require!(update.valid_until > current_timestamp(), FreshnessError);
    
    // 3. Approximation quality bounds
    require!(
        update.max_approximation_error <= MAX_ALLOWED_ERROR,
        AccuracyError
    );
    
    // 4. State consistency
    verify_state_consistency(current_state, &update)?;
    
    Ok(())
}
```

## Hysteresis Controller

### Off‑Chain Computation
Base fee changes are produced off‑chain using:
- **Banded hysteresis rule**: Prevents oscillation
- **EWMA smoothing**: Exponentially weighted moving average

### On‑Chain Verification
The chain verifies posted changes respect:
- **Minimum interval**: Between fee adjustments
- **Rate‑of‑change cap**: Maximum change per update  
- **Absolute fee caps**: Global minimum/maximum fee limits

## TWAP Safety and Leverage Bounds

### Price Input Requirements
Price inputs are subject to:
- **Minimum window length**: Required TWAP calculation period
- **Observation count**: Minimum number of price observations
- **Confidence thresholds**: Statistical confidence requirements

### Leverage Controls
- **Notional bounds**: Leverage notional per epoch bounded relative to depth
- **TWAP window depth**: Calculated over TWAP window period
- **Anti‑ping‑pong clamps**: Prevent window gaming attacks

### Mathematical Constraint
$$L_{\text{notional}} \leq \alpha \cdot D_{\text{TWAP}} \cdot W_{\text{length}}$$

where:
- $L_{\text{notional}}$: Leverage notional amount
- $D_{\text{TWAP}}$: Depth over TWAP window
- $W_{\text{length}}$: Window length factor
- $\alpha$: Safety coefficient (typically 0.1-0.5 based on market conditions)

### Specific Parameter Bounds and Values

#### Rate of Change Limits
- **Spot dimension**: Maximum 5% change per update
- **Time dimension**: Maximum 2% change per update  
- **Leverage dimension**: Maximum 10% change per update
- **Update frequency**: Minimum 30 seconds between updates

#### Fee and Rebate Parameters
- **Maximum surcharge**: 200 basis points (2.0%)
- **Maximum instantaneous fee**: 500 basis points (5.0%)
- **Maximum rebate rate**: 50 basis points (0.5%)
- **Rebate participation η**: 50% (normal), 0% (fallback)
- **Price improvement κ**: 50% (normal), 25% (fallback)

#### Risk Parameter Bounds
- **Volatility bounds**: $\sigma_{\min} = 0.05$, $\sigma_{\max} = 2.0$ (5%-200% annualized)
- **EWMA half-life**: 1-6 hours for smoothing
- **Confidence threshold**: Minimum 95% for TWAP calculations
- **Minimum observations**: 10 price points per TWAP window

## Fallback Behavior

### Entry Conditions
System enters fallback when no fresh update is available because updates exceed the staleness limit, an invalid update is received that fails validation checks, or the provider becomes unavailable with no updates received within the required interval.

### Fallback Mode Operations
During fallback mode, the system operates with base fees only as dynamic surcharges are disabled, rebates are completely disabled with no rebate calculations performed, an optional fixed spread may be applied as a conservative measure, and tighter clamps enforce more restrictive safety limits.

### Exit Conditions
System exits fallback when:
1. **Fresh, valid update accepted**: Passes all validation checks
2. **Minimum dwell time elapsed**: Prevents rapid mode switching

## Implementation Examples

### Update Rejection Logic
Rejects updates that are stale, too frequent, violate rate‑of‑change caps, or have non‑monotonic sequence numbers.
```rust
fn should_reject_update(
    update: &Update,
    current_time: i64,
    last_update: &LastUpdate,
    config: &PolicyConfig,
) -> bool {
    let stale = (current_time - update.timestamp) > config.max_staleness;
    let too_frequent = (current_time - last_update.time) < config.min_interval;
    let roc_exceeds = exceeds_rate_of_change_cap(update, last_update, config);
    let seq_invalid = update.sequence <= last_update.sequence;
    
    stale || too_frequent || roc_exceeds || seq_invalid
}
```

### Rate of Change Validation
Enforces per‑update caps on changes to posted S, T, and L scalars.
```rust
fn exceeds_rate_of_change_cap(
    new_update: &Update,
    last_update: &LastUpdate,
    config: &PolicyConfig,
) -> bool {
    let s_change = (new_update.S / last_update.S - 1.0).abs();
    let t_change = (new_update.T / last_update.T - 1.0).abs();
    let l_change = (new_update.L / last_update.L - 1.0).abs();
    
    s_change > config.max_s_change_pct ||
    t_change > config.max_t_change_pct ||
    l_change > config.max_l_change_pct
}
```

## Security Model and Attack Resistance

### Trust Minimization Architecture

The verification system achieves security through layered constraints rather than provider trust by cryptographically anchoring all commitments to chain state, applying economic bounds that limit maximum damage from malicious updates, enforcing temporal constraints that prevent replay and staleness attacks, performing mathematical verification through on-chain validation of critical computations, and implementing redundant safety nets where multiple independent checks must all pass.

### Comprehensive Attack Mitigation

#### Data Integrity Attacks
- **Stale data injection**: Freshness windows with strict timestamp validation
- **Replay attacks**: Monotonic sequence numbers with gap detection
- **Man-in-the-middle**: Cryptographic signatures from authorized provider keys
- **State corruption**: Merkle commitments with inclusion proof verification

#### Economic Manipulation Attacks
- **Fee extraction**: Hard caps on maximum fees regardless of claimed work
- **Rebate farming**: Multi-layer rebate caps with budget tracking and \u03ba clamps
- **Rate manipulation**: Bounded rate-of-change limits on all posted parameters
- **Flash loan attacks**: Minimum update intervals prevent intra-block manipulation

#### System Stability Attacks  
- **Parameter whipsawing**: Hysteresis controls with smoothed adjustments
- **Liquidity draining**: Leverage bounds relative to available depth
- **Oracle manipulation**: TWAP requirements with confidence thresholds
- **Circuit overload**: Fallback modes with conservative defaults

### Formal Security Properties

The system maintains these invariants under all conditions:

1. **Bounded economic exposure**: Maximum fee and rebate exposure per transaction and epoch
2. **Conservation preservation**: No update can violate thermodynamic conservation laws
3. **Temporal ordering**: All state transitions respect causal ordering via sequence numbers  
4. **Cryptographic integrity**: All commitments remain unforgeable and verifiable
5. **Graceful degradation**: System continues operating safely even with provider failures
