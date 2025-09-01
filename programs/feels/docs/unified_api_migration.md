# Unified API Migration Guide

## Overview

The Feels Protocol now provides a simplified unified API for all order operations. This guide explains how to migrate from the internal API to the unified API.

## API Structure

### Public API (Recommended)

These are the primary instructions that developers should use:

```rust
// Orders
pub fn order_unified(params: UnifiedOrderParams) -> Result<UnifiedOrderResult>
pub fn order_compute_unified(params: UnifiedComputeParams) -> Result<Tick3DArrayInfo>
pub fn order_modify_unified(params: UnifiedModifyParams) -> Result<()>

// Cleanup
pub fn cleanup_tick_array_v2(params: CleanupTickArrayParams) -> Result<()>
pub fn cleanup_empty_tick_array() -> Result<()>
```

### Internal API (Deprecated for External Use)

These instructions are now considered internal and should not be used by external developers:

```rust
// INTERNAL ONLY - Use order_unified instead
pub fn order(params: OrderParams) -> Result<OrderResult>

// INTERNAL ONLY - Use order_compute_unified instead  
pub fn order_compute(params: OrderComputeParams) -> Result<Tick3DArrayInfo>

// INTERNAL ONLY - Use order_modify_unified instead
pub fn order_modify(params: OrderModifyParams) -> Result<()>

// DEPRECATED - Use cleanup_tick_array_v2 instead
pub fn cleanup_tick_array(params: CleanupTickArrayParams) -> Result<()>
```

## Migration Examples

### 1. Swap Order

**Old (Internal API):**
```rust
let params = OrderParams {
    amount: 1000000,
    rate_params: RateParams::TargetRate {
        sqrt_rate_limit: u128::MIN,
        is_token_a_to_b: true,
    },
    duration: Duration::Swap,
    leverage: 1_000_000,
    order_type: OrderType::Immediate,
    limit_value: 0,
};
program.order(ctx, params).await?;
```

**New (Unified API):**
```rust
let params = UnifiedOrderParams {
    amount: 1000000,
    config: OrderConfig::Swap {
        is_token_a_to_b: true,
        min_amount_out: 900000,
        sqrt_rate_limit: None,
    },
    advanced: None,
};
program.order_unified(ctx, params).await?;
```

### 2. Add Liquidity

**Old (Internal API):**
```rust
let params = OrderParams {
    amount: 1000000,
    rate_params: RateParams::RateRange {
        tick_lower: -1000,
        tick_upper: 1000,
    },
    duration: Duration::Swap,
    leverage: 1_000_000,
    order_type: OrderType::Liquidity,
    limit_value: 0,
};
program.order(ctx, params).await?;
```

**New (Unified API):**
```rust
let params = UnifiedOrderParams {
    amount: 1000000,
    config: OrderConfig::AddLiquidity {
        tick_lower: -1000,
        tick_upper: 1000,
        token_amounts: None,
    },
    advanced: None,
};
program.order_unified(ctx, params).await?;
```

### 3. Limit Order with Advanced Features

**New (Unified API):**
```rust
let params = UnifiedOrderParams {
    amount: 1000000,
    config: OrderConfig::LimitOrder {
        is_buy: true,
        target_sqrt_rate: target_rate,
        expiry: timestamp + 3600,
    },
    advanced: Some(AdvancedOrderParams {
        duration: Duration::Week,
        leverage: 2_000_000, // 2x
        mev_protection: Some(MevProtection {
            max_slippage_bps: 50,
            min_blocks_delay: 2,
            validator_signature: None,
        }),
        hook_data: None,
    }),
};
program.order_unified(ctx, params).await?;
```

## Benefits of Unified API

1. **Simplified Interface**: Single entry point for all order types
2. **Type Safety**: Enums ensure valid parameter combinations
3. **Extensibility**: Easy to add new order types
4. **Better Documentation**: Self-documenting parameter structures
5. **Reduced Errors**: Impossible to mix incompatible parameters

## SDK Updates

### TypeScript SDK

```typescript
// Old
await program.methods
  .order({
    amount: new BN(1000000),
    rateParams: { targetRate: { sqrtRateLimit: MIN_SQRT_RATE, isTokenAToB: true } },
    // ... many more fields
  })
  .accounts({ ... })
  .rpc();

// New
await program.methods
  .orderUnified({
    amount: new BN(1000000),
    config: { 
      swap: { 
        isTokenAToB: true, 
        minAmountOut: new BN(900000),
        sqrtRateLimit: null 
      } 
    },
    advanced: null,
  })
  .accounts({ ... })
  .rpc();
```

### Rust Client

```rust
// Import unified types
use feels_protocol::instructions::unified_order::*;

// Create order
let order = UnifiedOrderParams {
    amount: 1_000_000,
    config: OrderConfig::Swap {
        is_token_a_to_b: true,
        min_amount_out: 900_000,
        sqrt_rate_limit: None,
    },
    advanced: None,
};

// Execute
program.request()
    .accounts(accounts)
    .args(order)
    .send()?;
```

## Deprecation Timeline

1. **Current**: Both APIs available, unified API recommended
2. **Next Release**: Internal API marked with deprecation warnings
3. **Future**: Internal API made private to crate

## Support

For questions about migration:
- Review the unified order types in `unified_order.rs`
- Check example implementations in the SDK
- Contact the development team for complex migrations