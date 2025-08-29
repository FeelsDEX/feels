# Swap Pool Specification - Phased Solana Implementation

## Overview

This specification details the phased implementation of the Feels Protocol on Solana, evolving from a basic concentrated liquidity AMM into a sophisticated 3D trading system with automated liquidity management and dynamic pricing.

**Core Design Principle**: All pools in the Feels Protocol use **FeelsSOL** as the common base pair. Every tradeable token must be paired with FeelsSOL, creating a hub-and-spoke liquidity model where FeelsSOL serves as the universal routing token.

## Implementation Roadmap

### Phase 1: Foundation - Concentrated Liquidity AMM
- Uniswap V3-style concentrated liquidity with single dimension (price)
- FeelsSOL as universal base pair for all pools
- Zero-copy architecture for optimal performance
- Simple oracle system with basic TWAP functionality
- Circuit breaker for emergency control
- Event emission for future hook integration

### Phase 2: Intelligence - Automated Management
- PositionVault activation for automated liquidity management
- Continuous leverage system with dynamic ceilings
- Enhanced oracle with volatility tracking
- Dynamic fee adjustments based on market conditions
- Atomic backrunning via Valence hooks

### Phase 3: Full Model - 3D Trading System
- Three-dimensional liquidity: rate × duration × leverage
- Weighted geometric mean invariant
- Term-based lending and structured products
- Complete yield curve modeling

---

# Phase 1: Concentrated Liquidity AMM

Phase 1 delivers a fully functional AMM similar to Uniswap V3, with all tokens paired against FeelsSOL. The architecture is designed for future upgrades without liquidity migration.

## Core Concepts

### Liquidity Concentration
Liquidity providers can concentrate their capital within specific price ranges, improving capital efficiency compared to traditional xy=k AMMs.

### FeelsSOL Base Pair
Every token trades against FeelsSOL, which wraps liquid staking tokens (e.g., JitoSOL) to provide automatic yield generation.

### Price Impact
- Small trades: Minimal impact (~0.01%)
- Large trades: Price impact increases with size relative to liquidity in range

## Account Architecture

All core accounts use Anchor's `zero_copy` deserialization for optimal performance on Solana.

### Pool Account

The main state account for each trading pair:

```rust
#[account(zero_copy)]
pub struct Pool {
    // Version control
    pub version: u8,                    // Set to 1 for Phase 1
    
    // Token configuration
    pub token_a_mint: Pubkey,           // Any token mint
    pub token_b_mint: Pubkey,           // Always FeelsSOL mint
    pub token_a_vault: Pubkey,          // Token A vault PDA
    pub token_b_vault: Pubkey,          // FeelsSOL vault PDA
    
    // Fee configuration
    pub fee_rate: u16,                  // Fee tier in basis points (1, 5, 30, 100)
    pub protocol_fee_rate: u16,         // Protocol's share of fees
    
    // Price and liquidity state
    pub current_tick: i32,              // Current price tick
    pub current_sqrt_price: u128,       // Square root of price (Q64.96)
    pub liquidity: u128,                // Total active liquidity
    
    // Tick bitmap for efficient searching
    pub tick_array_bitmap: [u64; 16],   // 1024-bit bitmap
    pub tick_spacing: i16,              // Minimum tick spacing
    
    // Fee tracking
    pub fee_growth_global_0: u256,      // Cumulative fees (token 0)
    pub fee_growth_global_1: u256,      // Cumulative fees (token 1)
    pub protocol_fees_0: u64,           // Uncollected protocol fees
    pub protocol_fees_1: u64,           // Uncollected protocol fees
    
    // Pool metadata
    pub authority: Pubkey,              // Pool authority
    pub creation_timestamp: i64,        // Creation time
    pub last_update_slot: u64,          // Last update slot
    
    // Statistics
    pub total_volume_0: u128,           // Cumulative volume
    pub total_volume_1: u128,           // Cumulative volume
    
    // Future upgrade space
    pub _reserved: [u8; 512],           // Reserved for Phase 2+
}

impl Pool {
    pub const SIZE: usize = 8 +         // Discriminator
        1 +                             // version
        32 * 4 +                        // token configuration (128)
        2 + 2 +                         // fee rates (4)
        4 + 16 + 16 +                   // price and liquidity (36)
        128 + 2 +                       // tick bitmap (130)
        32 + 32 + 8 + 8 +               // fee tracking (80)
        32 + 8 + 8 +                    // metadata (48)
        16 + 16 +                       // statistics (32)
        512;                            // reserved
        // Total: 8 + 1 + 128 + 4 + 36 + 130 + 80 + 48 + 32 + 512 = 979
}
```

### FeelsSOL Token Wrapper

The protocol's wrapped SOL token that serves as the universal base pair:

```rust
#[account]
pub struct FeelsSOL {
    pub underlying_mint: Pubkey,        // JitoSOL or other LST
    pub feels_mint: Pubkey,             // FeelsSOL Token-2022 mint
    pub total_wrapped: u128,            // Total LST wrapped
    pub virtual_reserves: u128,         // Virtual balance for AMM
    pub yield_accumulator: u128,        // Accumulated staking yield
    pub last_update_slot: u64,          // Last yield update
    pub authority: Pubkey,              // Protocol authority
}
```

### Tick Array Account

Ticks are grouped into arrays for efficiency:

```rust
#[account(zero_copy)]
pub struct TickArray {
    pub pool: Pubkey,                   // Associated pool
    pub start_tick_index: i32,          // First tick in array
    pub ticks: [Tick; TICK_ARRAY_SIZE], // Array of ticks
    pub initialized_tick_count: u8,     // Number of initialized ticks
}

#[zero_copy]
#[derive(Default)]
pub struct Tick {
    pub liquidity_net: i128,            // Net liquidity change
    pub liquidity_gross: u128,          // Total liquidity
    pub fee_growth_outside_0: u256,     // Fee growth outside
    pub fee_growth_outside_1: u256,     // Fee growth outside
    pub initialized: bool,              // Whether initialized
}

pub const TICK_ARRAY_SIZE: usize = 60;
```

### Position NFT Metadata

Each liquidity position is represented by an NFT:

```rust
#[account]
pub struct PositionMetadata {
    // Position identification
    pub pool: Pubkey,
    pub position_mint: Pubkey,
    pub owner: Pubkey,
    
    // Range definition
    pub tick_lower: i32,
    pub tick_upper: i32,
    
    // Liquidity tracking
    pub liquidity: u128,
    
    // Fee tracking
    pub fee_growth_inside_last_0: u256,
    pub fee_growth_inside_last_1: u256,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
    
    // Future extensions
    pub _reserved: [u8; 64],
}
```

### Oracle Account (Phase 1 Simplified)

A simple observation buffer for TWAP calculations:

```rust
#[account]
pub struct ObservationState {
    pub pool: Pubkey,
    pub observations: [Observation; 100],   // Fixed size for Phase 1
    pub observation_index: u16,             // Current position in ring buffer
    pub cardinality: u16,                   // Active observations
    pub last_update_timestamp: i64,
}

#[derive(Default, Clone, Copy)]
pub struct Observation {
    pub timestamp: i64,
    pub sqrt_price_x96: u128,
    pub cumulative_tick: i128,
    pub initialized: bool,
}
```

### Fee Configuration

A unified account for all fee-related configuration:

```rust
#[account]
pub struct FeeConfig {
    pub pool: Pubkey,
    pub base_fee: u16,                  // Base fee rate
    pub protocol_share: u16,            // Protocol's share
    pub lp_share: u16,                  // LP's share
    
    // Dynamic fee parameters (Phase 2 prep)
    pub volume_24h: u128,               // 24h volume tracking
    pub last_update_slot: u64,          // Last update
    pub _reserved: [u8; 128],           // Future dynamic params
}
```

## Core Operations

### Pool Initialization

Creates a new pool with token A paired against FeelsSOL:

```rust
pub fn initialize_pool(
    ctx: Context<InitializePool>,
    fee_rate: u16,
    initial_sqrt_price: u128,
) -> Result<()>
```

**Validation:**
- `token_b_mint` must be the official FeelsSOL mint
- Fee rate must be from allowed set (1, 5, 30, 100 bps)
- No duplicate pools for the same token pair

### Liquidity Management

#### Add Liquidity

```rust
pub fn add_liquidity(
    ctx: Context<AddLiquidity>,
    liquidity_amount: u128,
    amount_0_max: u64,
    amount_1_max: u64,
) -> Result<(u64, u64)>
```

**Process:**
1. Calculate required token amounts
2. Initialize ticks if needed
3. Update tick liquidity values
4. Transfer tokens to pool vaults
5. Update position NFT metadata
6. Update global liquidity if in range

#### Remove Liquidity

```rust
pub fn remove_liquidity(
    ctx: Context<RemoveLiquidity>,
    liquidity_amount: u128,
    amount_0_min: u64,
    amount_1_min: u64,
) -> Result<(u64, u64)>
```

### Trading

#### Swap

```rust
pub fn swap(
    ctx: Context<Swap>,
    amount_in: u64,
    amount_out_minimum: u64,
    sqrt_price_limit: u128,
    is_token_a_to_b: bool,
) -> Result<u64>
```

**Process:**
1. Initialize swap state
2. Iterate through ticks:
   - Calculate swap amount within tick
   - Update sqrt price
   - Cross tick if necessary
   - Accumulate fees
3. Apply fees and update global state
4. Transfer tokens
5. Update oracle observation
6. Emit event

### Cross-Token Routing

Since all pools use FeelsSOL as base pair, routing is simple:

```rust
pub enum SwapRoute {
    Direct(Pubkey),           // One pool (token involves FeelsSOL)
    TwoHop(Pubkey, Pubkey),   // Two pools (neither token is FeelsSOL)
}

impl SwapRoute {
    pub fn find(from: Pubkey, to: Pubkey) -> Self {
        if from == FEELS_SOL_MINT || to == FEELS_SOL_MINT {
            SwapRoute::Direct(get_pool(from, to))
        } else {
            SwapRoute::TwoHop(
                get_pool(from, FEELS_SOL_MINT),
                get_pool(FEELS_SOL_MINT, to)
            )
        }
    }
}
```

## Mathematical Model

### Price Representation

Prices use square root representation for efficiency:
- `sqrt_price = sqrt(amount_1 / amount_0) * 2^96`
- `tick = log₁.₀₀₀₁(price)`

### Liquidity Calculation

Virtual liquidity for concentrated positions:
```
L = sqrt(x * y)

amount_0 = L * (sqrt_price_upper - sqrt_price) / (sqrt_price_upper * sqrt_price)
amount_1 = L * (sqrt_price - sqrt_price_lower)
```

### Fee Accumulation

Fees accumulate globally and per-position:
```
fee_growth_global += (fee_amount * 2^128) / liquidity
tokens_owed = liquidity * (fee_growth_inside - fee_growth_inside_last) / 2^128
```

## Gas Optimizations

### Zero-Copy Deserialization
All large accounts use `zero_copy` to avoid heap allocations.

### Tick Bitmap
O(1) tick searching using bitmap instead of linked lists.

### Transient Tick Updates

To reduce compute units when crossing multiple ticks:

```rust
#[account]
pub struct TransientTickUpdates {
    pub pool: Pubkey,
    pub slot: u64,
    pub updates: Vec<TickUpdate>,
    pub finalized: bool,
}

pub struct TickUpdate {
    pub tick_array_pubkey: Pubkey,
    pub tick_index: i32,
    pub liquidity_net_delta: i128,
    pub fee_growth_outside_0: u256,
    pub fee_growth_outside_1: u256,
    pub slot: u64,              // When update was generated
    pub priority: u8,           // Urgency hint for keepers
}
```

Swaps can defer tick updates to a separate transaction, reducing compute usage by ~50%.

## Security Features

### Circuit Breaker

Emergency controls stored in pool's reserved space:

```rust
pub struct CircuitBreakerStatus {
    pub swaps_paused: bool,
    pub deposits_paused: bool,
    pub withdrawals_paused: bool,
    pub fee_collection_paused: bool,
    pub guardian: Option<Pubkey>,
    pub pause_expiry: Option<i64>,
}
```

### Validation Requirements

1. **Price Bounds**: Sqrt prices must not overflow
2. **Tick Bounds**: Ticks within MIN_TICK and MAX_TICK
3. **Liquidity Limits**: Total liquidity < u128::MAX
4. **Fee Limits**: Only allowed fee tiers
5. **Token Ordering**: Consistent token ordering

## PositionVault Role in Phase 1

In Phase 1, the PositionVault account exists but has minimal functionality:

- **Structure**: Basic account created but not actively managing liquidity
- **Fee Collection**: Can collect protocol fees to designated treasury
- **Preparation**: Reserves space for Phase 2 automation features
- **No Active Management**: Does not rebalance or provide JIT liquidity

This allows protocols to deploy the infrastructure while keeping Phase 1 simple.

## Event System

Unified event model for all operations:

```rust
pub trait EventBase {
    fn pool(&self) -> Pubkey;
    fn timestamp(&self) -> i64;
    fn actor(&self) -> Pubkey;
}

#[event]
pub struct SwapEvent {
    #[index]
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_price_after: u128,
    pub tick_after: i32,
    pub fee: u64,
}

#[event]
pub struct LiquidityEvent {
    #[index]
    pub pool: Pubkey,
    pub position: Pubkey,
    pub liquidity_delta: i128,
    pub amount_0: u64,
    pub amount_1: u64,
}
```

## Phase 1 → Phase 2 Upgrade Path

1. **Deploy New Program**: Updated program with Phase 2 features
2. **Upgrade Pools**: Call `upgrade_pool` instruction per pool
3. **Activate Features**: Reserved space becomes structured data
4. **No Migration**: Existing liquidity remains untouched

The 512-byte reserved space will accommodate:
- Leverage parameters (64 bytes)
- Enhanced oracle reference (32 bytes)
- PositionVault reference (32 bytes)
- Dynamic fee parameters (64 bytes)
- Additional features (320 bytes)

---

# Phase 2: Automated Management & Continuous Leverage

Phase 2 activates new features while maintaining full backward compatibility.

## Continuous Leverage System

### Overview

Phase 2 introduces a continuous leverage scale from 1.0x to pool-specific maximums.

### Risk Profile Model

All risk calculations unified under a single concept:

```rust
pub struct RiskProfile {
    pub leverage: u64,                  // Input leverage (6 decimals)
    pub protection_factor: u64,         // Derived from curve
    pub fee_multiplier: u64,            // Sqrt scaling
    pub max_loss_percentage: u64,       // Clear risk metric
    pub required_margin_ratio: u64,     // Margin requirement
}

impl RiskProfile {
    pub fn from_leverage(leverage: u64, pool: &Pool) -> Self {
        let protection = calculate_protection(leverage, &pool.leverage_params);
        let fee_mult = sqrt_u64(leverage * LEVERAGE_SCALE) / 1_000;
        let max_loss = PROTECTION_SCALE - protection;
        let margin = LEVERAGE_SCALE / leverage + calculate_buffer(leverage);
        
        Self {
            leverage,
            protection_factor: protection,
            fee_multiplier: fee_mult,
            max_loss_percentage: max_loss,
            required_margin_ratio: margin,
        }
    }
}
```

### Leverage and Collateral Model

**Key Insight**: Your liquidity position IS your collateral.

1. **No Separate Margin Account**: Deposited tokens serve as margin
2. **Natural Risk Management**: Protection curve handles losses gradually
3. **No Forced Liquidations**: Positions redenominate during stress

Example:
- Deposit: 100 FeelsSOL of liquidity
- 3x leverage: Control 300 FeelsSOL worth of liquidity
- Protection: 45% (at 3x leverage)
- Max loss: 55% of leveraged value during redenomination

### Protection Curves

Three configurable curve types:

```rust
pub enum ProtectionCurve {
    Linear,       // protection = 1 - (leverage - 1) / (max - 1)
    Exponential { // protection = e^(-k * (leverage - 1))
        decay_rate: u64,
    },
    Piecewise {   // Custom breakpoints
        points: Vec<(u64, u64)>,
    },
}
```

### Dynamic Leverage Ceilings

Each pool calculates its own maximum leverage based on:

```rust
pub struct LeverageCeilingFactors {
    pub base_ceiling: u64,              // Normal conditions
    pub liquidity_depth: u64,           // Deep liquidity → higher ceiling
    pub volatility: u64,                // High volatility → lower ceiling
    pub concentration: u64,             // Concentrated liquidity → higher ceiling
}

pub fn calculate_pool_leverage_ceiling(
    pool: &Pool,
    factors: &LeverageCeilingFactors,
) -> u64 {
    let depth_mult = match factors.liquidity_depth {
        d if d > 80 => 120,  // +20% for deep liquidity
        d if d > 60 => 100,  // Standard
        d if d > 40 => 80,   // -20% for medium
        _ => 60,             // -40% for thin
    };
    
    let vol_mult = if factors.volatility > 500 {
        60  // -40% for high volatility
    } else if factors.volatility > 300 {
        80  // -20% for medium volatility
    } else {
        100 // Standard for low volatility
    };
    
    (factors.base_ceiling * depth_mult * vol_mult) / 10_000
}
```

## Enhanced Account Structures

### Pool V2 (Using Reserved Space)

```rust
// Phase 2 additions in reserved space
pub struct PoolV2Extensions {
    pub leverage_params: LeverageParameters,
    pub position_vault: Option<Pubkey>,
    pub enhanced_oracle: Pubkey,
    pub dynamic_fee_config: DynamicFeeConfig,
    pub volume_tracker: VolumeTracker,
}

pub struct LeverageParameters {
    pub max_leverage: u64,
    pub current_ceiling: u64,
    pub protection_curve: ProtectionCurve,
    pub last_ceiling_update: u64,
}
```

### Enhanced Oracle

Phase 2 expands the observation buffer and adds volatility tracking:

```rust
#[account]
pub struct EnhancedOracle {
    pub pool: Pubkey,
    pub observations: [Observation; 1000],  // Expanded from 100
    pub observation_index: u16,
    pub cardinality: u16,
    
    // New volatility tracking
    pub volatility_basis_points: u64,
    pub volatility_window: u32,
    pub last_volatility_update: i64,
}
```

### PositionVault Activation

The dormant PositionVault from Phase 1 becomes active:

```rust
#[account]
pub struct PositionVault {
    pub pool: Pubkey,
    pub authority: Pubkey,
    
    // Dual liquidity sources
    pub protocol_owned_liquidity: u128,
    pub user_deposits: u128,
    pub total_shares: u128,
    
    // Active management
    pub active_positions: Vec<ManagedPosition>,
    pub rebalance_config: RebalanceConfig,
    pub last_rebalance_slot: u64,
    
    // Performance metrics
    pub total_fees_earned: u128,
    pub performance_metrics: VaultMetrics,
}
```

## Dynamic Fee System

Fees adjust based on market conditions:

```rust
pub struct DynamicFeeConfig {
    pub base_fee: u16,
    pub min_fee: u16,
    pub max_fee: u16,
    pub volatility_coefficient: u64,
    pub volume_discount_threshold: u128,
}

pub fn calculate_dynamic_fee(
    base_fee: u16,
    volatility: u64,
    volume_24h: u128,
    config: &DynamicFeeConfig,
) -> u16 {
    let vol_adjustment = if volatility > 500 {
        150  // 1.5x fee in high volatility
    } else {
        100
    };
    
    let volume_discount = if volume_24h > config.volume_discount_threshold {
        90  // 10% discount for high volume
    } else {
        100
    };
    
    let dynamic_fee = (base_fee * vol_adjustment * volume_discount) / 10_000;
    dynamic_fee.clamp(config.min_fee, config.max_fee)
}
```

## Redenomination During Stress

When market conditions trigger redenomination:

```rust
pub fn apply_redenomination(
    pool: &mut Pool,
    market_loss: u128,
) -> Result<()> {
    // Group positions by leverage
    let mut leverage_groups = BTreeMap::new();
    for (leverage, liquidity) in pool.liquidity_by_leverage.iter() {
        let risk_profile = RiskProfile::from_leverage(*leverage, pool);
        leverage_groups.insert(*leverage, (*liquidity, risk_profile));
    }
    
    // Calculate loss distribution
    let mut remaining_loss = market_loss;
    for (leverage, (liquidity, profile)) in leverage_groups.iter().rev() {
        // Higher leverage absorbs losses first
        let unprotected_value = liquidity * profile.max_loss_percentage / SCALE;
        let position_loss = remaining_loss.min(unprotected_value);
        
        // Apply loss to position
        apply_position_loss(*leverage, position_loss);
        remaining_loss -= position_loss;
        
        if remaining_loss == 0 { break; }
    }
    
    Ok(())
}
```

---

# Phase 3: Full 3D Trading System

Phase 3 implements the complete vision with three-dimensional liquidity.

## 3D Liquidity Model

### Dimensions

1. **Rate**: Interest rate or price (existing)
2. **Duration**: Flash, Swap, Monthly, etc.
3. **Leverage**: Continuous scale with protection

### Weighted Geometric Mean

The invariant becomes:
```
K = R^wr × D^wd × L^wl
```

Where positions can specify values across all three dimensions.

### Tick Space Design

```rust
pub struct Tick3D {
    pub rate_tick: i32,
    pub duration_tick: i16,
    pub leverage_tick: i16,
}

// Efficient encoding for the three dimensions
impl Tick3D {
    pub fn encode(&self) -> i32 {
        // Rate uses primary bits (highest precision needed)
        let rate_masked = self.rate_tick & ((1 << RATE_BITS) - 1);
        
        // Duration uses 6 bits (supports Duration enum)
        let duration_shifted = (self.duration_tick as i32) << RATE_BITS;
        
        // Leverage uses 6 bits (64 discrete levels of continuous leverage)
        let leverage_shifted = (self.leverage_tick as i32) << (RATE_BITS + DURATION_BITS);
        
        rate_masked | duration_shifted | leverage_shifted
    }
    
    pub fn decode(encoded: i32) -> Self {
        let rate_tick = encoded & ((1 << RATE_BITS) - 1);
        let duration_tick = ((encoded >> RATE_BITS) & ((1 << DURATION_BITS) - 1)) as i16;
        let leverage_tick = ((encoded >> (RATE_BITS + DURATION_BITS)) & ((1 << LEVERAGE_BITS) - 1)) as i16;
        
        Self { rate_tick, duration_tick, leverage_tick }
    }
}
```

## Term-Based Products

### Duration Types

The protocol supports a fixed set of duration types:

```rust
#[repr(u8)]
pub enum Duration {
    Flash = 0,      // 1 block
    Swap = 1,       // Immediate (spot)
    Weekly = 2,     // 7 days
    Monthly = 3,    // 28 days
    Quarterly = 4,  // 90 days
    Annual = 5,     // 365 days
}

impl Duration {
    pub const COUNT: usize = 6;
    
    pub fn to_blocks(&self) -> u64 {
        match self {
            Duration::Flash => 1,
            Duration::Swap => 0,
            Duration::Weekly => 7 * 24 * 60 * 5,     // 5 blocks/min
            Duration::Monthly => 28 * 24 * 60 * 5,
            Duration::Quarterly => 90 * 24 * 60 * 5,
            Duration::Annual => 365 * 24 * 60 * 5,
        }
    }
}
```

### Yield Curves

The protocol automatically generates yield curves from the 3D liquidity distribution, with no need for external configuration.

## 3D System Architecture

### Fixed Three-Dimensional Design

The protocol is specifically optimized for exactly three dimensions:

```rust
pub struct Dimensions {
    pub rate: DimensionConfig,      // Price/interest rate
    pub duration: DimensionConfig,  // Time commitment
    pub leverage: DimensionConfig,  // Risk level
}

pub struct DimensionConfig {
    pub weight: u64,                // Dimension weight in invariant
    pub tick_range: Range<i32>,     // Allocated tick space
    pub enabled: bool,              // Whether dimension is active
}
```

### Tick Space Allocation

The system uses efficient bit packing for the three dimensions:

```rust
// Total 32-bit tick space allocation:
// - Rate: 20 bits (primary dimension)
// - Duration: 6 bits (8 duration types)
// - Leverage: 6 bits (64 leverage levels)
pub const RATE_BITS: u8 = 20;
pub const DURATION_BITS: u8 = 6;
pub const LEVERAGE_BITS: u8 = 6;
```

### Why Exactly Three Dimensions?

1. **Optimal Complexity**: Three dimensions provide sophisticated pricing without overwhelming users
2. **Efficient Computation**: 3D geometric mean calculations are fast enough for on-chain execution
3. **Natural Financial Model**: Rate, time, and risk are the fundamental axes of finance
4. **Tick Space Efficiency**: 32-bit integers perfectly accommodate three dimensions

---

## Summary

This specification provides a clear progression from simple AMM to sophisticated 3D trading:

- **Phase 1**: Solid Uniswap V3 foundation with FeelsSOL pairs
- **Phase 2**: Continuous leverage and automated management
- **Phase 3**: Complete 3D model with rate × duration × leverage

The protocol is specifically optimized for these three dimensions:
- **Rate**: The price dimension (existing concentrated liquidity)
- **Duration**: Time commitments from flash loans to annual terms
- **Leverage**: Continuous risk scaling with protection curves

Each phase maintains backward compatibility through careful use of reserved space. The architecture prioritizes simplicity in Phase 1 while building toward the complete three-dimensional vision - no more, no less.