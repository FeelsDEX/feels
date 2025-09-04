# Feels Protocol: Mode A Implementation

## Current Implementation Status

The Feels Protocol currently implements **Mode A (Deterministic Recomputation)** for all thermodynamic calculations and fee determination. Mode B (Cryptographic Commitments) is designed in the architecture but not yet implemented on-chain.

## Mode A: What's Implemented

### Field Commitments
- Keepers post compact field state snapshots (`FieldCommitment`)
- Contains scalar values (S, T, L), domain weights, and fee parameters
- All calculations are deterministically recomputed on-chain

### On-Chain Verification
```rust
// Current flow:
1. Keeper posts FieldCommitment with scalars
2. Trades use these scalars to compute work W = V(P₂) - V(P₁)
3. Fees calculated as: Fee = W / Π_in (uphill) or Rebate = min(|W| / Π_out, κ × price_improvement)
4. All calculations happen on-chain with full transparency
```

### Verify-Apply Pattern
Field updates follow a strict verify-apply pattern:
- **No on-chain computation**: Fields are never calculated on-chain
- **Pre-computed updates**: Keepers compute updates off-chain
- **Verification only**: Chain verifies update constraints (rate limits, freshness, etc.)
- **Atomic application**: Verified updates applied atomically

### Key Components
- `FieldCommitment`: Contains Mode A data (scalars, weights, TWAPs)
- `calculate_path_work()`: Computes thermodynamic work along trading path
- `calculate_instantaneous_fee()`: Determines fees/rebates from work

## Mode B: Reserved for Future

The `FieldCommitment` struct includes optional Mode B fields:
- `root: [u8; 32]` - Merkle root for segment commitments
- `lipschitz_L: u64` - Global Lipschitz constant
- `curvature_bounds: (min, max)` - Curvature constraints

These fields are set to zero/empty in Mode A and reserved for future Mode B implementation.

## Why Mode A?

1. **Maximum Transparency**: All calculations verifiable on-chain
2. **No Trust Assumptions**: No reliance on keeper computation correctness
3. **Sufficient for Launch**: Current segment limits (20 per trade) manageable with Mode A
4. **Proven Security**: No cryptographic proof verification risks

## When Mode B?

Mode B will be considered when:
- Trade complexity exceeds efficient on-chain computation
- Gas optimization becomes critical at scale
- Cryptographic proof generation/verification is battle-tested

## Implementation Notes

### Current Limitations
- Max 20 segments per trade (10 per hop × 2 hops)
- All field updates use verify-apply pattern
- No dense gradient tables or micro-field approximations

### Future Mode B Requirements
1. Merkle tree construction for segment commitments
2. Inclusion proof verification logic
3. Global bound enforcement
4. Transition mechanism from Mode A to Mode B

## Summary

The Feels Protocol operates entirely in Mode A, providing full on-chain transparency and deterministic execution. Mode B remains a designed upgrade path for future scalability needs but is not required for current functionality.