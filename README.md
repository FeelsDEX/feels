# Feels Protocol

A concentrated liquidity AMM on Solana with a unique hub-and-spoke model where all tokens trade through a universal FeelsSOL base pair.

## Overview

Feels Protocol implements Uniswap V3-style concentrated liquidity with a key innovation: every token must pair with FeelsSOL (wrapped liquid staking tokens). This creates efficient routing and unified liquidity while preparing for future three-dimensional trading capabilities.

### FeelsSOL Synthetic Pair
- All pools use FeelsSOL as the base pair
- FeelsSOL wraps yield-bearing LSTs (e.g., JitoSOL)
- Cross-token swaps route automatically: TokenA → FeelsSOL → TokenB
- Simplifies liquidity aggregation and price discovery

### Concentrated Liquidity
- LPs provide liquidity within custom price ranges
- Capital efficiency through tick position concentration
- NFT-based tick position tracking with accumulated fees
- Tick-based pricing with configurable spacing

### Architecture
- Canonical token ordering ensuring unique pool addresses
- 512-byte reserved space in pools for future upgrades
- Zero-copy accounts
- Safe math
- Token-2022 support

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
cargo test --test integration_phase1
```

## Program Addresses

- Protocol: `Fee1sProtoco11111111111111111111111111111111`
- FeelsSOL Mint: Determined at initialization
