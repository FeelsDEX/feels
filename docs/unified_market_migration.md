# Unified Market Account Migration Plan

## Overview

This document outlines the migration from the current split architecture (MarketField + MarketManager) to a unified Market account structure. This consolidation will create a single authoritative account for all market state, reducing complexity and improving efficiency.

## Current Architecture

### Split State Model
- **MarketField**: Thermodynamic physics parameters (S, T, L, weights, volatility)
- **MarketManager**: Traditional AMM state (sqrt_price, liquidity, fee tracking)

### Issues with Current Model
1. Two accounts required for nearly every instruction
2. Complex synchronization logic between accounts
3. Higher transaction costs due to multiple account loads
4. Fragmented mental model for developers

## Unified Architecture

### Single Market Account
The new `Market` account consolidates all state:
```rust
pub struct Market {
    // Identity
    pool: Pubkey,
    token_0: Pubkey,
    token_1: Pubkey,
    
    // Thermodynamic state
    S: u128,
    T: u128, 
    L: u128,
    
    // Weights
    w_s: u32,
    w_t: u32,
    w_l: u32,
    w_tau: u32,
    
    // AMM state
    sqrt_price: u128,
    current_tick: i32,
    liquidity: u128,
    
    // Fee tracking
    fee_growth_global_0: [u64; 4],
    fee_growth_global_1: [u64; 4],
    
    // ... additional fields
}
```

## Migration Steps

### Phase 1: Infrastructure (Completed ✅)
- [x] Create unified Market account structure (`state/unified_market.rs`)
- [x] Create unified state access layer (`logic/unified_state_access.rs`)
- [x] Create unified market instructions (`instructions/unified_market.rs`)
- [x] Update module exports

### Phase 2: Instruction Migration (In Progress)
- [ ] Update order instructions to use unified Market
- [ ] Update position management instructions
- [ ] Update maintenance operations
- [ ] Remove dependencies on MarketField/MarketManager

### Phase 3: Logic Layer Updates
- [ ] Update OrderManager to use unified state
- [ ] Update risk management modules
- [ ] Update fee calculation logic
- [ ] Update work calculation to use unified market

### Phase 4: Testing and Validation
- [ ] Update unit tests
- [ ] Update integration tests
- [ ] Create migration tests
- [ ] Validate state consistency

### Phase 5: SDK and Keeper Updates
- [ ] Update SDK to use unified Market structure
- [ ] Update keeper to work with unified state
- [ ] Update documentation

## Implementation Details

### State Access Pattern Changes

#### Before (Split Model)
```rust
pub struct StateContext<'info> {
    market: MarketStateAccess<'info>,  // Wraps both MarketField and MarketManager
    ticks: TickStateAccess<'info>,
    positions: PositionStateAccess<'info>,
    buffer: BufferStateAccess<'info>,
}
```

#### After (Unified Model)
```rust
pub struct UnifiedStateContext<'info> {
    market: MarketAccess<'info>,  // Direct access to unified Market
    ticks: TickStateAccess<'info>,
    positions: PositionStateAccess<'info>,
    buffer: BufferStateAccess<'info>,
}
```

### Instruction Account Changes

#### Before
```rust
#[derive(Accounts)]
pub struct Order<'info> {
    #[account(mut)]
    pub market_field: Account<'info, MarketField>,
    
    #[account(mut)]
    pub market_manager: AccountLoader<'info, MarketManager>,
    // ...
}
```

#### After
```rust
#[derive(Accounts)]
pub struct UnifiedOrder<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    // ...
}
```

## Benefits

1. **Simplified Mental Model**: Single source of truth for market state
2. **Reduced Transaction Costs**: One account load instead of two
3. **Better Performance**: Fewer account deserializations
4. **Cleaner Code**: No synchronization logic needed
5. **Easier Maintenance**: Single state model to maintain

## Migration Compatibility

The migration will maintain backward compatibility during the transition:
1. Keep existing instructions operational
2. Add new unified instructions alongside
3. Gradually deprecate old instructions
4. Final cutover after thorough testing

## Next Steps

1. Complete Phase 2 instruction migrations
2. Update order system to use unified state
3. Begin comprehensive testing
4. Plan deployment strategy