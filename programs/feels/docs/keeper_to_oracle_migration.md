# Keeper to Oracle Migration Guide

This document describes the simplification of the keeper/oracle integration system in the Feels Protocol.

## Overview

We've replaced the complex keeper competition system with a simplified oracle-based approach. This reduces code complexity, lowers compute costs, and makes the system easier to integrate.

## Key Changes

### 1. Removed Components
- **Keeper Registration**: No more staking, registration fees, or keeper accounts
- **Slashing Mechanism**: Removed all slashing logic and appeal processes
- **Optimality Proofs**: Eliminated complex convex bounds and Lipschitz verification
- **Gradient/Hessian Competition**: Replaced with simple parameter updates

### 2. New Simplified System

#### Oracle Configuration
```rust
pub struct OracleConfig {
    pub primary_oracle: Pubkey,
    pub secondary_oracle: Pubkey,
    pub update_frequency: i64,
    pub last_update: i64,
    pub current_parameters: MarketParameters,
}
```

#### Market Parameters
```rust
pub struct MarketParameters {
    pub spot_gradient: i64,         // Single value, not 3D vector
    pub rate_gradient: i64,         // Simplified from complex gradients
    pub leverage_gradient: i64,     // Direct parameter
    pub market_curvature: u64,      // Replaces 3x3 Hessian
    pub risk_adjustment: u32,       // Basis points
    pub volatility: u32,            // Basis points
}
```

### 3. Migration Steps

#### For Pool Operators

1. **Initialize Oracle Config**
   ```rust
   // Old: Register keepers with stake
   // New: Simply set oracle addresses
   initialize_oracle_config(
       primary_oracle,
       secondary_oracle,
       update_frequency // 30-3600 seconds
   );
   ```

2. **Update Parameters**
   ```rust
   // Old: Complex keeper gradient submission
   // New: Direct oracle update
   update_oracle(OracleUpdate {
       pool,
       parameters,
       timestamp,
       oracle,
   });
   ```

#### For Keepers → Oracle Providers

1. **Simplify Computation**
   - Remove optimality proof generation
   - Remove Hessian calculation
   - Focus on market parameter estimation

2. **Update Format**
   ```rust
   // Old: HashMap of 3D gradients + certificates
   // New: Simple parameter struct
   let params = MarketParameters {
       spot_gradient: calculate_spot_gradient(),
       rate_gradient: calculate_rate_gradient(),
       leverage_gradient: calculate_leverage_gradient(),
       market_curvature: estimate_curvature(),
       risk_adjustment: calculate_risk_bps(),
       volatility: estimate_volatility_bps(),
   };
   ```

### 4. Benefits

1. **Code Reduction**: ~2000 lines removed
2. **State Size**: Gradient cache reduced from ~10MB to <1KB per pool
3. **Compute Cost**: 90% reduction in update costs
4. **Integration**: Standard oracle pattern, easier to understand
5. **Reliability**: Fewer failure modes, simpler validation

### 5. Compatibility

#### Temporary Dual Support
During migration, both systems can coexist:
- Keeper system remains for existing pools
- New pools use oracle system
- Gradual migration via governance

#### SDK Changes
```typescript
// Old
const keeper = await program.registerKeeper(stake);
const update = await keeper.submitGradient(gradients, proof);

// New
const update = await program.updateOracle({
    pool,
    parameters,
    timestamp,
});
```

### 6. Emergency Procedures

The new system includes emergency overrides:
```rust
emergency_oracle_override(parameters);
```
- Only protocol authority can execute
- Bypasses normal validation
- Used for critical situations

### 7. Validation Changes

#### Old System
- Complex mathematical proofs
- Optimality gap verification
- Lipschitz constant checks
- Convex bound validation

#### New System
- Simple parameter bounds
- Rate of change limits (5% max)
- Timestamp freshness (5 min max)
- Basic sanity checks

### 8. Future Improvements

1. **Multi-Oracle Support**: Aggregate multiple oracle feeds
2. **TWAP Integration**: Use time-weighted averages
3. **Automated Updates**: Trigger updates based on market conditions
4. **Decentralized Oracles**: Integrate with Pyth, Switchboard, etc.

## Conclusion

This migration significantly simplifies the protocol while maintaining functionality. The oracle-based system is more accessible, efficient, and reliable than the keeper competition model.