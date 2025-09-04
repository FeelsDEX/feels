# WorkUnit Pattern Documentation

## Overview

The WorkUnit pattern is a critical design pattern in the Feels Protocol that ensures all state mutations are atomic. It acts as the exclusive gateway for state changes within an instruction, providing automatic rollback on failure and preventing partial state updates.

## Core Principles

### 1. **Single Source of Truth**
- One WorkUnit per instruction execution
- All state changes tracked in a single place
- No direct account access after WorkUnit creation

### 2. **Atomic Operations**
- All changes committed at once or not at all
- Automatic rollback on any failure
- No partial state updates possible

### 3. **Type Safety**
- Compile-time guarantees about state access
- Prevents accidental direct account manipulation
- Clear separation between read and write operations

## Architecture

```
┌─────────────────┐
│   Instruction   │
│     Handler     │
└────────┬────────┘
         │ 1. Create
         ▼
┌─────────────────┐
│    WorkUnit     │ ◄── Tracks all state changes
└────────┬────────┘
         │ 2. Load accounts
         ▼
┌─────────────────┐
│  StateContext   │ ◄── Provides controlled access
└────────┬────────┘
         │ 3. Use in business logic
         ▼
┌─────────────────┐
│  OrderManager   │ ◄── Executes operations
│  (or other      │
│  business logic)│
└────────┬────────┘
         │ 4. Complete operations
         ▼
┌─────────────────┐
│ Commit WorkUnit │ ◄── Atomic write of all changes
└─────────────────┘
```

## Implementation Guide

### Step 1: Create WorkUnit

```rust
let mut work_unit = WorkUnit::new();
```

### Step 2: Load Required Accounts

```rust
// Load all accounts that will be accessed
work_unit.load_market_field(&ctx.accounts.market_field)?;
work_unit.load_buffer(&ctx.accounts.buffer_account)?;
work_unit.load_market_manager(&ctx.accounts.market_manager)?;

// Load optional accounts
if let Some(oracle) = ctx.accounts.oracle.as_ref() {
    work_unit.load_twap_oracle(oracle)?;
}
```

### Step 3: Create StateContext

```rust
let state_context = create_state_context(
    &mut work_unit,
    &ctx.accounts.market_field,
    &ctx.accounts.buffer_account,
    &ctx.accounts.market_manager,
    ctx.accounts.oracle.as_ref(),
)?;
```

### Step 4: Execute Business Logic

```rust
// Create business logic component with StateContext
let mut order_manager = OrderManager::new(state_context, current_time);

// Execute operations - all mutations tracked by WorkUnit
let result = order_manager.execute_swap(
    route,
    amount_in,
    minimum_amount_out,
    exact_input,
)?;
```

### Step 5: Perform External Operations

```rust
// Token transfers and other external operations
// These happen outside the WorkUnit
token::transfer(/* ... */)?;
```

### Step 6: Commit Changes

```rust
// Atomic write of all changes
// This is the ONLY place where state is written
work_unit.commit()?;
```

### Step 7: Emit Events

```rust
// Events are emitted after successful commit
emit!(SwapEvent { /* ... */ });
```

## Best Practices

### DO:

1. **Load all accounts upfront**
   - Load every account you'll need at the beginning
   - This includes accounts you'll only read from

2. **Use StateContext exclusively**
   - After creating StateContext, never access accounts directly
   - All state access should go through StateContext methods

3. **Keep WorkUnit lifetime minimal**
   - Create WorkUnit as late as possible
   - Commit as soon as operations are complete

4. **Handle errors gracefully**
   - WorkUnit automatically rolls back on drop if not committed
   - No need for manual rollback logic

5. **Test atomicity**
   - Write tests that verify partial failures don't leave inconsistent state
   - Test that drops without commit don't persist changes

### DON'T:

1. **Don't access accounts directly after WorkUnit creation**
   ```rust
   // BAD: Direct access after loading into WorkUnit
   work_unit.load_market_field(&ctx.accounts.market_field)?;
   let field = &ctx.accounts.market_field; // Don't do this!
   
   // GOOD: Use StateContext
   let field_params = state_context.market_field()?;
   ```

2. **Don't create multiple WorkUnits**
   ```rust
   // BAD: Multiple WorkUnits
   let work_unit1 = WorkUnit::new();
   let work_unit2 = WorkUnit::new();
   
   // GOOD: Single WorkUnit
   let mut work_unit = WorkUnit::new();
   ```

3. **Don't commit before all operations complete**
   ```rust
   // BAD: Premature commit
   work_unit.commit()?;
   token::transfer(/* ... */)?; // Could fail!
   
   // GOOD: Commit after all operations
   token::transfer(/* ... */)?;
   work_unit.commit()?;
   ```

## Common Patterns

### Pattern 1: Simple State Update

```rust
pub fn update_market_params(ctx: Context<UpdateMarket>, new_params: MarketParams) -> Result<()> {
    let mut work_unit = WorkUnit::new();
    
    // Load only what we need
    work_unit.load_market_field(&ctx.accounts.market_field)?;
    
    // Get mutable access through WorkUnit
    let market = work_unit.get_market_field_mut(&ctx.accounts.market_field.key())?;
    
    // Make changes
    market.base_fee_rate = new_params.base_fee_rate;
    market.kappa_fee = new_params.kappa_fee;
    
    // Commit atomically
    work_unit.commit()
}
```

### Pattern 2: Complex Multi-Account Operation

```rust
pub fn complex_operation(ctx: Context<ComplexOp>) -> Result<()> {
    let mut work_unit = WorkUnit::new();
    
    // Load all accounts
    work_unit.load_market_field(&ctx.accounts.market_field)?;
    work_unit.load_buffer(&ctx.accounts.buffer)?;
    work_unit.load_market_manager(&ctx.accounts.manager)?;
    for array in &ctx.remaining_accounts {
        if let Ok(tick_array) = AccountLoader::<TickArray>::try_from(array) {
            work_unit.load_tick_array(&tick_array)?;
        }
    }
    
    // Create context and execute
    let state_context = create_state_context(/* ... */)?;
    let mut logic = ComplexLogic::new(state_context);
    logic.execute()?;
    
    // Commit all changes atomically
    work_unit.commit()
}
```

### Pattern 3: Conditional Operations

```rust
pub fn conditional_update(ctx: Context<ConditionalOp>, condition: bool) -> Result<()> {
    let mut work_unit = WorkUnit::new();
    work_unit.load_market_field(&ctx.accounts.market_field)?;
    
    if condition {
        let market = work_unit.get_market_field_mut(&ctx.accounts.market_field.key())?;
        market.is_paused = true;
    }
    
    // Commit only happens if we reach here
    // If condition is false, no changes are made
    work_unit.commit()
}
```

## Error Handling

The WorkUnit pattern provides automatic error handling:

1. **Automatic Rollback**: If an error occurs before `commit()`, all changes are discarded
2. **Drop Safety**: The `Drop` implementation warns about uncommitted changes
3. **No Partial Updates**: Either all changes are applied or none are

```rust
// This is safe - if transfer fails, market updates are not persisted
pub fn swap_with_transfer(ctx: Context<Swap>, amount: u64) -> Result<()> {
    let mut work_unit = WorkUnit::new();
    
    // ... load accounts and execute swap logic ...
    
    // If this fails, work_unit is dropped without commit
    token::transfer(/* ... */)?;
    
    // Only reached if transfer succeeds
    work_unit.commit()
}
```

## Testing

The WorkUnit pattern makes testing more robust:

```rust
#[test]
fn test_atomic_rollback() {
    let mut work_unit = WorkUnit::new();
    
    // Make changes
    let market = work_unit.get_market_field_mut(&market_key)?;
    market.base_fee_rate = 100;
    
    // Don't commit - changes should not persist
    drop(work_unit);
    
    // Verify original state unchanged
    assert_eq!(original_market.base_fee_rate, 50);
}
```

## Performance Considerations

1. **Single Write**: More efficient than multiple account writes
2. **Memory Overhead**: WorkUnit stores copies of state
3. **Batch Operations**: Can batch multiple account updates

## Migration Guide

To migrate existing code to use WorkUnit:

1. Identify all account access in the instruction
2. Load all accounts into WorkUnit at the start
3. Replace direct account access with StateContext methods
4. Move commit() to the end of the instruction
5. Test that atomicity is preserved

## Conclusion

The WorkUnit pattern is essential for maintaining data consistency in the Feels Protocol. By ensuring all state changes are atomic, it prevents partial updates and makes the protocol more robust against failures. Always use this pattern for any instruction that modifies state.