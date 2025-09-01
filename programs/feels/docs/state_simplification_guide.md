# State & Account Simplification Guide

## Overview

This guide describes the simplification of the Feels Protocol's state management, reducing the number of accounts required per pool while maintaining performance.

## Changes Summary

### Before: 5+ Accounts per Pool
1. **Pool** - Core hot-path data
2. **PoolMetrics** - Volume and leverage statistics  
3. **PoolHooks** - Hook configuration
4. **PoolRebase** - Rebase configuration
5. **LendingMetrics** - Lending-specific metrics
6. **VolatilityTracker** - Volatility observations
7. **Oracle** - Price observations

### After: 2-3 Accounts per Pool
1. **PoolSimplified** - Core data + merged hooks/rebase config
2. **PoolMetricsConsolidated** - All metrics in one account
3. **Oracle** (optional) - Kept separate due to size

## Benefits

1. **Reduced Transaction Costs**: ~50% fewer accounts = lower fees
2. **Simpler Client Integration**: Less accounts to manage
3. **Easier Pool Creation**: Single initialization transaction
4. **Maintained Performance**: Hot/cold data separation preserved
5. **Better UX**: Fewer accounts for users to approve

## Account Details

### PoolSimplified

Merges data from Pool, PoolHooks, and PoolRebase:

```rust
pub struct PoolSimplified {
    // Hot-path data (unchanged)
    pub current_tick: i32,
    pub current_sqrt_rate: u128,
    pub liquidity: u128,
    pub tick_array_bitmap: [u64; 16],
    pub fee_growth_global_a: [u64; 4],
    pub fee_growth_global_b: [u64; 4],
    
    // Merged from PoolHooks
    pub hook_registry: Pubkey,
    pub hooks_enabled: bool,
    
    // Merged from PoolRebase  
    pub rebase_accumulator: Pubkey,
    pub last_redenomination: i64,
    pub redenomination_threshold: u64,
    pub last_rebase_timestamp: i64,
    pub rebase_epoch_duration: i64,
    
    // Reference to consolidated metrics
    pub pool_metrics: Pubkey,
}
```

### PoolMetricsConsolidated

Combines all metrics into a single account:

```rust
pub struct PoolMetricsConsolidated {
    // Volume metrics
    pub total_volume_a: u128,
    pub total_volume_b: u128,
    pub volume_tracker: VolumeTracker,
    
    // Lending metrics
    pub total_supplied: u128,
    pub total_borrowed: u128,
    pub utilization_rate: u16,
    pub flash_volume_total: u128,
    
    // Volatility summary
    pub volatility_composite: u16,
    pub volatility_percentile: u8,
    
    // Fee metrics
    pub fees_collected_a: u128,
    pub fees_collected_b: u128,
    pub avg_fee_rate_24h: u16,
}
```

## Migration Path

### Phase 1: Dual Support
- Deploy new account structures
- Update SDK to support both old and new
- New pools use simplified structure
- Existing pools continue with old structure

### Phase 2: Migration Tools
```rust
// Automatic migration helper
pub fn migrate_pool(
    old_pool: Pool,
    old_hooks: PoolHooks,
    old_rebase: PoolRebase,
    old_metrics: PoolMetrics,
) -> (PoolSimplified, PoolMetricsConsolidated)
```

### Phase 3: Gradual Migration
- Provide incentives for pool migration
- Run migration bot for inactive pools
- Update all client libraries

### Phase 4: Deprecation
- Mark old accounts as deprecated
- Remove support in future version

## Instruction Updates

### Before
```rust
#[derive(Accounts)]
pub struct SwapAccounts<'info> {
    pub pool: AccountLoader<'info, Pool>,
    pub pool_metrics: Account<'info, PoolMetrics>,
    pub pool_hooks: Account<'info, PoolHooks>,
    pub lending_metrics: Account<'info, LendingMetrics>,
    // ... 20+ more accounts
}
```

### After
```rust
#[derive(Accounts)]
pub struct SwapAccountsSimplified<'info> {
    pub pool: AccountLoader<'info, PoolSimplified>,
    pub pool_metrics: Account<'info, PoolMetricsConsolidated>,
    // ... fewer accounts total
}
```

## Performance Considerations

### Zero-Copy Optimization
All hot-path accounts use `#[account(zero_copy)]`:
- PoolSimplified
- TickArray
- GradientCache
- BufferAccount

### Data Access Patterns
- **Hot Data**: In PoolSimplified (accessed every swap)
- **Warm Data**: In PoolMetricsConsolidated (accessed periodically)
- **Cold Data**: In separate specialized accounts (Oracle, VolatilityTracker)

## Size Analysis

### Account Sizes
- **PoolSimplified**: ~1.2KB (from 1KB)
- **PoolMetricsConsolidated**: ~800B (combined from 1.5KB)
- **Total**: ~2KB (from 3.5KB+)

### Rent Savings
- Per pool: ~0.015 SOL saved
- 10,000 pools: ~150 SOL saved
- Annual: ~$15,000 at $100/SOL

## Best Practices

1. **Keep Hot Data Separate**: Don't merge frequently accessed data with cold data
2. **Use Zero-Copy**: For any account >1KB or accessed in hot paths
3. **Batch Updates**: Update metrics in batches to reduce write locks
4. **Version Accounts**: Include version field for future migrations

## Future Optimizations

### Compressed Accounts
Future versions could use state compression:
- Store only recent data on-chain
- Archive historical data to merkle trees
- Generate proofs for historical queries

### Account Recycling
Implement account recycling for tick arrays:
- Reuse empty tick array accounts
- Reduce account creation costs
- Implement garbage collection

## Conclusion

This simplification reduces complexity while maintaining performance. The 50% reduction in accounts translates to real cost savings and better user experience.