# Feels Protocol SDK

The Feels Protocol SDK provides a high-level interface for interacting with the Feels Protocol on Solana.

## Structure

```
sdk/
├── src/
│   ├── lib.rs              # Main SDK module
│   ├── client.rs           # Main client for protocol interaction
│   ├── config.rs           # SDK configuration
│   ├── errors.rs           # Error types
│   ├── types.rs            # Result and info types
│   ├── utils.rs            # PDA derivation utilities
│   └── instructions/       # Instruction builders
│       ├── mod.rs
│       ├── protocol.rs     # Protocol initialization
│       ├── pool.rs         # Pool operations
│       ├── liquidity.rs    # Liquidity management
│       ├── swap.rs         # Swap operations
│       └── token.rs        # Token operations
```

## Usage

```rust
use feels_sdk::{FeelsClient, SdkConfig};

// Initialize client
let config = SdkConfig::localnet(payer);
let client = FeelsClient::new(config);

// Initialize protocol
client.initialize_protocol(&authority, &emergency_authority).await?;

// Create a pool
let result = client.create_pool(
    &token_a,
    &token_b,
    30, // 0.3% fee
    sqrt_price,
).await?;

// Add liquidity
let liquidity = client.add_liquidity(
    &pool,
    &position_mint,
    amount_0,
    amount_1,
    amount_0_min,
    amount_1_min,
    tick_lower,
    tick_upper,
).await?;

// Execute swap
let swap = client.swap(
    &pool,
    amount_in,
    amount_out_min,
    sqrt_price_limit,
    is_base_input,
    is_exact_input,
).await?;
```

## Key Features

- High-level client interface
- Automatic PDA derivation
- Type-safe instruction building
- Error handling
- Network configuration support