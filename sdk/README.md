# Feels Protocol SDK

The Feels SDK provides a convenient interface for interacting with the Feels thermodynamic AMM protocol on Solana.

## Features

- **Client Interface**: High-level client for protocol interactions
- **Hub-Constrained Router**: Automatic route finding through FeelsSOL hub token
- **Instruction Builders**: Type-safe instruction construction with proper discriminators
- **PDA Utilities**: Helper functions for deriving program addresses
- **Error Handling**: Comprehensive error types for all SDK operations

## Structure

```
sdk/
├── src/
│   ├── client.rs       # Main FeelsClient interface
│   ├── config.rs       # SDK configuration
│   ├── error.rs        # Error types
│   ├── instructions.rs # Instruction builders
│   ├── router.rs       # Hub-constrained router
│   ├── types.rs        # Common types
│   └── utils.rs        # Utility functions
└── examples/
    ├── basic_usage.rs    # Simple SDK demonstration
    └── complete_flow.rs  # Comprehensive example
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
feels-sdk = { path = "../path/to/feels-solana/sdk" }
```

## Quick Start

```rust
use feels_sdk::{FeelsClient, SdkConfig};
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let payer = Keypair::new();
    let config = SdkConfig::localnet(payer);
    let client = FeelsClient::new(config)?;
    
    // Enter FeelsSOL (deposit JitoSOL)
    let sig = client.enter_feelssol(
        &user_jitosol_account,
        &user_feelssol_account,
        &jitosol_mint,
        &feelssol_mint,
        1_000_000_000, // 1 JitoSOL
    ).await?;
    
    println!("Transaction: {}", sig);
    Ok(())
}
```

## Hub-Constrained Routing

All swaps in Feels must route through the FeelsSOL hub token:

```rust
use feels_sdk::{HubRouter, PoolInfo};

let mut router = HubRouter::new(feelssol_mint);

// Add pool (must include FeelsSOL)
router.add_pool(PoolInfo {
    address: pool_address,
    token_a: usdc_mint,
    token_b: feelssol_mint,
    fee_rate: 30, // 0.3%
})?;

// Find route
let route = router.find_route(&usdc_mint, &sol_mint)?;
match route {
    Route::Direct { from, to } => {
        println!("Direct: {} -> {}", from, to);
    }
    Route::TwoHop { from, intermediate, to } => {
        println!("Two-hop: {} -> {} -> {}", from, intermediate, to);
    }
}
```

## Available Operations

### Market Operations
- `initialize_market` - Create new CLMM market
- `swap` - Execute token swaps with tick arrays
- `open_position` - Create liquidity position
- `close_position` - Remove liquidity position
- `collect_fees` - Harvest accumulated trading fees

### Oracle Operations
- `initialize_oracle` - Create price oracle for market
- `observe_oracle` - Query historical price data

### FeelsSOL Operations  
- `enter_feelssol` - Deposit JitoSOL to mint FeelsSOL
- `exit_feelssol` - Burn FeelsSOL to redeem JitoSOL

## Examples

### Basic Usage

```bash
cargo run --example basic_usage
```

Demonstrates:
- SDK configuration
- PDA derivation
- Hub router setup
- Basic route finding

### Complete Flow

```bash
cargo run --example complete_flow
```

Comprehensive example showing:
- Full client setup
- Token configuration  
- Advanced routing scenarios
- Transaction examples (dry run)
- Error handling patterns

## PDA Derivation

```rust
use feels_sdk::{find_market_address, find_buffer_address};

// Derive market PDA
let (market, bump) = find_market_address(&token_0, &token_1);

// Derive buffer PDA
let (buffer, bump) = find_buffer_address(&market);

// Sort tokens for consistent ordering
let (sorted_0, sorted_1) = sort_tokens(token_a, token_b);
```

## Error Handling

The SDK provides comprehensive error types:

```rust
match client.swap(...).await {
    Ok(sig) => println!("Success: {}", sig),
    Err(SdkError::SlippageExceeded { expected, actual }) => {
        println!("Slippage too high: {} vs {}", expected, actual);
    }
    Err(SdkError::InvalidRoute(msg)) => {
        println!("Invalid route: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

Run all tests:

```bash
cargo test -p feels-sdk
```

Run specific test module:

```bash
cargo test -p feels-sdk router
```