# Feels Protocol Test Suite

This directory contains comprehensive tests for the Feels protocol, organized by test type and functionality.

## Overview

The test suite supports multiple testing environments:
- **In-Memory**: Fast unit tests using `solana-program-test` 
- **Devnet**: Integration tests against devnet
- **Localnet**: Integration tests against local validator

Test coverage includes:
- Mathematical operations and utilities (unit tests)
- Security features and vulnerability regression tests
- Protocol lifecycle and cross-instruction interactions (integration tests)
- Complete user flows and AMM operations (e2e tests)
- Invariant checking and fuzz testing (property tests)

## Directory Structure

```
tests/
├── common/                 # Shared test infrastructure
│   ├── assertions.rs      # Custom assertion helpers
│   ├── builders.rs        # Test data builders
│   ├── client.rs          # Test client abstraction (InMemory/Devnet)
│   ├── context.rs         # Test context and pre-configured accounts
│   ├── environment.rs     # Environment configuration
│   ├── fixtures.rs        # Reusable test data
│   ├── helpers.rs         # High-level test helpers
│   ├── macros.rs          # Test macros for multi-environment testing
│   ├── time.rs            # Time manipulation utilities
│   └── tracing.rs         # Test output and debugging
├── unit/                   # Unit tests for individual components
│   ├── math/              # Mathematical operations (fees, rounding, dust)
│   ├── security/          # Security features (reentrancy, race conditions)
│   ├── pomm/              # Protocol-Owned Market Making logic
│   ├── buffer/            # Buffer management and overflow handling
│   ├── position/          # Position management and safety
│   ├── oracle/            # Oracle and observation logic
│   └── instructions/      # Instruction-specific tests
├── integration/            # Integration tests for feature workflows
│   ├── initialization/    # Market initialization variations
│   ├── liquidity/         # Liquidity provision patterns
│   ├── debug/             # Debug and diagnostic tests
│   └── token/             # Token lifecycle operations
├── e2e/                    # End-to-end tests for complete user flows
│   ├── test_full_trading_flow.rs    # Complete trading lifecycle
│   ├── test_position_metadata.rs    # NFT position tests
│   ├── test_token_lifecycle.rs      # Token mint→trade→burn flow
│   └── test_feelssol_basic.rs       # FeelsSOL entry/exit flows
├── property/               # Property-based and fuzz tests
│   └── fuzz_swap_invariants.rs      # Swap invariant testing
├── helpers/                # Legacy helpers (to be migrated to common/)
└── README.md              # This file
```

## Quick Start

### Prerequisites

1. **Build the BPF program first**:
```bash
cargo build-sbf
```

2. The tests will automatically find the BPF binary in `target/deploy/feels.so`

### Running Tests

The test suite uses a modular justfile system with all test commands imported into the root justfile:

```bash
# From project root
just test               # Run all tests
just test-unit          # Run unit tests only
just test-integration   # Run integration tests only
just test-property      # Run property tests only
just test-e2e           # Run e2e tests only

# Advanced test commands
just filter test_swap         # Run tests matching "test_swap"
just verbose                  # Run with verbose output
just parallel 4               # Run with 4 threads
just nocapture                # Don't capture output
just release                  # Run in release mode
just coverage                 # Generate coverage report
just watch                    # Watch mode (auto-rerun)

# Direct justfile usage (from tests directory)
cd programs/feels/tests
just --list             # Show all test commands
just all                # Run all tests
just unit               # Run unit tests
just integration        # Run integration tests
just property           # Run property tests
just e2e                # Run e2e tests
```

#### Using cargo directly (within Nix environment)

```bash
# Enter Nix development shell first
nix develop

# Run tests
cargo test --features test-utils                        # Run all tests
cargo test --features test-utils unit::                # Unit tests only
cargo test --features test-utils integration::         # Integration tests only  
cargo test --features test-utils e2e::                # E2E tests only
cargo test --features test-utils property::           # Property tests only

# Run specific subdirectory
cargo test --features test-utils unit::math::          # Math unit tests
cargo test --features test-utils unit::security::      # Security unit tests
cargo test --features test-utils integration::initialization::  # Initialization tests

# Run specific test
cargo test --features test-utils test_swap_exact_amount

# Run with output
cargo test --features test-utils -- --nocapture
```

## Test Categories

### Unit Tests (`unit/`)
Tests for individual components in isolation.

**Subdirectories:**
- `math/` - Fee calculations, rounding behavior, dust control
- `security/` - Reentrancy guards, race conditions, griefing protection
- `pomm/` - POMM width calculations, saturation tests
- `buffer/` - Buffer overflow handling, balance management
- `position/` - Position closing safety, fee collection
- `oracle/` - Observation offsets, timestamp security

### Integration Tests (`integration/`)
Tests for feature workflows involving multiple components.

**Subdirectories:**
- `initialization/` - Market initialization with various configurations
- `liquidity/` - Liquidity provision patterns (e.g., stair pattern)
- `debug/` - Debug and diagnostic tests
- `token/` - Token minting and lifecycle operations

### End-to-End Tests (`e2e/`)
Complete user journey tests that simulate real usage scenarios.

**Key tests:**
- Full trading flows with multiple participants
- Token lifecycle from creation to destruction
- Position management including metadata NFTs
- FeelsSOL hub token entry and exit flows

### Property Tests (`property/`)
Fuzz testing to verify invariants hold under random inputs.

**Current coverage:**
- Swap invariants (conservation of value)
- Mathematical properties (no overflow, consistent rounding)

## Naming Conventions

### File Names
- All test files follow the pattern: `test_*.rs`
- Use descriptive names: `test_<feature>_<scenario>.rs`
- Group related tests in the same file

### Test Function Names
```rust
#[test]
fn test_<component>_<scenario>_<expected_outcome>() {
    // Example: test_swap_large_amount_succeeds()
    // Example: test_position_close_with_fees_fails()
}
```

### Module Organization
Each subdirectory has a `mod.rs` that exports all test modules:
```rust
// unit/math/mod.rs
pub mod test_fee_growth;
pub mod test_fee_rounding;
pub mod test_dust_control;
```

## Writing Tests

### Test Macros

The test infrastructure provides macros for environment-specific testing:

```rust
use crate::common::*;

// Standard Rust test (converted from custom macros)
#[tokio::test]
async fn test_name() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    // Test logic here
    Ok(())
}
```

### Using Test Helpers

```rust
// Create a market with liquidity
let market = ctx.market_builder()
    .token_0(token_0.pubkey())
    .token_1(token_1.pubkey())
    .add_liquidity(alice.insecure_clone(), -1000, 1000, 1_000_000_000)
    .build()
    .await?;

// Execute a swap
let swap_result = ctx.swap_helper()
    .swap_exact_input(
        &market,
        &bob,
        true, // zero_for_one
        1_000_000,
        0,
    )
    .await?;

// Open a position
let position = ctx.position_helper()
    .open_position(
        &market,
        &alice,
        -100,
        100,
        1_000_000
    )
    .await?;
```

## Adding New Tests

### 1. Determine the Category
- **Unit**: Testing a single function or component in isolation?
- **Integration**: Testing a feature that uses multiple components?
- **E2E**: Testing a complete user workflow?
- **Property**: Testing invariants with random inputs?

### 2. Find or Create the Appropriate Location
- For unit tests, use the component subdirectory (math, security, etc.)
- For integration tests, use the feature subdirectory (initialization, liquidity, etc.)
- For e2e tests, place directly in the `e2e/` directory
- For property tests, place directly in the `property/` directory

### 3. Follow the Template
```rust
use crate::common::*;  // Import common test infrastructure

#[tokio::test]
async fn test_my_feature() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    
    // Setup
    let creator = Keypair::new();
    ctx.airdrop(&creator.pubkey(), 1_000_000_000).await?;
    
    // Execute
    let result = my_operation(&ctx, &creator).await?;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.value, expected_value);
    
    Ok(())
}
```

## Key Test Coverage Areas

### Mathematical Operations
- Safe arithmetic with overflow/underflow protection
- Fee calculation precision and rounding behavior
- Tick-price conversions and boundary conditions
- Liquidity math validation and delta calculations

### Security Features
- Reentrancy protection
- Race condition prevention
- Oracle timestamp validation
- Tick array griefing protection
- Buffer overflow handling

### Protocol Operations
- Market initialization with various configurations
- Token validation and restricted ticker handling
- Position lifecycle (open, update, close)
- Fee collection and distribution
- Liquidity provision and removal

### Edge Cases
- Maximum/minimum values for all parameters
- Boundary conditions (e.g., tick edges)
- Invalid input rejection
- Duplicate operation prevention

## Best Practices

1. **Keep tests focused**: Each test should verify one specific behavior
2. **Use descriptive names**: Test names should clearly indicate what is being tested
3. **Minimize dependencies**: Unit tests should not depend on external state
4. **Use the common infrastructure**: Leverage utilities in `common/` for consistency
5. **Document complex tests**: Add comments explaining the test scenario
6. **Clean up resources**: Ensure tests clean up any resources they create
7. **Avoid test interdependencies**: Tests should run successfully in any order
8. **Test both success and failure cases**: Verify error conditions are handled correctly

## Maintenance

The test infrastructure uses a modular justfile system:
- Test commands are defined in `programs/feels/tests/justfile`
- Commands are imported into the root `justfile` for convenient access
- All tests run within the Nix environment for consistency

Test reorganization has been completed and the structure is now stable. When adding new tests, follow the established patterns and directory structure documented above.
