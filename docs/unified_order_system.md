# Unified Order System

## Overview

The Feels Protocol has been refactored to use a single unified order system for ALL trading operations. This replaces the previous architecture where different operations (swaps, position flows, entry/exit) had separate instruction handlers.

## Architecture Changes

### Before (Multiple Entry Points)
- `swap.rs` - Token swaps
- `entry_exit.rs` - JitoSOL ↔ FeelsSOL conversion
- `position_flows.rs` - Position transitions
- `order.rs` - Liquidity and limit orders

### After (Single Entry Point)
- `order.rs` - ALL operations through unified order system

## Order Types

The unified system supports all trading operations through the `OrderType` enum:

```rust
pub enum OrderType {
    /// Token-to-token swap (immediate execution)
    Swap {
        route: Vec<Pubkey>,
        min_amount_out: u64,
        zero_for_one: Vec<bool>,
    },
    
    /// Enter a position from FeelsSOL
    EnterPosition {
        position_type: PositionType,
        min_position_tokens: u64,
    },
    
    /// Exit a position to FeelsSOL
    ExitPosition {
        position_mint: Pubkey,
        min_feelssol_out: u64,
    },
    
    /// Convert between positions via FeelsSOL hub
    ConvertPosition {
        source_position: Pubkey,
        target_position_type: PositionType,
        min_tokens_out: u64,
    },
    
    /// Add liquidity to a pool
    AddLiquidity {
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    },
    
    /// Remove liquidity from a pool
    RemoveLiquidity {
        liquidity: u128,
        min_amounts: [u64; 2],
    },
    
    /// Place a limit order
    LimitOrder {
        sqrt_price_limit: u128,
        zero_for_one: bool,
        expiration: Option<i64>,
    },
}
```

## Benefits

1. **Consistency**: All operations follow the same validation, fee calculation, and execution paths
2. **Simplicity**: Single entry point reduces complexity for SDK and client integration
3. **Maintainability**: Less code duplication and easier to add new features
4. **Security**: Centralized validation and access control
5. **Composability**: Easier to build complex operations on top

## SDK Usage

The SDK has been updated with a new unified interface in `crates/sdk/src/instructions/order.rs`:

```rust
// Token swap
let ix = order::swap(
    &program_id,
    &market_field,
    &user,
    &user_token_0,
    &user_token_1,
    &market_token_0,
    &market_token_1,
    route,
    amount_in,
    min_amount_out,
    zero_for_one,
);

// Enter position
let ix = order::enter_position(
    &program_id,
    &market_field,
    &user,
    &user_feelssol,
    &position_mint,
    position_type,
    amount_in,
    min_position_tokens,
);

// Add liquidity
let ix = order::add_liquidity(
    &program_id,
    &market_field,
    &user,
    &user_token_0,
    &user_token_1,
    tick_lower,
    tick_upper,
    liquidity,
    amount,
);
```

## Entry/Exit System

Entry and exit from the system (JitoSOL ↔ FeelsSOL) now uses the swap order type at the hub pool:

```rust
// Enter: JitoSOL -> FeelsSOL
let ix = order::enter_system(
    &program_id,
    &hub_pool,
    &user,
    amount_jitosol,
    min_feelssol,
);

// Exit: FeelsSOL -> JitoSOL
let ix = order::exit_system(
    &program_id,
    &hub_pool,
    &user,
    amount_feelssol,
    min_jitosol,
);
```

## Migration Notes

- All existing SDK code using separate instruction builders should migrate to the unified `order::*` functions
- The old instruction modules have been removed from the program
- Account contexts remain similar but are now unified under the `Order` context

## Implementation Status

✅ Unified order.rs with all operation types
✅ Removed duplicate instruction handlers
✅ Updated lib.rs to expose single order handler
✅ Moved implementation logic into execution functions
✅ Updated SDK with unified interface
⏳ Testing and validation pending