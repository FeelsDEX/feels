# Tick Array Management Strategy

## Overview

This document defines the strategy for managing TickArray accounts in the Feels Protocol concentrated liquidity AMM. Each TickArray stores 60 consecutive ticks and serves as the fundamental unit for organizing the price space.

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
- Start tick index MUST be divisible by 60 (TICK_ARRAY_SIZE)
- Each pool can have up to ~30,000 tick arrays (covering tick range -887272 to +887272)
- PDAs are deterministic and can be pre-calculated off-chain
- Tick arrays contain 60 consecutive ticks each for efficient memory usage

## 2. Tick Array Lifecycle

### 2.1 Creation (Lazy Initialization)
Tick arrays are created on-demand when:
- A user adds liquidity to a previously empty price range
- The first position references ticks in that array

```rust
// Creation logic in add_liquidity instruction
fn ensure_tick_array_exists(
    pool: &Pubkey,
    tick_index: i32,
    payer: &Signer,
    system_program: &Program<System>,
) -> Result<()> {
    let start_tick = (tick_index / TICK_ARRAY_SIZE) * TICK_ARRAY_SIZE;
    let seeds = tick_array_seeds(pool, start_tick);
    
    // Create account if it doesn't exist
    if !tick_array_account.data_is_empty() {
        return Ok(());
    }
    
    // Initialize with rent-exempt balance
    create_account_with_seed(/* ... */)?;
    
    // Update pool's tick_array_bitmap
    update_bitmap(pool, start_tick, true)?;
}
```

### 2.2 Cleanup Strategy (Rent Reclamation)
To prevent state bloat and reclaim rent:

```rust
// Cleanup empty tick arrays
fn cleanup_empty_tick_array(
    pool: &mut Pool,
    tick_array: &mut TickArray,
    beneficiary: &AccountInfo,
) -> Result<()> {
    // Only cleanup if all ticks are uninitialized
    require!(tick_array.initialized_tick_count == 0, ErrorCode::ArrayNotEmpty);
    
    // Update bitmap
    let array_index = tick_array.start_tick_index / TICK_ARRAY_SIZE;
    let word_index = (array_index / 64) as usize;
    let bit_index = (array_index % 64) as u8;
    pool.tick_array_bitmap[word_index] &= !(1u64 << bit_index);
    
    // Close account and return rent to beneficiary
    close_account(tick_array, beneficiary)?;
}
```

### 2.3 Incentivized Cleanup (Future Implementation)
- Allow anyone to close empty tick arrays and claim a portion of the rent
- Protocol retains 20% of reclaimed rent as treasury fee  
- Remaining 80% goes to the cleanup initiator
- **Status**: Not yet implemented - requires additional security mechanisms

## 3. Efficient Traversal Strategy

### 3.1 Swap Traversal
During swaps, tick arrays must be loaded dynamically:

```rust
pub struct SwapTickArrays {
    // Pre-calculated tick array addresses needed for swap
    pub current: Pubkey,      // Array containing current tick
    pub next: Option<Pubkey>, // Next array in swap direction
    pub boundary: Option<Pubkey>, // Array at price boundary
}

impl SwapTickArrays {
    pub fn calculate(
        pool: &Pool,
        current_tick: i32,
        zero_for_one: bool,
        sqrt_price_limit: u128,
    ) -> Self {
        let current_array = tick_to_array_start(current_tick);
        
        // Pre-calculate potential arrays needed
        let next_array = if zero_for_one {
            current_array - TICK_ARRAY_SIZE
        } else {
            current_array + TICK_ARRAY_SIZE
        };
        
        // Calculate boundary array from price limit
        let boundary_tick = sqrt_price_to_tick(sqrt_price_limit);
        let boundary_array = tick_to_array_start(boundary_tick);
        
        Self {
            current: derive_tick_array_pda(pool, current_array),
            next: Some(derive_tick_array_pda(pool, next_array)),
            boundary: Some(derive_tick_array_pda(pool, boundary_array)),
        }
    }
}
```

### 3.2 Bitmap-Guided Search
Use the tick array bitmap for efficient traversal:

```rust
fn find_next_initialized_tick_array(
    pool: &Pool,
    start_array_index: i32,
    search_direction: bool, // true = up, false = down
) -> Option<i32> {
    let mut current_word = (start_array_index / 64) as usize;
    let mut bit_pos = (start_array_index % 64) as u8;
    
    loop {
        let word = pool.tick_array_bitmap[current_word];
        
        // Search within current word
        let mask = if search_direction {
            !((1u64 << bit_pos) - 1) // Mask bits below
        } else {
            (1u64 << (bit_pos + 1)) - 1 // Mask bits above
        };
        
        let masked_word = word & mask;
        if masked_word != 0 {
            let next_bit = if search_direction {
                masked_word.trailing_zeros()
            } else {
                63 - masked_word.leading_zeros()
            };
            
            return Some((current_word * 64 + next_bit as usize) as i32);
        }
        
        // Move to next word
        if search_direction {
            current_word += 1;
            if current_word >= 16 { break; }
        } else {
            if current_word == 0 { break; }
            current_word -= 1;
        }
        
        bit_pos = if search_direction { 0 } else { 63 };
    }
    
    None
}
```

## 4. Cross-Array Operations

### 4.1 Position Management
When adding/removing liquidity across multiple arrays:

```rust
pub struct PositionTickArrays {
    pub lower: Pubkey,  // Array containing lower tick
    pub upper: Pubkey,  // Array containing upper tick
    pub middle: Vec<Pubkey>, // Arrays fully covered by position
}

impl PositionTickArrays {
    pub fn calculate(pool: &Pubkey, tick_lower: i32, tick_upper: i32) -> Self {
        let lower_array = tick_to_array_start(tick_lower);
        let upper_array = tick_to_array_start(tick_upper);
        
        // Find all arrays between lower and upper
        let mut middle = Vec::new();
        let mut current = lower_array + TICK_ARRAY_SIZE;
        while current < upper_array {
            middle.push(derive_tick_array_pda(pool, current));
            current += TICK_ARRAY_SIZE;
        }
        
        Self {
            lower: derive_tick_array_pda(pool, lower_array),
            upper: derive_tick_array_pda(pool, upper_array),
            middle,
        }
    }
}
```

### 4.2 Batched Updates
For operations affecting multiple tick arrays:

```rust
pub struct BatchedTickUpdate {
    pub updates: Vec<(Pubkey, Vec<TickUpdate>)>, // (tick_array_pda, updates)
}

impl BatchedTickUpdate {
    pub fn execute(&self, remaining_accounts: &[AccountInfo]) -> Result<()> {
        for (array_pda, updates) in &self.updates {
            // Find matching account
            let array_account = remaining_accounts
                .iter()
                .find(|acc| acc.key == array_pda)
                .ok_or(ErrorCode::MissingTickArray)?;
            
            let mut tick_array = TickArray::load_mut(array_account)?;
            
            for update in updates {
                tick_array.update_tick(update)?;
            }
        }
        Ok(())
    }
}
```

## 5. Gas Optimization Strategies

### 5.1 Tick Array Caching
For frequently accessed tick arrays:
- Implement a client-side cache with TTL
- Pre-fetch likely arrays based on current price and volatility
- Bundle multiple array loads in single transaction when possible

### 5.2 Transient Updates
Use TransientTickUpdates for gas efficiency:
```rust
// Collect updates during operation
let transient = TransientTickUpdates::new(pool_key);
transient.add_update(tick_index, liquidity_delta, fee_growth);

// Apply all updates in single pass
transient.apply_to_arrays(remaining_accounts)?;
```

### 5.3 Array Prefetching
For UI/SDK implementations:
```rust
pub fn prefetch_swap_arrays(
    pool: &Pool,
    amount_in: u64,
    zero_for_one: bool,
) -> Vec<Pubkey> {
    // Estimate price impact
    let estimated_ticks = estimate_tick_movement(pool, amount_in);
    
    // Prefetch arrays that might be needed
    let mut arrays = Vec::new();
    let current_array = tick_to_array_start(pool.current_tick);
    
    for i in 0..=estimated_ticks / TICK_ARRAY_SIZE {
        let array_start = if zero_for_one {
            current_array - (i * TICK_ARRAY_SIZE)
        } else {
            current_array + (i * TICK_ARRAY_SIZE)
        };
        arrays.push(derive_tick_array_pda(&pool.key(), array_start));
    }
    
    arrays
}
```

## 6. Implementation Checklist

### Phase 1 (Core Functionality)
- [x] Basic TickArray structure and PDA derivation
- [x] Bitmap tracking of initialized arrays  
- [x] Efficient next_initialized_tick search
- [x] TickArrayRouter for optimized access patterns
- [x] Tick array creation during add_liquidity (integrated in AddLiquidity instruction)
- [x] Basic cleanup mechanism for empty arrays (CleanupTickArray instruction implemented)

### Phase 2 (Optimizations)
- [ ] Batched tick updates across arrays
- [ ] Incentivized cleanup mechanism
- [ ] Client-side array prefetching
- [ ] Transient update optimization
- [ ] Array caching strategy

### Phase 3 (Advanced Features)
- [ ] Compressed tick arrays for inactive ranges
- [ ] Dynamic tick spacing adjustment  
- [ ] Cross-program composability for tick data
- [ ] Historical tick data archival

## Current Implementation Status

### ✅ **Implemented Features**
- **TickArray Structure**: 60-tick arrays with zero-copy optimization
- **TickArrayRouter**: Optimized access patterns for up to 8 pre-registered arrays
- **Bitmap Tracking**: Efficient tick array initialization tracking in Pool state
- **PDA Derivation**: Deterministic tick array addressing
- **Tick Crossing Logic**: Basic tick boundary crossing during swaps
- **Safety Mechanisms**: Bounds checking and overflow protection
- **Lazy Initialization**: Tick arrays automatically created during add_liquidity via TickArrayManager
- **Cleanup Mechanisms**: Rent reclamation for empty tick arrays via cleanup_tick_array instruction

### ⚠️ **Partially Implemented**  
- **Swap Integration**: Core logic exists but needs instruction-level integration
- **Fee Growth Tracking**: Infrastructure present but incomplete fee accumulation
- **Position Management**: Basic structure but missing cross-array position handling

### ❌ **Missing Features**
- **Batched Updates**: No optimization for multiple tick array modifications
- **Client-Side Caching**: No prefetching strategies implemented

## 7. Security Considerations

1. **Array Initialization**: Only allow creation when liquidity is actually added
2. **Cleanup Authorization**: Ensure only truly empty arrays can be closed
3. **Bitmap Consistency**: Always keep bitmap in sync with actual arrays
4. **PDA Validation**: Verify tick array PDAs match expected derivation
5. **Bounds Checking**: Validate all tick indices are within valid ranges

## 8. Testing Strategy

1. **Unit Tests**: Each tick array operation in isolation
2. **Integration Tests**: Multi-array operations (swaps, positions)
3. **Stress Tests**: Maximum arrays per pool, bitmap edge cases
4. **Gas Tests**: Measure CU usage for various array configurations
5. **Security Tests**: Attempt to corrupt bitmap, invalid cleanups

This strategy ensures efficient, secure, and scalable tick array management while maintaining compatibility with the Uniswap V3 model.