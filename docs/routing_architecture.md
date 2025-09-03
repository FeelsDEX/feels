# Hub-and-Spoke Routing Architecture

## Overview

The Feels Protocol implements a hub-and-spoke routing model with FeelsSOL as the central hub token. This design ensures predictable gas costs, simplified liquidity aggregation, and maximum composability while maintaining capital efficiency.

## Core Principles

1. **Hub Constraint**: All spot pools must include FeelsSOL as one side
2. **Bounded Routes**: Maximum 2 hops for any token swap
3. **Entry/Exit Gateway**: System entry/exit exclusively through JitoSOL ↔ FeelsSOL
4. **Position Transitions**: All position changes route through FeelsSOL

## Architecture Components

### 1. Pool Initialization
```rust
// All pools must include FeelsSOL
let is_valid = token_0 == FEELSSOL_MINT || token_1 == FEELSSOL_MINT;
```

### 2. Route Types

#### Direct Routes (1 hop)
- TokenA ↔ FeelsSOL
- FeelsSOL ↔ TokenB
- JitoSOL ↔ FeelsSOL (entry/exit)

#### Indirect Routes (2 hops)
- TokenA → FeelsSOL → TokenB
- PositionX → FeelsSOL → PositionY

### 3. Segmentation

Trades may be segmented within each hop for better execution:
- Maximum segments per hop: 10
- Maximum total segments: 20
- Segmentation is size-based, not route-based

## Implementation Details

### Constants
```rust
pub const MAX_ROUTE_HOPS: usize = 2;
pub const MAX_SEGMENTS_PER_HOP: usize = 10;
pub const MAX_SEGMENTS_PER_TRADE: usize = 20;
```

### Validation Functions
```rust
// Pool validation
validate_pool_includes_feelssol(&token_0, &token_1, &FEELSSOL_MINT)?;

// Route validation
validate_route(&route_pools, MAX_ROUTE_HOPS)?;

// Entry/exit validation
validate_entry_exit_pairing(&token_in, &token_out, &JITOSOL_MINT, &FEELSSOL_MINT)?;
```

## User Flows

### 1. System Entry
```
User JitoSOL → FeelsSOL → Use in protocol
```

### 2. Token Swap
```
USDC → FeelsSOL → SOL (2 hops)
USDC → FeelsSOL (1 hop)
```

### 3. Position Management
```
Enter: FeelsSOL → Time Position
Convert: Time Position → FeelsSOL → Leverage Position
Exit: Leverage Position → FeelsSOL
```

### 4. System Exit
```
Protocol activity → FeelsSOL → JitoSOL → User
```

## SDK Usage

### Router Example
```rust
use feels_sdk::{HubRouter, PoolInfo};

// Initialize router
let mut router = HubRouter::new(FEELSSOL_MINT);

// Add pools
router.add_pool(PoolInfo {
    address: pool_address,
    token_a: USDC_MINT,
    token_b: FEELSSOL_MINT,
    fee_rate: 30, // 0.3%
})?;

// Find route
let route = router.find_route(&USDC_MINT, &SOL_MINT)?;
assert_eq!(route.hops, 2); // USDC → FeelsSOL → SOL
```

### Entry/Exit Example
```rust
use feels_sdk::instructions::{enter_system, exit_system};

// Enter system
let entry_ix = enter_system(
    &program_id,
    &user,
    &JITOSOL_MINT,
    &FEELSSOL_MINT,
    &user_jitosol_ata,
    &user_feelssol_ata,
    &feelssol_state,
    &feelssol_vault,
    amount_in,
    min_amount_out,
);

// Exit system
let exit_ix = exit_system(
    &program_id,
    &user,
    &JITOSOL_MINT,
    &FEELSSOL_MINT,
    &user_jitosol_ata,
    &user_feelssol_ata,
    &feelssol_state,
    &feelssol_vault,
    amount_in,
    min_amount_out,
);
```

## Benefits

1. **Gas Predictability**: Maximum 2 hops ensures bounded compute
2. **Liquidity Concentration**: All liquidity includes FeelsSOL
3. **Simplified Routing**: No complex pathfinding needed
4. **Composability**: Standard interface for all operations
5. **Risk Management**: Central hub enables better monitoring

## Migration Guide

This is a clean implementation with zero backwards compatibility:
- No legacy route support
- No migration paths
- All pools must be recreated with FeelsSOL
- Existing positions must be exited and re-entered

## Security Considerations

1. **Pool Validation**: Enforced at initialization
2. **Route Validation**: Checked on every trade
3. **Slippage Protection**: Built into all operations
4. **Segmentation Limits**: Prevent DoS attacks

## Future Enhancements

- Intra-domain position morphing (Time ↔ Time)
- Dynamic fee optimization per route
- Cross-chain hub connections
- Advanced segmentation strategies