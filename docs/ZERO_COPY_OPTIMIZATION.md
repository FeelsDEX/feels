# Zero-Copy and Account Compression Optimization Plan

## Overview
This document outlines the future optimization path for the Feels Protocol using zero-copy deserialization and account compression. These optimizations will significantly reduce account rent costs and improve performance.

## Current State
The Pool account is already using `#[account(zero_copy)]` which provides:
- Direct memory mapping without deserialization overhead
- Efficient field access through references
- Lower compute unit usage for reads

## Future Optimization: Account Compression

### Phase 1: Identify Cold vs Hot Data

**Hot Data** (frequently accessed, stays in Pool):
- `current_tick`
- `current_sqrt_rate`
- `liquidity`
- `fee_growth_global_a/b`
- `reentrancy_status`

**Cold Data** (infrequently accessed, move to compressed accounts):
- `leverage_stats`
- `volume_tracker`
- `dynamic_fee_config`
- `last_redenomination`
- Historical data

### Phase 2: Implement Compressed State Accounts

```rust
// Future compressed account structure
#[account]
pub struct PoolCompressedState {
    pub pool: Pubkey,
    pub merkle_tree: Pubkey,
    pub leaf_index: u32,
    
    // Compressed data stored off-chain
    // Accessed via merkle proofs
}

// Modified Pool structure
#[account(zero_copy)]
pub struct PoolV2 {
    // Essential hot state
    pub current_tick: i32,
    pub current_sqrt_rate: u128,
    pub liquidity: u128,
    
    // References to compressed data
    pub compressed_state: Pubkey,
    pub state_tree_root: [u8; 32],
}
```

### Phase 3: Migration Strategy

1. **Deploy Compression Infrastructure**
   - Set up merkle tree accounts
   - Deploy compression program
   - Create state transition handlers

2. **Gradual Migration**
   - New pools use compressed structure
   - Existing pools migrate on first interaction
   - Maintain backwards compatibility temporarily

3. **Access Patterns**
   ```rust
   // Reading compressed data
   pub fn read_leverage_stats(
       pool: &Pool,
       proof: &[u8],
   ) -> Result<LeverageStatistics> {
       // Verify merkle proof
       // Decompress data
       // Return stats
   }
   ```

## Benefits

### Rent Reduction
- Current Pool size: ~1KB → Future: ~200 bytes
- 80% reduction in rent costs
- Enables more pools without rent burden

### Performance Improvements
- Faster account loading (smaller size)
- Selective data access (only decompress needed fields)
- Better cache utilization

### Scalability
- Support for unlimited historical data
- No account size constraints
- Efficient batch operations

## Implementation Locations

### Pool State (`state/pool.rs`)
```rust
// TODO: When implementing compression, these fields move to compressed storage:
// - leverage_params
// - leverage_stats  
// - volume_tracker
// - dynamic_fee_config
// - oracle history
```

### Tick Arrays (`state/tick.rs`)
```rust
// TODO: Implement compressed tick storage
// - Store inactive ticks in merkle tree
// - Keep active range in account
// - Lazy loading on access
```

### Position Metadata (`state/tick_position.rs`)
```rust
// TODO: Compress historical position data
// - Keep current state in NFT
// - Archive closed positions
// - Merkle proofs for claims
```

## Technical Considerations

### Merkle Tree Design
- Use concurrent merkle trees for parallelism
- 32-byte leaves for efficient proofs
- Canopy depth based on access patterns

### Compression Algorithm
- Borsh serialization for consistency
- Optional zstd compression for cold data
- Batch compression for efficiency

### Proof Generation
- Off-chain indexer maintains proofs
- RPC methods for proof queries
- Client-side proof caching

## Timeline Estimate

1. **Research & Design**: 2 weeks
2. **Infrastructure Setup**: 3 weeks
3. **Implementation**: 4 weeks
4. **Testing & Optimization**: 2 weeks
5. **Migration Tools**: 1 week

Total: ~12 weeks

## Debugging Considerations

While compression adds complexity for debugging:
- Maintain uncompressed dev/test environments
- Build comprehensive logging
- Create inspection tools
- Document access patterns

## Conclusion

Account compression is the natural evolution for the Feels Protocol, enabling massive scalability while reducing costs. The current zero-copy implementation provides a solid foundation for this optimization.