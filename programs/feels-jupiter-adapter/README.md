# Feels Jupiter Adapter

This crate provides the Jupiter AMM interface implementation for the Feels Protocol, enabling Jupiter aggregator to route swaps through Feels markets.

## Overview

The adapter implements the `jupiter_amm_interface::Amm` trait for Feels markets, allowing Jupiter to:
- Discover Feels markets
- Get quotes for swaps
- Build swap transactions
- Route user swaps through the most efficient path

## Architecture

### FeelsAmm

The main component is the `FeelsAmm` struct which implements the Jupiter AMM interface:

```rust
pub struct FeelsAmm {
    key: Pubkey,                    // Market address
    market: Market,                 // Feels market state
    authority: Pubkey,             // Market authority PDA
    program_id: Pubkey,            // Feels program ID
    reserve_mints: [Pubkey; 2],    // Token mints
    reserves: [u64; 2],            // Token reserves
    vault_0: Pubkey,               // Vault for token 0
    vault_1: Pubkey,               // Vault for token 1
}
```

### Key Methods

1. **from_keyed_account**: Deserializes a Feels market account into a FeelsAmm instance
2. **update**: Updates reserve balances from vault accounts
3. **quote**: Calculates swap output amounts and fees
4. **get_swap_and_account_metas**: Builds the instruction accounts for swap execution

## Usage

### For Jupiter Integration

Jupiter will automatically discover and use Feels markets by:

1. Finding all Feels market accounts on-chain
2. Creating FeelsAmm instances for each market
3. Including Feels markets in its routing algorithm
4. Executing swaps through the standard Feels swap instruction

### Example

```rust
use feels_jupiter_adapter::FeelsAmm;
use jupiter_amm_interface::{Amm, KeyedAccount, QuoteParams};

// Create AMM from market account
let feels_amm = FeelsAmm::from_keyed_account(&keyed_account, &amm_context)?;

// Get a quote
let quote = feels_amm.quote(&QuoteParams {
    amount: 1_000_000,
    input_mint: token_0_mint,
    output_mint: token_1_mint,
    swap_mode: SwapMode::ExactIn,
})?;

// Build swap transaction
let swap_accounts = feels_amm.get_swap_and_account_metas(&swap_params)?;
```

## Integration Status

- Implements Jupiter AMM interface v0.6.0
- Compatible with Feels Protocol concentrated liquidity
- Supports exact input swaps
- Calculates fees based on market parameters
- Provides account metadata for swap execution

## Future Enhancements

- [ ] Implement exact output swaps
- [ ] Add dynamic fee calculations based on volatility
- [ ] Support for multi-hop routes through FeelsSOL hub
- [ ] Integration with Jupiter's priority fee system