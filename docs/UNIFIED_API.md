# Unified API Documentation

The Feels Protocol has been refactored to provide a simplified, unified API that reduces complexity while maintaining full functionality.

## Overview

The protocol now provides three main unified instruction interfaces:

1. **`configure_pool`** - All pool configuration operations
2. **`order_unified`** - All trading operations (swaps, liquidity, limits)
3. **`order_compute_unified`** - Pre-computation for complex orders

## Pool Configuration

### Before (Multiple Instructions)
```rust
// Multiple separate instructions for configuration
enable_leverage(ctx, EnableLeverageParams { ... });
update_leverage_ceiling(ctx, new_ceiling, ...);
update_dynamic_fees(ctx, UpdateDynamicFeesParams { ... });
register_hook(ctx, RegisterHookParams { ... });
```

### After (Single Unified Instruction)
```rust
// Single instruction with enum variants
configure_pool(ctx, PoolConfigParams::Leverage(LeverageConfig {
    operation: LeverageOperation::Enable,
    max_leverage: Some(10_000_000), // 10x
    current_ceiling: Some(5_000_000), // 5x
    protection_curve: Some(ProtectionCurveConfig { ... }),
}));

// Batch multiple configurations
configure_pool(ctx, PoolConfigParams::Batch(vec![
    PoolConfigParams::Leverage(config),
    PoolConfigParams::DynamicFees(fees),
    PoolConfigParams::Hook(hook_config),
]));
```

## Trading Operations

### Before (Complex Parameters)
```rust
// Complex nested parameter structures
order(ctx, OrderParams {
    amount: 1000,
    rate_params: RateParams::TargetRate {
        sqrt_rate_limit: 1234567890,
        is_token_a_to_b: true,
    },
    duration: Duration::Swap,
    leverage: 1_000_000,
    order_type: OrderType::Immediate,
    limit_value: 900,
});
```

### After (Simplified Unified API)
```rust
// Simple swap
order_unified(ctx, UnifiedOrderParams {
    amount: 1000,
    config: OrderConfig::Swap {
        is_token_a_to_b: true,
        min_amount_out: 900,
        sqrt_rate_limit: None, // Optional
    },
    advanced: None, // Use defaults
});

// Add liquidity
order_unified(ctx, UnifiedOrderParams {
    amount: 10000,
    config: OrderConfig::AddLiquidity {
        tick_lower: -887220,
        tick_upper: 887220,
        token_amounts: None, // Auto-calculate
    },
    advanced: None,
});

// Advanced order with leverage
order_unified(ctx, UnifiedOrderParams {
    amount: 1000,
    config: OrderConfig::Swap { ... },
    advanced: Some(AdvancedOrderParams {
        duration: Duration::Weekly,
        leverage: 5_000_000, // 5x
        mev_protection: Some(MevProtection { ... }),
        hook_data: None,
    }),
});
```

## Configuration Enum Reference

### PoolConfigParams
- `Leverage(LeverageConfig)` - Configure leverage parameters
- `DynamicFees(DynamicFeeConfig)` - Update fee parameters
- `Authority(AuthorityConfig)` - Change pool authority
- `Hook(HookConfig)` - Register/unregister hooks
- `Oracle(OracleConfig)` - Configure price oracle
- `Redenomination(RedenominationConfig)` - Set redenomination parameters
- `Batch(Vec<PoolConfigParams>)` - Apply multiple configurations

### OrderConfig
- `Swap` - Spot or leveraged swaps
- `AddLiquidity` - Provide liquidity to pools
- `RemoveLiquidity` - Remove liquidity from positions
- `LimitOrder` - Create limit orders
- `FlashLoan` - Borrow tokens for atomic arbitrage

## Benefits

1. **Reduced API Surface**: Fewer instructions to learn and maintain
2. **Type Safety**: Enum-based parameters prevent invalid combinations
3. **Flexibility**: Batch operations and optional advanced parameters
4. **Forward Compatibility**: Easy to add new configuration types
5. **Simplified Client Code**: Less boilerplate, clearer intent

## Migration Guide

### For Developers

1. Replace multiple configuration calls with `configure_pool`
2. Use `order_unified` instead of constructing complex `OrderParams`
3. Leverage batch operations to reduce transaction count
4. Use the simplified parameter builders in the SDK

### Migration Guide

All functionality has been migrated to the unified API. The old individual instruction functions have been removed in favor of the cleaner unified interface.

## Examples

See the `/sdk/examples/unified_api_examples.rs` for complete examples of:
- Basic swaps
- Liquidity provision
- Leveraged trading
- Pool configuration
- Batch operations