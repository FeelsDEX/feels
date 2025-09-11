# Feels Protocol Test Suite

This directory contains comprehensive tests for the Feels protocol, organized by test type and functionality.

## Overview

The test suite has been refactored to support multiple testing environments:
- **In-Memory**: Fast unit tests using `solana-program-test` 
- **Devnet**: Integration tests against devnet (requires `RUN_DEVNET_TESTS=1`)
- **Localnet**: Integration tests against local validator

The test suite provides coverage for:
- Mathematical operations and utilities (unit tests)
- Instruction validation and constraints (unit tests)  
- Protocol lifecycle and cross-instruction interactions (integration tests)
- AMM operations and token management (functional tests)
- Security vulnerability regression tests

## Module Structure

```
tests/
├── common/               # Shared test infrastructure
│   ├── mod.rs           # Module organization
│   ├── client.rs        # Test client abstraction (InMemory/Devnet)
│   ├── context.rs       # Test context and pre-configured accounts
│   ├── environment.rs   # Environment configuration
│   ├── helpers.rs       # High-level test helpers
│   ├── builders.rs      # Test data builders
│   ├── macros.rs        # Test macros for multi-environment testing
│   └── time.rs          # Time manipulation utilities
├── unit/                # Unit tests for individual components
│   ├── instructions/    # Instruction-specific tests
│   ├── state/           # State struct tests
│   └── test_*.rs        # Component unit tests
├── integration/         # Integration tests for features
│   ├── basic_test.rs    # Basic integration tests
│   ├── initialize_market_test.rs # Market initialization
│   └── test_swap_scenarios.rs    # Swap integration tests
├── e2e/                 # End-to-end scenario tests
│   ├── test_full_trading_flow.rs # Complete trading lifecycle
│   ├── test_position_metadata.rs # NFT position tests
│   └── test_token_lifecycle.rs   # Token lifecycle tests
├── property/            # Property-based tests
│   └── fuzz_*.rs        # Fuzz tests for invariants
└── README.md            # This file
```

## Quick Start

### Prerequisites

1. **Build the BPF program first**:
```bash
cargo build-sbf
```

2. The tests will automatically find the BPF binary in `target/deploy/feels.so`

### Running Tests

```bash
# Using nix environment (recommended)
nix develop --command cargo test

# Or use the test runner script
./run-tests.sh

# Run specific test categories
cargo test --test mod unit::        # Unit tests only
cargo test --test mod integration:: # Integration tests only  
cargo test --test mod e2e::        # E2E tests only
cargo test --test mod property::   # Property tests only

# Run specific test
cargo test --test mod test_simple_example

# Run with output
cargo test --test mod test_simple_example -- --nocapture
```

### Writing Tests

The test infrastructure provides macros for environment-specific testing:

```rust
use crate::common::*;

// Run test in in-memory environment only  
test_in_memory!(test_name, |ctx: TestContext| async move {
    // Test logic here
    Ok::<(), Box<dyn std::error::Error>>(())
});

// Run test in all environments
test_all_environments!(test_name, |ctx: TestContext| async move {
    // Test logic here
    Ok::<(), Box<dyn std::error::Error>>(())
});

// Run test on devnet only
test_devnet!(test_name, |ctx: TestContext| async move {
    // Test logic here
    Ok::<(), Box<dyn std::error::Error>>(())
});
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

## Key Features

### Mathematical Test Coverage
- Safe arithmetic operations with overflow/underflow protection
- U256 big integer operations and precision testing
- Tick-price conversion accuracy and bounds checking
- Liquidity math validation and delta calculations
- Fee calculation precision and rounding behavior

### Instruction Validation Tests
- PDA derivation correctness and determinism
- Account constraint validation
- Parameter validation and bounds checking
- Authority and ownership verification

### Integration Testing
- Protocol initialization sequence validation
- Cross-instruction state consistency
- Token validation system integration
- Multi-step operation workflows

### Security and Vulnerability Testing
- Overflow/underflow detection in math operations
- Input validation and sanitization
- Access control and authorization checks
- Edge case and boundary condition testing

## Testing Patterns

### 1. Mathematical Property Testing
```rust
#[test]
fn test_math_property_invariants() {
    // Test that mathematical operations maintain invariants
    // Example: Addition overflow detection
    assert!(add_u64(u64::MAX, 1).is_err());
    
    // Example: Rounding consistency
    assert!(round_up(x) >= round_down(x));
}
```

### 2. Integration Workflow Testing
```rust
#[test]
fn test_protocol_integration_workflow() {
    // 1. Initialize protocol
    // 2. Create FeelsSOL wrapper
    // 3. Create token pools
    // 4. Validate cross-component interactions
    // 5. Verify state consistency
}
```

### 3. Security Regression Testing
```rust
#[test]
fn test_vulnerability_fixes() {
    // Test that previously identified vulnerabilities remain fixed
    // Example: U256 overflow protection
    let result = U256::MAX.checked_add(&U256::from(1u128));
    assert!(result.is_none());
}
```

## Test Coverage Areas

### Unit Tests (`tests/unit/`)
- **math_operations.rs**: Safe arithmetic, U256 operations, precision testing
- **math_tick.rs**: Tick math conversions and boundary conditions  
- **instruction_validation.rs**: PDA derivation, account constraints

### Integration Tests (`tests/integration/`)
- **protocol_lifecycle.rs**: End-to-end protocol initialization workflows
- **token_validate.rs**: Token ticker validation system integration

### Functional Tests (`tests/functional/`)
- **amm_operations.rs**: Complete AMM functionality from pool creation to trading

## Token Ticker Validation Testing

The test suite includes comprehensive coverage of the token ticker validation system:

```rust
#[test]
fn test_token_create_with_restricted_ticker() {
    // Test that SOL, USDC, USDT, FeelsSOL are properly restricted
    let result = validate_ticker_format("SOL");
    assert!(result.is_err());
    
    // Test alternatives are suggested
    let alternatives = get_ticker_alternatives("SOL");
    assert!(alternatives.contains(&"SOL2".to_string()));
}
```

## Running Specific Tests

### Mathematical Tests
```bash
# Test all mathematical operations
cargo test math_operations

# Test specific mathematical components
cargo test safe_math
cargo test u256_operations  
cargo test tick_math
cargo test precision
```

### Integration Tests
```bash
# Test protocol initialization
cargo test protocol_lifecycle

# Test token validation system
cargo test token_validate

# Test all integration workflows
cargo test integration
```

### Security and Vulnerability Tests
```bash
# Run security regression tests
cargo test vulnerability_regression

# Test overflow protection
cargo test overflow_protection

# Test access control
cargo test authorization
```

## Troubleshooting

### Common Issues

1. **Tests fail with "Program processor not available"**
   - Make sure you've built the BPF program: `cargo build-sbf`
   - The test infrastructure will automatically find the binary in `target/deploy/feels.so`

2. **Tests fail with "KeypairPubkeyMismatch"**
   - This usually indicates an issue with account ownership or signing
   - Check that the correct keypair is being used for signing
   - Ensure account ownership is properly set up

3. **Environment-specific test issues**
   - For devnet tests: Set `RUN_DEVNET_TESTS=1`
   - For localnet tests: Set `RUN_LOCALNET_TESTS=1`
   - Devnet/localnet tests require appropriate network connectivity

4. **Compilation errors**
   - Ensure all dependencies are built: `cargo build`
   - Check that the SDK is properly built
   - Verify Anchor version matches: `anchor --version`

## Contributing

When adding new tests:
1. **Follow the directory structure**: Unit tests in `unit/`, integration tests in `integration/`, e2e tests in `e2e/`
2. **Use the test macros**: `test_in_memory!` for fast tests, `test_all_environments!` for comprehensive testing
3. **Use descriptive test names**: `test_specific_functionality_condition()`
4. **Add comprehensive documentation**: Explain what the test validates
5. **Include edge cases**: Test boundary conditions and error cases
6. **Update this README**: Document new test categories and coverage areas