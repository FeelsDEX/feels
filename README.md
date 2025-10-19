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
# Using Nix (recommended)
nix develop
just build

# Using Anchor directly (requires proper environment)
anchor build
```

**Note**: The `just build` command uses a Nix shell and runs `nix develop --command anchor build --no-idl --program-name feels`. Direct `anchor build` may not work without the proper environment setup.

## Testing

```bash
# Run all tests
just test

# Run specific test categories
just test unit           # Unit tests only
just test integration    # Integration tests only
just test e2e           # End-to-end tests only
just test property      # Property-based tests only
just test localnet      # Localnet tests with validator
just test devnet        # Devnet tests

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

## Deployment

### Environment Configuration

The protocol uses a `.env` file to configure deployment parameters:

1. Copy `.env.example` to `.env`:
```bash
cp .env.example .env
```

2. Set your program authority in `.env`:
```bash
PROGRAM_AUTHORITY=YourAuthorityPublicKeyHere
```

3. Verify your configuration:
```bash
just check-env
```

### Deploying

Deploy to localnet:
```bash
just deploy
```

Deploy to devnet:
```bash
just deploy-devnet
```

The deployment scripts will automatically:
- Load the `PROGRAM_AUTHORITY` from your `.env` file
- Search for the corresponding keypair in standard locations
- Deploy with the specified authority

## Program Addresses

- Protocol: `5EeFL2XscLKAg9HWq5Ssbo3h4nBWHb1qcRZK6V6yt18S`
- FeelsSOL Mint: Determined at initialization

## Documentation

- [System Introduction](docs/900_system_intro.md)
- [Unified Markets](docs/901_unified_markets.md)
- [FeelsSOL Solvency](docs/200_feelssol_solvency.md)
- [Dynamic Fees](docs/201_dynamic_fees.md)
- [JIT Liquidity](docs/202_jit_liquidity.md)
- [Pool CLMM](docs/203_pool_clmm.md)
- [Pool Oracle](docs/204_pool_oracle.md)
- [Floor Liquidity](docs/205_floor_liquidity.md)
- [Pool Allocation](docs/206_pool_allocation.md)
- [Bonding Curve Feels](docs/207_bonding_curve_feels.md)
- [After Swap Pipeline](docs/208_after_swap_pipeline.md)
- [Params and Governance](docs/209_params_and_governance.md)
- [Safety Controller](docs/210_safety_controller.md)
- [Events and Units](docs/211_events_and_units.md)
- [Pool Registry](docs/212_pool_registry.md)
- [Launch Sequence](docs/300_launch_sequence.md)
- [Vaults and Lending Future](docs/400_vaults_and_lending_future.md)
- [Phase 2 Roadmap](docs/500_phase2_roadmap.md)
