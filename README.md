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

## For AI Agents

**Start with [CLAUDE.md](CLAUDE.md)** - Primary entry point with project orientation, code-to-doc mapping, and navigation to detailed specifications.

## Quick Start

### Entry Flow
1. Deposit JitoSOL to protocol vaults using `enter_feelssol` to mint FeelsSOL
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
2. Redeem FeelsSOL for JitoSOL using `exit_feelssol` from protocol vaults

## Building

### Solana Program

```bash
# Using Nix (recommended)
nix develop
just build

# Using Anchor directly (requires proper environment)
anchor build
```

**Note**: The `just build` command automatically detects your environment and uses Anchor by default. It can optionally use Nix with `just build feels nix` for reproducible builds. Direct `anchor build` may not work without the proper environment setup.

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
just test unit          # Unit tests only
just test integration   # Integration tests only
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

- Protocol: `J13h8cLst2B5H6RbWi9CVmeaDnAqio38uyJiUmAd1RPF`
- FeelsSOL Mint: Determined at initialization

## Documentation

**Start Here**: [Documentation Index](feels-app/content/specs/DOCS-INDEX.md) | [Glossary](feels-app/content/specs/GLOSSARY.md)

### Core Documentation
- [Introduction](feels-app/content/specs/001-introduction.md)
- [Quickstart Guide](feels-app/content/specs/002-quickstart.md)
- [Hub and Spoke Architecture](feels-app/content/specs/003-hub-and-spoke-architecture.md)
- [FeelsSOL Solvency](feels-app/content/specs/200-feelssol-solvency.md)
- [Dynamic Fees](feels-app/content/specs/201-dynamic-fees.md)
- [JIT Liquidity](feels-app/content/specs/202-jit-liquidity.md)
- [Pool CLMM](feels-app/content/specs/203-pool-clmm.md)
- [Pool Oracle](feels-app/content/specs/204-pool-oracle.md)
- [Floor Liquidity](feels-app/content/specs/205-floor-liquidity.md)
- [Pool Allocation](feels-app/content/specs/206-pool-allocation.md)
- [Bonding Curve Implementation](feels-app/content/specs/207-bonding-curve-feels.md)
- [After Swap Pipeline](feels-app/content/specs/208-after-swap-pipeline.md)
- [Params and Governance](feels-app/content/specs/209-params-and-governance.md)
- [Safety Controller](feels-app/content/specs/210-safety-controller.md)
- [Events and Units](feels-app/content/specs/211-events-and-units.md)
- [Pool Registry](feels-app/content/specs/212-pool-registry.md)
- [Launch Sequence](feels-app/content/specs/300-launch-sequence.md)
- [Market State and Lifecycle](feels-app/content/specs/301-market-state-and-lifecycle.md)
