# Feels Protocol

A concentrated liquidity AMM on Solana implementing a physics-based 3D trading model with hub-and-spoke routing where all tokens trade through a universal FeelsSOL base pair.

## Overview

Feels Protocol is a hub-and-spoke AMM where all tokens trade through FeelsSOL as the central routing asset. The protocol converts speculative trading into long-term value through programmatic market making and floor price mechanisms. Trading fees accumulate in protocol-owned accounts that provide just-in-time liquidity and maintain hard price floors.

The protocol implements concentrated liquidity with tick-based positioning to place capital precisely where needed. Each token has exactly one market paired with FeelsSOL, creating unified liquidity and eliminating routing complexity. Protocol-owned accounts deploy autonomous market making strategies including floor liquidity that creates hard price floors and JIT liquidity that captures value from directional trades.

Implementation includes:

- Concentrated liquidity AMM with tick-based price ranges
- Protocol-owned market making with floor and JIT strategies
- Geometric time-weighted average pricing for manipulation resistance
- Dynamic fee structure based on price impact and market conditions

Architecture constraints include maximum 2-hop routing, segmented trade execution, and zero-copy account management for efficient state updates. The system uses Token-2022 standards and implements comprehensive overflow protection throughout the codebase.

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

### Solana Program

```bash
# Using Nix (recommended)
nix develop
just build

# Using Anchor directly (requires proper environment)
anchor build
```

**Note**: The `just build` command uses a Nix shell and runs `nix develop --command anchor build --no-idl --program-name feels`. Direct `anchor build` may not work without the proper environment setup.

### WASM Vanity Miner

The vanity address miner uses multi-threaded WebAssembly with `wasm-bindgen-rayon`:

```bash
# Build with parallel features (production)
just frontend generate-wasm

# Or from vanity-miner-wasm directory
cd vanity-miner-wasm
just build         # Production build
just build-dev     # Development build with debug info
```

See [vanity-miner-wasm/BUILD.md](vanity-miner-wasm/BUILD.md) for detailed build configuration and troubleshooting.

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

### Developer Resources
- [DeepWiki MCP Integration](docs/deepwiki-mcp-guide.md) - AI-assisted documentation access via MCP
- [WASM Build Guide](vanity-miner-wasm/BUILD.md) - Parallel WASM builds with wasm-bindgen-rayon

### Core Documentation
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
