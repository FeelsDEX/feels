# Feels Protocol Simulation Framework

The simulation framework provides utilities for testing the Feels Protocol in a controlled environment.

## Structure

```
simulation/
├── src/
│   ├── lib.rs                  # Main simulation module
│   ├── test_environment.rs     # Test environment setup
│   ├── account_factory.rs      # Test account creation
│   ├── token_factory.rs        # Token creation and minting
│   ├── pool_factory.rs         # Pool creation utilities
│   ├── liquidity_simulator.rs  # Liquidity operation simulation
│   ├── swap_simulator.rs       # Swap operation simulation
│   └── scenario_runner.rs      # High-level test scenarios
```

## Features

### Test Environment
- Initializes a local Solana test validator
- Sets up the Feels Protocol
- Provides utilities for advancing time and funding accounts

### Account Factory
- Creates funded test accounts
- Specialized account types (traders, liquidity providers)

### Token Factory
- Creates SPL Token-2022 tokens
- Creates Feels tokens with metadata
- Minting utilities
- FeelsSOL issuance

### Pool Factory
- Creates pools with different fee tiers
- Price initialization
- Multiple pool creation

### Liquidity Simulator
- Add liquidity at specific price ranges
- Full-range liquidity positions
- Remove liquidity
- Fee collection

### Swap Simulator
- Exact input swaps
- Exact output swaps
- Multi-hop swaps
- Arbitrage simulation
- Price impact analysis

### Scenario Runner
- Pre-built test scenarios
- Basic AMM operations
- Complex liquidity scenarios
- Stress testing
- Multi-pool arbitrage

## Usage

```rust
use feels_simulation::ScenarioRunner;

// Run a basic AMM scenario
let mut runner = ScenarioRunner::new().await?;
let result = runner.run_basic_amm_scenario().await?;

// Custom scenario
let mut env = TestEnvironment::new().await?;
env.initialize_protocol().await?;

let mut account_factory = AccountFactory::new(&mut env);
let trader = account_factory.create_trader().await?;

let mut token_factory = TokenFactory::new(&mut env);
let token = token_factory.create_feels_token(
    "Test Token".to_string(),
    "TEST".to_string(),
    9,
    &creator,
).await?;

// ... continue building your test scenario
```

## Usage with Tests

The simulation framework can be used together with the SDK for comprehensive testing in your own test files.