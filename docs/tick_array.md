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
- Start tick index MUST be divisible by (32 * tick_spacing)
- Each pool can have up to ~27,727 tick arrays (covering implementation range -443636 to +443636)
- PDAs are deterministic and can be pre-calculated off-chain
- Tick arrays contain 32 consecutive ticks each for efficient memory usage and alignment
- Total size: 3373 bytes per tick array

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
    pub initialized: u8,                // Whether this tick is initialized (0/1)
    pub _padding: [u8; 7],              // Explicit padding for 8-byte alignment
}
```

### 2.2 TickArray Account
```rust
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TickArray {
    pub pool: Pubkey,                   // Associated pool
    pub start_tick_index: i32,          // First tick in this array
    pub ticks: [Tick; TICK_ARRAY_SIZE], // Array of tick data (32 ticks)
    pub initialized_tick_count: u8,     // Number of initialized ticks
}
```

## 3. Tick Array Lifecycle

### 3.1 Creation (Lazy Initialization)
Tick arrays are created on-demand when:
- A user adds liquidity to a previously empty price range
- The first position references ticks in that array

This is handled automatically by the `TickArrayManager` during the `add_liquidity` instruction:
- Arrays are only created when actually needed
- The pool's tick_array_bitmap is updated to track initialized arrays
- Creation is atomic with the liquidity addition

### 3.2 Cleanup Strategy (Rent Reclamation)
The protocol includes two cleanup mechanisms:

#### Basic Cleanup (Implemented)
```rust
// Via CleanupTickArray instruction
pub fn cleanup_tick_array(ctx: Context<CleanupTickArray>) -> Result<()> {
    // Validates tick array belongs to pool
    // Ensures array is completely empty (initialized_tick_count == 0)
    // Updates the pool's tick_array_bitmap
    // Closes the account and returns rent
}
```

#### Incentivized Cleanup (Implemented)
```rust
// Via CleanupTickArray instruction (comprehensive version)
pub fn cleanup_tick_array(ctx: Context<CleanupTickArray>) -> Result<()> {
    // Validates array is empty
    // Calculates rent distribution:
    //   - Protocol keeps 20% as treasury fee
    //   - Cleaner gets 80% as incentive
    // Updates bitmap and closes account
    // Emits TickArrayCleanedEvent
}
```

## 4. Tick Array Router System

### 4.1 Router Structure
The TickArrayRouter enables efficient tick array access without remaining_accounts:

```rust
#[account]
pub struct TickArrayRouter {
    pub pool: Pubkey,
    pub tick_arrays: [Pubkey; MAX_ROUTER_ARRAYS],     // Up to 8 pre-registered arrays
    pub start_indices: [i32; MAX_ROUTER_ARRAYS],      // Start tick for each array
    pub active_bitmap: u8,                            // Which slots are active
    pub last_update_slot: u64,                        // Cache invalidation
    pub authority: Pubkey,                            // Update authority
    pub _reserved: [u8; 64],
}
```

### 4.2 Router Configuration
```rust
pub struct RouterConfig {
    pub arrays_around_current: u8,    // Number of arrays to pre-load (default: 3)
    pub update_frequency: u64,        // Update every N slots (default: 100)
    pub auto_update_enabled: bool,    // Auto-update on price moves
    pub price_move_threshold: i32,    // Tick movement threshold (default: 100)
}
```

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
```rust
impl TickArray {
    // Get next array in swap direction
    pub fn next_array_start_index(&self, zero_for_one: bool) -> i32 {
        if zero_for_one {
            self.start_tick_index - TICK_ARRAY_SIZE as i32
        } else {
            self.start_tick_index + TICK_ARRAY_SIZE as i32
        }
    }
    
    // Check if swap needs to cross arrays
    pub fn needs_crossing(&self, target_tick: i32, zero_for_one: bool) -> bool {
        if zero_for_one {
            target_tick < self.start_tick_index
        } else {
            target_tick >= self.start_tick_index + TICK_ARRAY_SIZE as i32
        }
    }
}
```

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

### **Partially Implemented**
- **Transient Updates**: Structure exists but not fully integrated with swap operations
- **Batch Operations**: Infrastructure present but no instruction-level batching

### **Not Implemented**
- **Client-Side Caching**: No SDK-level prefetching strategies
- **Compressed Arrays**: No optimization for inactive price ranges
- **Historical Data**: No archival mechanism for old tick data

## 7. Key Differences from Original Design

1. **Array Size**: Changed from 60 to 32 ticks for better memory alignment
2. **Cleanup Mechanism**: Both basic and incentivized cleanup are implemented
3. **Router System**: Added TickArrayRouter for Valence compatibility
4. **Bitmap Size**: Uses 1024-bit bitmap (16 u64s) instead of variable size
5. **Fee Tracking**: Uses token 0/1 naming convention instead of A/B

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

## 10. Testing Checklist

- [x] Unit tests for tick array operations
- [x] Integration tests for cross-array liquidity positions
- [x] Bitmap consistency tests
- [x] PDA derivation tests
- [x] Cleanup mechanism tests
- [ ] Gas usage benchmarks
- [ ] Stress tests with maximum arrays

This implementation provides efficient, secure, and scalable tick array management while maintaining compatibility with concentrated liquidity AMM requirements and optimizing for Solana's architecture.