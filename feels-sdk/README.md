# Feels Protocol SDK

A modern, service-based Rust SDK for interacting with the Feels Protocol concentrated liquidity AMM on Solana.

## Installation

```toml
[dependencies]
feels-sdk = "0.1.0"
```

## Architecture

The SDK is organized into four main modules:

- **`core`** - Core types, constants, and errors
- **`protocol`** - Protocol math, PDA derivation, and fee calculations  
- **`instructions`** - Type-safe instruction builders
- **`client`** - Service-based API for protocol interaction

## Quick Start

```rust
use feels_sdk::FeelsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = FeelsClient::new("https://api.mainnet-beta.solana.com").await?;
    
    // Get market info
    let market = client.market.get_market_by_tokens(&token_0, &token_1).await?;
    
    // Execute swap
    let result = client.swap.swap_exact_in(
        &signer,
        market.address,
        user_token_in,
        user_token_out,
        1_000_000,     // amount in
        950_000,       // min amount out
        Some(100),     // 1% slippage
    ).await?;
    
    Ok(())
}
```

## Services

### Market Service
- Find markets by token pair
- Get market state and oracle data
- Query liquidity and pricing

### Swap Service  
- Execute swaps (exact input/output)
- Simulate swaps without execution
- Estimate fees and find routes

### Liquidity Service
- Enter/exit FeelsSOL
- Open/close positions
- Initialize new markets

## Examples

See the [`examples/`](examples/) directory for complete usage examples:
- `basic_usage.rs` - Getting started with the SDK
- `swap_flow.rs` - Complete swap execution flow

## Features

- Async-first design
- Type-safe instruction building
- Automatic PDA derivation with caching
- Comprehensive error handling
- Zero-copy account parsing
- Service-oriented architecture