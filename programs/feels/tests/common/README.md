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
- `test_all_environments!`: Run test in all available environments
- `test_in_memory!`: Run test only in-memory
- `test_devnet!`: Run test only on devnet
- `assert_instruction_error!`: Assert specific errors

## Usage Examples

### Basic Market Creation and Swap

```rust
#[test_in_memory!(test_basic_swap)]
async fn test_basic_swap(ctx: TestContext) -> TestResult<()> {
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

### Multi-Environment Testing

```rust
#[test_all_environments!(test_position_management)]
async fn test_position_management(ctx: TestContext) -> TestResult<()> {
    // This test will run in:
    // - In-memory (always)
    // - Devnet (if RUN_DEVNET_TESTS=1)
    // - Localnet (if RUN_LOCALNET_TESTS=1)
    
    // Test implementation...
}
```

### Complex Scenarios

```rust
#[test_in_memory!(test_sandwich_attack)]
async fn test_sandwich_attack(ctx: TestContext) -> TestResult<()> {
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
#[with_time_test!(test_twap)]
async fn test_twap(ctx: &TestContext) -> TestResult<()> {
    // Create market...
    
    // Simulate price movement over time
    TimeScenarios::simulate_price_movement(
        ctx,
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

### Devnet Tests
```bash
RUN_DEVNET_TESTS=1 cargo test -- --ignored
```

### Localnet Tests
```bash
# Start local validator first
RUN_LOCALNET_TESTS=1 cargo test -- --ignored
```

### Custom RPC Endpoint
```bash
TEST_RPC_URL=http://custom-rpc:8899 cargo test -- --ignored
```

## Environment Detection

The test infrastructure automatically detects the environment based on:
1. `TEST_RPC_URL` env var → Use custom RPC endpoint
2. `RUN_DEVNET_TESTS=1` → Use devnet
3. Default → In-memory testing

## Best Practices

1. **Use builders for complex setups**: Builders provide a clean API and handle common setup tasks
2. **Prefer helpers over raw SDK calls**: Helpers include proper error handling and result parsing
3. **Test in multiple environments**: Use `test_all_environments!` for critical functionality
4. **Use time utilities for TWAP/oracle tests**: Ensures consistent behavior across environments
5. **Leverage pre-configured accounts**: Use `ctx.accounts` for consistent test identities

## Integration with devnet.nix

When testing against localnet started with `devnet.nix`:
1. Set `RUN_LOCALNET_TESTS=1`
2. The infrastructure will automatically connect to `http://localhost:8899`
3. Program deployment and account setup is handled by devnet.nix
4. Test accounts are automatically funded via airdrop

## Extending the Infrastructure

To add new functionality:
1. Add methods to relevant helpers
2. Create new builders for complex patterns
3. Add new test macros for common assertions
4. Update this README with examples