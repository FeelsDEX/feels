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

### Dynamic Fee Model
- **Base Fees**: Configurable per-market base trading fees
- **Impact Fees**: Dynamic fees based on price movement (ticks crossed)
- **Fee Distribution**: Split between LPs, protocol treasury, and creators
- **Floor Protection**: Monotonic floor ratchet mechanism

### Architecture
- **Routing Constraints**: MAX_ROUTE_HOPS = 2, MAX_SEGMENTS_PER_HOP = 10
- **Segmentation**: Size-based trade splitting within hops
- **Zero-copy accounts**: Efficient state management
- **Safe math**: Overflow protection throughout
- **Token-2022 support**: Next-gen token standard

## Quick Start

### Entry Flow
1. Convert JitoSOL to FeelsSOL using `enter_feelssol`
2. Use FeelsSOL to trade any token in the protocol
3. Provide liquidity to any FeelsSOL pair

### Trading Flow
```rust
// Direct swap (1 hop)
USDC → FeelsSOL

// Indirect swap (2 hops)  
USDC → FeelsSOL → SOL
```

### Exit Flow
1. Exit positions to FeelsSOL
2. Convert FeelsSOL back to JitoSOL using `exit_feelssol`

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
# Run all tests
just test

# Run specific test categories
just test-unit           # Unit tests only
just test-integration    # Integration tests only
just test-e2e           # End-to-end tests only
just test-property      # Property-based tests only

# Run specific tests
cargo test test_swap_exact_amount

# Run with output
cargo test -- --nocapture
```

## SDK Usage

```rust
use feels_sdk::{FeelsClient, SdkConfig};

// Initialize client
let config = SdkConfig::localnet(payer);
let client = FeelsClient::new(config)?;

// Execute swap
let sig = client.swap(
    &user_token_in,
    &user_token_out,
    &token_in_mint,
    &token_out_mint,
    amount_in,
    minimum_amount_out,
)?;
```

## Program Addresses

- Protocol: `Fee1sProtoco11111111111111111111111111111111`
- FeelsSOL Mint: Determined at initialization

## Documentation

- [System Overview](docs/000_system_overview.md)
- [Dynamic Fees](docs/001_instantaneous_fees.md)
- [Continuous Rebasing](docs/002_continuous_rebasing.md)
- [Verification and Policy](docs/003_verification_and_policy.md)
- [MVP Checklist](docs/100_mvp_checklist_work_plan.md)
