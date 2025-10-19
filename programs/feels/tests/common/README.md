# Feels Protocol Test Infrastructure

## Overview

This test infrastructure provides a unified interface for testing the Feels protocol both in-memory (using `ProgramTest`) and against live networks (devnet/localnet).

## Key Components

### TestContext
The main entry point for all test operations. Provides access to:
- Unified client interface (`TestClient` trait)
- SDK integration
- Pre-configured test accounts
- Helper methods for common operations

### TestClient Trait
Abstracts over different execution environments:
- `InMemoryClient`: Uses Solana's `ProgramTest` for fast, deterministic testing
- `DevnetClient`: Uses RPC client for testing against live networks

### Helpers
High-level abstractions for common operations:
- `MarketHelper`: Market creation and management
- `SwapHelper`: Swap execution and analysis
- `PositionHelper`: Position lifecycle management

### Builders
Fluent APIs for complex test setups:
- `MarketBuilder`: Configure and create markets
- `SwapBuilder`: Build complex swap scenarios
- `PositionBuilder`: Create multiple positions
- `ScenarioBuilder`: Compose multi-step test scenarios

### Time Utilities
Tools for testing time-dependent features:
- Time advancement (in-memory only)
- TWAP testing scenarios
- Epoch transition testing

### Macros
Test macros for different execution patterns:
- `assert_tx_success!`: Assert transaction success with detailed output
- `assert_error!`: Assert specific program errors
- `assert_balance_change!`: Assert token balance changes
- Custom assertion traits for comprehensive validation

## Usage Examples

### Basic Market Creation and Swap

```rust
#[tokio::test]
async fn test_basic_swap() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    // Create tokens
    let token_0 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 9).await?;
    let token_1 = ctx.create_mint(&ctx.accounts.market_creator.pubkey(), 6).await?;
    
    // Create market with liquidity
    let market = ctx.market_builder()
        .token_0(token_0.pubkey())
        .token_1(token_1.pubkey())
        .add_full_range_liquidity(ctx.accounts.alice.insecure_clone(), 1_000_000_000)
        .build()
        .await?;
    
    // Execute swap
    let result = ctx.swap_helper().swap(
        &market,
        &token_0.pubkey(),
        &token_1.pubkey(),
        100_000_000,
        &ctx.accounts.bob,
    ).await?;
    
    assert!(result.amount_out > 0);
    Ok(())
}
```

### Using Assertion Utilities

```rust
#[tokio::test]
async fn test_position_management() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    
    // Use assertion macros for comprehensive validation
    assert_tx_success!(result, "Position creation should succeed");
    assert_balance_change!(ctx, &user_account, expected_change);
    
    Ok(())
}
```

### Complex Scenarios

```rust
#[tokio::test]
async fn test_sandwich_attack() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    // Setup market...
    
    let results = ctx.swap_builder()
        .sandwich_attack(
            market,
            victim_keypair,
            attacker_keypair,
            token_in,
            token_out,
            victim_amount,
            front_run_amount,
        )
        .execute()
        .await?;
    
    // Analyze results...
}
```

### Time-Based Testing

```rust
#[tokio::test]
async fn test_twap() -> TestResult<()> {
    let ctx = TestContext::new(TestEnvironment::in_memory()).await?;
    // Create market...
    
    // Simulate price movement over time
    TimeScenarios::simulate_price_movement(
        &ctx,
        &market,
        vec![
            (60, new_price_1),   // After 60 seconds
            (120, new_price_2),  // After 120 seconds
        ],
    ).await?;
    
    // Verify TWAP calculations...
}
```

## Running Tests

### In-Memory Tests (Default)
```bash
cargo test
```

### All Tests
```bash
cargo test --features test-utils
```

### Specific Test Categories
```bash
cargo test --features test-utils unit::
cargo test --features test-utils integration::
cargo test --features test-utils e2e::
```

## Test Environment

The test infrastructure primarily uses in-memory testing with `solana-program-test` for:
- Fast execution
- Deterministic results
- No network dependencies
- Complete isolation

## Best Practices

1. **Use builders for complex setups**: Builders provide a clean API and handle common setup tasks
2. **Prefer helpers over raw SDK calls**: Helpers include proper error handling and result parsing
3. **Use assertion utilities**: Leverage `assert_tx_success!`, `assert_error!`, and custom traits
4. **Use time utilities for TWAP/oracle tests**: Ensures consistent behavior in time-based scenarios
5. **Leverage pre-configured accounts**: Use `ctx.accounts` for consistent test identities

## Extending the Infrastructure

To add new functionality:
1. Add methods to relevant helpers
2. Create new builders for complex patterns
3. Add new test macros for common assertions
4. Update this README with examples