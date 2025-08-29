# Feels Protocol Module Structure

## Overview
The Feels Protocol is organized into four main module categories, each with a specific purpose:

## 1. Instructions (`/instructions`)
Entry points for all protocol operations. Each instruction module contains handler functions that coordinate the business logic.

- **`pool`** - Pool initialization and configuration
- **`order`** - Unified 3D order execution (swaps, liquidity, limits)
- **`order_compute`** - Tick array computation for orders
- **`order_modify`** - Modify existing orders
- **`order_redenominate`** - Leverage redenomination
- **`fee`** - Fee collection and management
- **`config`** - Protocol configuration updates
- **`token`** - Token/NFT creation
- **`vault`** - Position vault operations
- **`cleanup`** - Maintenance operations

## 2. Logic (`/logic`)
Core business logic separated from instruction handlers. Contains complex calculations and state transitions.

- **`concentrated_liquidity`** - Core AMM math and liquidity management
- **`order`** - Order routing and execution logic
- **`tick`** - Tick array management and navigation
- **`tick_position`** - Position NFT calculations
- **`fee_manager`** - Fee calculation and distribution
- **`volatility_manager`** - Volatility tracking for dynamic fees
- **`hook`** - Hook system integration
- **`pool`** - Pool state management
- **`event`** - Event definitions

## 3. State (`/state`)
On-chain account definitions and data structures.

### Core State
- **`pool`** - Main pool account with all integrated features
- **`protocol`** - Global protocol state
- **`tick`** - Tick arrays for liquidity storage
- **`tick_position`** - Position NFT metadata
- **`token`** - Token metadata

### Advanced Features
- **`duration`** - Time dimension for 3D orders
- **`leverage`** - Continuous leverage system
- **`fee`** - Dynamic fee configuration
- **`oracle`** - Price oracle integration
- **`oracle_safe`** - Secure oracle with TWAP
- **`reentrancy`** - Reentrancy protection
- **`position_vault`** - Automated position management
- **`volume`** - Volume tracking
- **`volatility_tracker`** - High-frequency volatility
- **`flash_loan_twav`** - Flash loan tracking
- **`hook`** - Hook registry

### System
- **`error`** - Consolidated error definitions

## 4. Utils (`/utils`)
Shared utilities and helper functions.

- **`math`** - All mathematical operations
  - `amm` - AMM-specific math (sqrt prices, ticks, fees)
  - `big_int` - 256/512-bit integer operations
  - `q96` - Fixed-point Q96 math
  - `safe` - Safe arithmetic operations
- **`cpi_helpers`** - Cross-program invocation helpers
- **`account_pattern`** - Reusable account validation patterns
- **`deterministic_seed`** - PDA generation utilities
- **`time_weighted_average`** - TWAP/TWAV calculations
- **`token_validation`** - Token ticker validation
- **`error_handling`** - Error utility functions
- **`types`** - Common type definitions

## 5. Constants (`constant.rs`)
All protocol-wide constants in a single file.

## Key Design Principles

1. **Separation of Concerns** - Instructions handle coordination, logic handles calculations, state defines storage
2. **Unified 3D Model** - All trading is through the unified order system with Rate × Duration × Leverage
3. **Security First** - Reentrancy protection and secure oracles integrated at the core
4. **Zero-Copy Ready** - Pool struct designed for future compression optimization
5. **Clean Pre-Launch** - No legacy code or migration paths

## Future Optimizations

The codebase includes TODO comments for zero-copy optimization:
- Most Pool fields would move to compressed accounts
- Pool would store only hot state and references
- This reduces account rent and improves scalability