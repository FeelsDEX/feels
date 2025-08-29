# Tick Array Management Strategy

## Overview

This document defines the strategy for managing TickArray accounts in the Feels Protocol concentrated liquidity AMM. Each TickArray stores 32 consecutive ticks (for memory alignment) and serves as the fundamental unit for organizing the price space.

## 1. PDA Derivation Strategy

### TickArray PDA Seeds
```rust
seeds = [
    b"tick_array",
    pool_key.as_ref(),
    start_tick_index.to_le_bytes().as_ref()
]
```

### Key Properties
- Start tick index MUST be divisible by (TICK_ARRAY_SIZE * tick_spacing) where TICK_ARRAY_SIZE = 32
- Each pool can have up to ~27,727 tick arrays (covering implementation range -443,636 to +443,636)
- PDAs are deterministic and can be pre-calculated off-chain
- Tick arrays contain 32 consecutive ticks each for efficient memory usage and alignment
- Total size: 3,373 bytes per tick array (8 + 32 + 4 + 3328 + 1)

## 2. Tick Array Structure

### 2.1 Tick Data Structure
```rust
#[zero_copy]
#[derive(Default)]
#[repr(C, packed)]
pub struct Tick {
    // Liquidity tracking
    pub liquidity_net: i128,            // Net liquidity change when crossed
    pub liquidity_gross: u128,          // Total liquidity referencing this tick
    
    // Fee tracking (outside the tick)
    pub fee_growth_outside_0: [u64; 4], // Fee growth outside (token 0) - u256 as 4 u64s
    pub fee_growth_outside_1: [u64; 4], // Fee growth outside (token 1) - u256 as 4 u64s
    
    // Tick metadata
    pub initialized: u8,                // Whether this tick is initialized (0 = false, 1 = true)
    pub _padding: [u8; 7],              // Explicit padding for 8-byte alignment
}
// Total: 104 bytes per tick
```

### 2.2 TickArray Account
```rust
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TickArray {
    pub pool: Pubkey,                   // Associated pool (32 bytes)
    pub start_tick_index: i32,          // First tick in this array (4 bytes)
    pub ticks: [Tick; TICK_ARRAY_SIZE], // Array of tick data (32 ticks * 104 = 3,328 bytes)
    pub initialized_tick_count: u8,     // Number of initialized ticks (1 byte)
}
// Total: 8 (discriminator) + 32 + 4 + 3,328 + 1 = 3,373 bytes
```

## 3. Tick Array Lifecycle

### 3.1 Creation (Lazy Initialization)
Tick arrays are created on-demand when:
- A user adds liquidity to a previously empty price range
- The first position references ticks in that array

This is handled automatically during the `add_liquidity` instruction:
- Arrays are only created when actually needed
- The pool's tick_array_bitmap is updated to track initialized arrays
- Creation is atomic with the liquidity addition
- Uses the `initialize_tick_array` helper function in the liquidity logic

### 3.2 Cleanup Strategy (Rent Reclamation)
The protocol includes two cleanup mechanisms:

#### Incentivized Cleanup (Primary Implementation)
```rust
// Via tick_cleanup::handler
pub fn handler(ctx: Context<CleanupTickArray>) -> Result<()> {
    // Validates tick array belongs to pool
    // Ensures array is completely empty (initialized_tick_count == 0)
    // Updates the pool's tick_array_bitmap
    // Calculates rent distribution:
    //   - Protocol keeps 20% as treasury fee
    //   - Cleaner gets 80% as incentive
    // Uses safe lamport transfers
    // Emits TickArrayCleanedEvent
}
```

#### Basic Cleanup (Simplified Version)
```rust
// Via tick_cleanup::handler_empty  
pub fn handler_empty(ctx: Context<CleanupEmptyTickArray>) -> Result<()> {
    // Validates array is empty
    // Simple cleanup without rent distribution
    // Emits TickArrayCleanedUpEvent
}
```

## 4. Tick Array Router System

### 4.1 Router Structure
The TickArrayRouter enables efficient tick array access without remaining_accounts:

```rust
#[account]
pub struct TickArrayRouter {
    pub pool: Pubkey,                                 // The pool this router is associated with
    pub tick_arrays: [Pubkey; MAX_ROUTER_ARRAYS],     // Pre-registered tick array accounts (up to 8)
    pub start_indices: [i32; MAX_ROUTER_ARRAYS],      // Start tick index for each array (i32::MIN = unused)
    pub active_bitmap: u8,                            // Bitmap indicating which slots are active
    pub last_update_slot: u64,                        // Last update slot for cache invalidation
    pub authority: Pubkey,                            // Authority who can update the router
    pub _reserved: [u8; 64],                          // Reserved for future use
}
```

### 4.2 Router Usage
The router is designed for:
- Pre-registering commonly used tick arrays for gas optimization
- Enabling Valence-compatible operations without remaining_accounts
- Caching tick array addresses for efficient lookups
- Supporting up to 8 tick arrays (MAX_ROUTER_ARRAYS = 8)

## 5. Efficient Traversal Strategy

### 5.1 Bitmap-Based Search
The pool maintains a 1024-bit bitmap (16 u64s) for tracking initialized tick arrays:

```rust
pub tick_array_bitmap: [u64; 16],  // Each bit represents one tick array
```

Key operations:
- Check if array exists: `(bitmap[word_index] & (1u64 << bit_index)) != 0`
- Mark array initialized: `bitmap[word_index] |= 1u64 << bit_index`
- Mark array uninitialized: `bitmap[word_index] &= !(1u64 << bit_index)`

### 5.2 Array Navigation
Navigation helpers are implemented in the logic layer:
- `contains_tick()`: Check if a tick is within this array's range
- `get_tick()`: Retrieve a specific tick from the array
- Bitmap operations for efficient array discovery
- Start index calculations based on tick spacing

## 6. Implementation Status

### **Fully Implemented**
- **Core Structure**: 
  - 32-tick arrays with zero-copy optimization
  - Packed struct layout for minimal storage (3373 bytes)
  - Full tick data including liquidity and fee tracking
  
- **PDA System**:
  - Deterministic addressing using pool + start_tick
  - Automatic validation in instructions
  
- **Bitmap Tracking**: 
  - 1024-bit bitmap in Pool state
  - Efficient O(1) lookup for array existence
  - Automatic sync during creation/cleanup
  
- **TickArrayRouter**: 
  - Pre-register up to 8 arrays for gas optimization
  - Configurable update strategies
  - Authority-controlled updates
  
- **Lifecycle Management**:
  - Lazy initialization via TickArrayManager
  - Two cleanup mechanisms (basic and incentivized)
  - Rent reclamation with configurable fee split
  
- **Navigation**:
  - Cross-array boundary detection
  - Directional traversal helpers
  - Bitmap-guided search utilities

### **Additional Features**
- **Transient Updates**: TransientTickUpdates structure exists for gas optimization
- **Pool Extensions**: Phase 2 features are enabled by default in all pools

## 7. Key Implementation Details

1. **Array Size**: 32 ticks per array for optimal memory alignment
2. **Cleanup Mechanism**: Three implementations - incentivized (80/20 split), basic, and V2 (configurable)
3. **Router System**: TickArrayRouter supports up to 8 pre-registered arrays
4. **Bitmap Size**: 1024-bit bitmap (16 u64s) in Pool state
5. **Fee Tracking**: Uses token 0/1 naming convention for consistency
6. **Size Calculation**: Each tick is 104 bytes, total array size is 3,373 bytes

## 8. Security Considerations

1. **Array Initialization**: Only created when liquidity is actually added
2. **Cleanup Authorization**: Validates array is truly empty before allowing cleanup
3. **Bitmap Consistency**: Always updated atomically with array operations
4. **PDA Validation**: All array addresses verified against expected derivation
5. **Bounds Checking**: Tick indices validated against valid ranges
6. **Router Authority**: Only authorized addresses can update router configuration

## 9. Gas Optimization Features

1. **Zero-Copy Design**: Direct memory access without deserialization
2. **TickArrayRouter**: Pre-load commonly used arrays
3. **Bitmap Search**: O(1) existence checks, efficient traversal
4. **Lazy Initialization**: Arrays only created when needed
5. **Packed Structs**: Minimal memory footprint with explicit padding

## 10. Events

The protocol emits events for tick array operations:

```rust
#[event]
pub struct TickArrayCleanedEvent {
    pub pool: Pubkey,
    pub tick_array: Pubkey,
    pub start_tick: i32,
    pub initialized_count: u8,
    pub timestamp: i64,
}

#[event]
pub struct TickArrayCleanedUpEvent {
    pub pool: Pubkey,
    pub tick_array: Pubkey,
    pub start_tick: i32,
    pub ticks_cleaned: u16,
    pub gas_refund_estimate: u64,
    pub cleaner: Pubkey,
    pub timestamp: i64,
}
```

This implementation provides efficient, secure, and scalable tick array management while maintaining compatibility with concentrated liquidity AMM requirements and optimizing for Solana's architecture.