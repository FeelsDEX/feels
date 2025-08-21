# Feels Solana

A concentrated liquidity AMM that unifies exchange, lending, and leverage operations through a 3D position model.

## Architecture Overview

The protocol models all financial interactions as lending operations with three orthogonal dimensions:

- **Price**: Liquidity provision price point (specific price for exchange, 1.0 for staking-style positions)
- **Duration**: Capital commitment timeframe (Flash/Spot/Monthly)
- **Leverage**: Risk tier (Senior 1x or Junior 3x)

This parameter system allows a single liquidity pool to serve multiple functions that are typically handled by separate protocols.

## Core Components

### Position Model
All user interactions create positions defined by `{price, duration, leverage}` parameters. Different parameter combinations yield different behaviors:
- Exchange-focused: `{price: 1.25, duration: Spot, leverage: Senior}`
- Lending-focused: `{price: 1.0, duration: Monthly, leverage: Senior}`
- Leverage-focused: `{price: 1.0, duration: Spot, leverage: Junior}`

### FeelsSOL Token
The protocol uses FeelsSOL as a base asset, designed to be backed by jitoSOL through oracle-driven conversion rates. This allows users to maintain staking yield exposure while participating in DeFi activities. All trading pairs are structured as FeelsSOL ↔ User-Created Token.

### Tranched Risk System
Positions are allocated between two risk tranches:
- **Senior Tier (1x leverage)**: Protected positions with priority in loss allocation
- **Junior Tier (3x leverage)**: Amplified exposure positions that absorb losses first

Positions adjust in value based on pool performance but are never forcibly liquidated.

### Automated Maintenance
The protocol uses an execution scheduler where users must perform due maintenance operations before executing their own transactions. This eliminates dependency on external keepers while ensuring protocol operations continue.

### Protocol-Owned Liquidity (POL)
A percentage of fees from all position types is retained to build protocol-owned liquidity. This creates a permanent liquidity base that grows with protocol usage and provides price stability.

## Implementation Status

**Completed Components**:
- Core concentrated liquidity AMM with tick-based pricing
- Senior/Junior risk tier allocation without liquidations
- On-chain scheduler with mandatory execution model
- Monthly term system with rollover processing
- Yield distribution accounting for position parameters
- FeelsSOL Token-2022 implementation

**Planned Components**:
- jitoSOL oracle integration for FeelsSOL backing
- Unified three-parameter position interface
- Advanced tick state monitoring and metrics
- Synchronized global term boundaries

## Technical Architecture

The protocol consists of several Rust modules:

- **`state`**: Account definitions for `PoolState`, `Position`, `TickBitmap`, and other core data structures
- **`instructions`**: Transaction handlers for `swap`, `add_liquidity`, `remove_liquidity`, and other user operations
- **`scheduler`**: Task queue management and incentive distribution for maintenance operations
- **`yield_distributor`**: Fee allocation logic considering position leverage, duration, and LVR compensation
- **`term_system`**: Lifecycle management for fixed-term positions including expiry and rollover
- **`math`**: Q64.96 fixed-point arithmetic for precise AMM calculations

The design uses zero-copy account serialization for efficiency and supports Token-2022 standard for advanced token features.

## Building

```bash
# Build with Nix
nix develop
just build

# Or with Anchor
anchor build
```

## Testing

```bash
# Run tests
cargo test

# Integration tests
cargo test --test integration
```

## Deployment

Program is deployed to:
- `feels-protocol`: `Fee1sProtoco11111111111111111111111111111`
