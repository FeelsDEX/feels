# Feels Protocol

A concentrated liquidity AMM on Solana implementing a physics-based 3D trading model with hub-and-spoke routing where all tokens trade through a universal FeelsSOL base pair.

## Overview

Feels Protocol implements Uniswap V3-style concentrated liquidity with a key innovation: every token must pair with FeelsSOL (wrapped liquid staking tokens). This creates efficient routing and unified liquidity while preparing for future three-dimensional trading capabilities.

### Hub-and-Spoke Routing
- **Central Hub**: All pools must include FeelsSOL as one side
- **Bounded Routes**: Maximum 2 hops for any swap (TokenA → FeelsSOL → TokenB)
- **Entry/Exit**: System access only through JitoSOL ↔ FeelsSOL conversion
- **Predictable Gas**: Route length bounded by design
- **No Legacy Support**: Clean implementation with zero backwards compatibility

### FeelsSOL Hub Token
- Wraps yield-bearing LSTs (initially JitoSOL)
- Central token in all trading pairs
- Enables unified liquidity and simplified routing
- Automatic yield distribution to holders

### Concentrated Liquidity
- LPs provide liquidity within custom price ranges
- Capital efficiency through tick position concentration
- NFT-based tick position tracking with accumulated fees
- Tick-based pricing with configurable spacing

### 3D Trading Model
- **Spot Dimension**: Traditional price discovery
- **Time Dimension**: Duration-based positions (Flash, Weekly, Monthly, etc.)
- **Leverage Dimension**: Risk-adjusted leveraged positions
- Unified physics-based fee model across all dimensions

### Architecture
- **Routing Constraints**: MAX_ROUTE_HOPS = 2, MAX_SEGMENTS_PER_HOP = 10
- **Segmentation**: Size-based trade splitting within hops
- **Zero-copy accounts**: Efficient state management
- **Safe math**: Overflow protection throughout
- **Token-2022 support**: Next-gen token standard

## Quick Start

### Entry Flow
1. Convert JitoSOL to FeelsSOL using `enter_system`
2. Use FeelsSOL to trade any token in the protocol
3. Provide liquidity to any FeelsSOL pair
4. Enter time or leverage positions

### Trading Flow
```rust
// Direct swap (1 hop)
USDC → FeelsSOL

// Indirect swap (2 hops)  
USDC → FeelsSOL → SOL
```

### Exit Flow
1. Exit positions to FeelsSOL
2. Convert FeelsSOL back to JitoSOL using `exit_system`

## Building

```bash
# Using Nix
nix develop
just build

# Using Anchor
anchor build
```

## Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test integration

# Routing tests
cargo test routing
```

## SDK Usage

```rust
use feels_sdk::{HubRouter, instructions::*};

// Initialize router
let router = HubRouter::new(FEELSSOL_MINT);

// Find route
let route = router.find_route(&TOKEN_A, &TOKEN_B)?;

// Build swap instructions
let ixs = hub_swap(
    &program_id,
    &token_in,
    &token_out, 
    &feelssol_mint,
    &user,
    amount,
    min_out,
    &router
)?;
```

## Program Addresses

- Protocol: `Fee1sProtoco11111111111111111111111111111111`
- FeelsSOL Mint: Determined at initialization

## Documentation

- [Routing Architecture](docs/routing_architecture.md)
- [System Overview](docs/000_system_overview.md)
- [Instantaneous Fees](docs/001_instantaneous_fees.md)
- [Continuous Rebasing](docs/002_continuous_rebasing.md)
- [Verification and Policy](docs/003_verification_and_policy.md)
