# Feels Jupiter Adapter

This crate provides the Jupiter AMM interface implementation for the Feels Protocol concentrated liquidity AMM, enabling Jupiter aggregator to route swaps through Feels markets with accurate cross-tick price calculations.

## Overview

The adapter implements the `jupiter_amm_interface::Amm` trait for Feels markets, allowing Jupiter to:
- Discover Feels concentrated liquidity markets
- Calculate accurate quotes using tick array data
- Generate swap transaction instructions
- Route swaps through the hub-and-spoke FeelsSOL model
- Handle complex multi-tick price movements

## Architecture

### FeelsAmm

The main component implementing the Jupiter AMM interface with concentrated liquidity support:

```rust
pub struct FeelsAmm {
    /// Market account public key
    key: Pubkey,
    /// Deserialized market state from on-chain account
    market: Market,
    /// Market authority PDA that controls vault operations
    authority: Pubkey,
    /// Feels program ID
    program_id: Pubkey,
    /// Token mints for the trading pair [token_0, token_1]
    reserve_mints: [Pubkey; 2],
    /// Current token reserves in vaults [vault_0_amount, vault_1_amount]
    reserves: [u64; 2],
    /// Token vault addresses
    vault_0: Pubkey,
    vault_1: Pubkey,
    /// Tick spacing for this market (determines price granularity)
    tick_spacing: u16,
    /// Cached tick array views for liquidity calculations
    tick_arrays: AHashMap<i32, TickArrayView>, // start_index -> view
    /// Public keys of tick arrays to monitor for updates
    tick_array_keys: Vec<Pubkey>,
}
```

### TickArrayView

Cached representation of on-chain tick arrays for efficient quote calculations:

```rust
struct TickArrayView {
    start_tick_index: i32,                   // Array starting tick
    inits: AHashMap<i32, i128>,             // Sparse map of initialized ticks
}
```

### Key Methods

1. **from_keyed_account**: Deserializes market account and derives PDAs
2. **update**: Refreshes vault balances and tick array cache from account data
3. **quote**: Simulates concentrated liquidity swaps across multiple ticks
4. **get_swap_and_account_metas**: Generates complete account list for swap instruction

## Quote Calculation Process

The adapter performs sophisticated concentrated liquidity calculations:

1. **Initialize State**: Load market state and current liquidity/price
2. **Tick Traversal**: Simulate crossing ticks and applying liquidity changes
3. **Price Impact**: Calculate output using Orca Whirlpools math primitives
4. **Fee Calculation**: Apply base fees + impact fees based on ticks moved
5. **Account Generation**: Build complete account list for swap execution

### Concentrated Liquidity Features

- **Cross-Tick Quotes**: Accurately handles swaps that cross multiple price ranges
- **Liquidity Tracking**: Maintains sparse tick array cache for efficient calculations  
- **Impact Fees**: Calculates dynamic fees based on price movement (up to 25% cap)
- **Hub-and-Spoke**: Supports routing through FeelsSOL for token-to-token swaps

## Usage

### For Jupiter Integration

Jupiter automatically discovers and uses Feels markets:

1. **Discovery**: Finds Feels market accounts using program ID filter
2. **Initialization**: Creates FeelsAmm instances with tick array monitoring
3. **Routing**: Includes Feels in multi-hop route calculations
4. **Execution**: Generates proper swap instructions with all required accounts

### Example Integration

```rust
use feels_jupiter_adapter::FeelsAmm;
use jupiter_amm_interface::{Amm, KeyedAccount, QuoteParams};

// Jupiter creates AMM from discovered market account
let feels_amm = FeelsAmm::from_keyed_account(&market_account, &context)?;

// Update with latest on-chain data
feels_amm.update(&account_map)?;

// Get concentrated liquidity quote
let quote = feels_amm.quote(&QuoteParams {
    amount: 1_000_000,
    input_mint: usdc_mint,
    output_mint: feelssol_mint,
})?;

// Generate swap instruction accounts
let SwapAndAccountMetas { swap, account_metas } = 
    feels_amm.get_swap_and_account_metas(&swap_params)?;
```

## Technical Implementation

### Concentrated Liquidity Math

Uses Orca Whirlpools core primitives for accurate calculations:
- `try_get_next_sqrt_price_from_a/b`: Price movement within liquidity ranges
- `try_get_amount_delta_a/b`: Input/output amounts for price changes
- Uniswap V3 tick crossing conventions for liquidity updates

### Tick Array Management

- **Sparse Representation**: Only stores initialized ticks to save memory
- **Efficient Lookup**: HashMap-based tick search across multiple arrays
- **PDA Derivation**: Generates tick array addresses around current price
- **Cache Management**: Updates tick data when Jupiter refreshes accounts

### Fee Structure

- **Base Fee**: Configurable per-market base fee (e.g., 30 bps)
- **Impact Fee**: Dynamic fee based on ticks crossed (linear, capped at 25%)
- **Total Fees**: Combined base + impact, applied to output amount

## Current Status

**Implemented Features**:
- Jupiter AMM interface compliance
- Concentrated liquidity quote calculations  
- Cross-tick swap simulation
- Dynamic impact fee calculation
- Complete account metadata generation
- Tick array caching and management
- Hub-and-spoke routing support

**Limitations**:
- Exact-out swaps use exact-in simulation (close approximation)
- Tick array coverage limited to ±2 arrays from current price
- Impact fee uses simplified linear model

## Integration Notes

### For Jupiter Developers

The adapter is designed to work seamlessly with Jupiter's routing engine:
- Implements all required `Amm` trait methods
- Provides accurate quotes for route optimization
- Generates complete instruction account lists
- Handles Feels-specific PDA derivations automatically

### For Feels Protocol

The adapter maintains compatibility with the core protocol:
- Uses standard Feels swap instruction format
- Respects market pause states and validation
- Integrates with protocol fee distribution
- Supports all market configurations (tick spacing, fees, etc.)

## Fee Account Handling

**CRITICAL**: The adapter must provide correct fee accounts to prevent swap failures.

### Protocol Treasury Account

The protocol treasury must be an Associated Token Account (ATA) owned by `protocol_config.treasury`:

1. The treasury pubkey is stored in the on-chain ProtocolConfig account
2. For each output token, the treasury ATA is derived as:
   ```rust
   spl_associated_token_account::get_associated_token_address(
       &protocol_config.treasury,
       &output_mint
   )
   ```
3. Swaps will fail with `InvalidAuthority` if the treasury account owner doesn't match

### Creator Fee Accounts

Creator fees only apply to protocol-minted tokens:

1. Check if input token has a ProtocolToken registry entry
2. If yes, creator fees go to `protocol_token.creator`'s ATA for output token
3. Creator account can be `None` for non-protocol tokens

### Initialization

Before using the adapter in production, configure it with the correct treasury:

```rust
use feels_jupiter_adapter::config::{set_treasury, add_protocol_token};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

// Set the protocol treasury (must match on-chain ProtocolConfig)
let treasury = Pubkey::from_str("ACTUAL_TREASURY_PUBKEY").unwrap();
feels_jupiter_adapter::config::set_treasury(treasury);

// Register known protocol tokens for creator fees
let protocol_token = Pubkey::from_str("PROTOCOL_TOKEN_MINT").unwrap();
feels_jupiter_adapter::config::add_protocol_token(protocol_token);
```

### Jupiter Fee Override

Jupiter can provide fee accounts via `quote_mint_to_referrer`:
- If provided, these override the default treasury/creator derivations
- Useful for custom fee routing or referral programs
- Must still pass on-chain validation

### Common Errors

- **InvalidAuthority**: Treasury account owner ≠ protocol_config.treasury
- **InvalidMint**: Treasury account mint ≠ output token mint
- **Missing account**: Creator account not provided for protocol token

### Testing Fee Handling

The adapter includes tests to verify correct fee account generation:
```bash
cargo test test_fee_account_handling
```