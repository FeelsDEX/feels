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

## Testing

### Prerequisites

- Rust and Cargo installed
- [Solana CLI tools installed](https://solana.com/docs/intro/installation#install-the-solana-cli)
- [Anchor framework installed](https://solana.com/docs/intro/installation#install-anchor-cli)

### Build and deploy Program instructions

1. Build all programs

```bash
anchor build
```

2. (In a separate terminal) Spin up a Solana local validator forked from mainnet. Some of the tests use the local-validator because of some limitations of the solana-program-test tooling. We clone some mainnet programs to also test our integrations with external protocols

```bash
solana-test-validator \
    --clone J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn \
    --clone Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb \
    --clone-upgradeable-program SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy \
    --url mainnet-beta \
    --reset
```

3. Fund the test account with enough Solana to start interacting with the local network

```bash
solana airdrop 50000 AGkjyfbEoLbehpZ4BC4ZxAr6zsyeVoDYdp7s43PCtQjS
```

4. Deploy the programs. This will deploy all programs to the local running validator which are used by some of the tests

```bash
anchor deploy
```

5. Run the tests

```bash
cargo test
```
