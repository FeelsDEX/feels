# Feels Protocol Test Structure

This document describes the organization and conventions for the Feels Protocol test suite.

## Directory Structure

```
tests/
├── common/                 # Shared test infrastructure
├── unit/                   # Unit tests for individual components
├── integration/            # Integration tests for feature workflows
├── e2e/                    # End-to-end tests for complete user flows
└── property/               # Property-based and fuzz tests
```

## Test Categories

### Unit Tests (`unit/`)
Tests for individual components in isolation.

**Subdirectories:**
- `math/` - Mathematical operations (fees, rounding, dust control)
- `security/` - Security features (reentrancy, race conditions, griefing)
- `pomm/` - POMM (Protocol-Owned Market Making) logic
- `buffer/` - Buffer management and overflow handling
- `position/` - Position management and safety
- `oracle/` - Oracle and observation logic

### Integration Tests (`integration/`)
Tests for feature workflows that involve multiple components.

**Subdirectories:**
- `initialization/` - Market initialization variations
- `liquidity/` - Liquidity provision patterns
- `debug/` - Debug and diagnostic tests
- `token/` - Token lifecycle operations

### End-to-End Tests (`e2e/`)
Complete user journey tests that simulate real usage.

**Examples:**
- Full trading flows
- Token lifecycle (mint → trade → burn)
- Position management workflows
- FeelsSOL entry/exit flows

### Property Tests (`property/`)
Fuzz testing and invariant checking.

**Current tests:**
- Swap invariants
- Mathematical properties

## Naming Conventions

### File Names
- All test files should follow the pattern: `test_*.rs`
- Use descriptive names that indicate what is being tested
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
Each subdirectory should have a `mod.rs` that exports all test modules:
```rust
// unit/math/mod.rs
pub mod test_fee_growth;
pub mod test_fee_rounding;
pub mod test_dust_control;
```

## Adding New Tests

### 1. Determine the Category
- **Unit**: Testing a single function or component?
- **Integration**: Testing a feature that uses multiple components?
- **E2E**: Testing a complete user workflow?
- **Property**: Testing invariants or properties with random inputs?

### 2. Find or Create the Appropriate Subdirectory
- For unit tests, use the component category (math, security, etc.)
- For integration tests, use the feature category (initialization, liquidity, etc.)
- For e2e tests, place directly in the `e2e/` directory
- For property tests, place directly in the `property/` directory

### 3. Follow the Template
```rust
use crate::common::*;  // Import common test infrastructure

#[test]
fn test_my_feature() {
    // Setup
    let mut context = TestContext::new();
    
    // Execute
    let result = my_function();
    
    // Assert
    assert!(result.is_ok());
}
```

## Common Test Infrastructure

The `common/` directory provides shared utilities:

- `assertions.rs` - Custom assertion helpers
- `builders.rs` - Test data builders
- `client.rs` - Test client abstraction
- `context.rs` - Test context and environment setup
- `fixtures.rs` - Reusable test data
- `helpers.rs` - High-level test helpers
- `macros.rs` - Test macros
- `time.rs` - Time manipulation for tests
- `tracing.rs` - Test output and debugging

## Running Tests

```bash
# Run all tests
cargo test

# Run specific category
cargo test --test unit
cargo test --test integration
cargo test --test e2e

# Run specific subdirectory
cargo test unit::math
cargo test integration::initialization

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_swap_exact_amount
```

## Best Practices

1. **Keep tests focused**: Each test should verify one specific behavior
2. **Use descriptive names**: Test names should clearly indicate what is being tested
3. **Minimize dependencies**: Unit tests should not depend on external state
4. **Use the common infrastructure**: Leverage the utilities in `common/` for consistency
5. **Document complex tests**: Add comments explaining the test scenario
6. **Clean up resources**: Ensure tests clean up any resources they create
7. **Avoid test interdependencies**: Tests should be able to run in any order

## Maintenance Scripts

- `reorganize_tests.sh` - Reorganizes test files according to this structure
- `analyze_tests.sh` - Analyzes current test structure and generates report
- `consolidate_dummy_tests.sh` - Consolidates related tests into single files

Run with `--dry-run` flag to preview changes without modifying files.