/// Tick and tick array management. Includes fee growth calculations, liquidity
/// state, crossing mechanics, bitmap operations, and TickArrayRouter logic.
/// Also contains router initialization and update functions. Central module
/// for all tick operations in the concentrated liquidity system.

use anchor_lang::prelude::*;
use crate::state::{TickArray, Tick, PoolError};
use crate::utils::{TICK_ARRAY_SIZE, MAX_LIQUIDITY_DELTA};
use crate::utils::{safe_add_i128, safe_sub_i128, add_liquidity_delta, FeeGrowthMath};

// ============================================================================
// Individual Tick Implementation
// ============================================================================

/// Business logic operations for individual Tick management  
impl Tick {
    /// Check if this tick has any liquidity
    pub fn has_liquidity(&self) -> bool {
        self.liquidity_gross > 0
    }

    // ------------------------------------------------------------------------
    // Fee Growth Management
    // ------------------------------------------------------------------------

    /// Set fee growth outside values from u256 representation (token 0)
    pub fn set_fee_growth_outside_0(&mut self, value: [u64; 4]) {
        self.fee_growth_outside_0 = value;
    }

    /// Set fee growth outside values from u256 representation (token 1)
    pub fn set_fee_growth_outside_1(&mut self, value: [u64; 4]) {
        self.fee_growth_outside_1 = value;
    }

    /// Get fee growth outside as u256 for token 0
    pub fn get_fee_growth_outside_0(&self) -> [u64; 4] {
        self.fee_growth_outside_0
    }

    /// Get fee growth outside as u256 for token 1
    pub fn get_fee_growth_outside_1(&self) -> [u64; 4] {
        self.fee_growth_outside_1
    }

    // ------------------------------------------------------------------------
    // Fee Growth Calculations
    // ------------------------------------------------------------------------

    /// Update fee growth outside when crossing a tick
    /// This is used to track fees accumulated outside of a tick's range
    pub fn update_fee_growth_outside(
        &mut self,
        fee_growth_global_0: [u64; 4],
        fee_growth_global_1: [u64; 4],
        is_upper_tick: bool,
    ) -> Result<()> {
        if is_upper_tick {
            // For upper ticks, fee growth outside = global - current outside
            self.fee_growth_outside_0 = FeeGrowthMath::sub_fee_growth(
                fee_growth_global_0,
                self.fee_growth_outside_0,
            )?;
            self.fee_growth_outside_1 = FeeGrowthMath::sub_fee_growth(
                fee_growth_global_1,
                self.fee_growth_outside_1,
            )?;
        } else {
            // For lower ticks, fee growth outside = current outside
            self.fee_growth_outside_0 = fee_growth_global_0;
            self.fee_growth_outside_1 = fee_growth_global_1;
        }
        
        Ok(())
    }

    /// Calculate fee growth inside a tick range
    /// This is used when calculating fees owed to a position
    pub fn calculate_fee_growth_inside(
        tick_lower: &Tick,
        tick_upper: &Tick,
        current_tick: i32,
        fee_growth_global_0: [u64; 4],
        fee_growth_global_1: [u64; 4],
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<([u64; 4], [u64; 4])> {
        // Fee growth below the lower tick
        let fee_growth_below_0 = if current_tick >= tick_lower_index {
            tick_lower.fee_growth_outside_0
        } else {
            FeeGrowthMath::sub_fee_growth(fee_growth_global_0, tick_lower.fee_growth_outside_0)?
        };
        
        let fee_growth_below_1 = if current_tick >= tick_lower_index {
            tick_lower.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth(fee_growth_global_1, tick_lower.fee_growth_outside_1)?
        };

        // Fee growth above the upper tick
        let fee_growth_above_0 = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_0
        } else {
            FeeGrowthMath::sub_fee_growth(fee_growth_global_0, tick_upper.fee_growth_outside_0)?
        };
        
        let fee_growth_above_1 = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth(fee_growth_global_1, tick_upper.fee_growth_outside_1)?
        };

        // Fee growth inside = global - below - above
        let fee_growth_inside_0 = FeeGrowthMath::sub_fee_growth(
            FeeGrowthMath::sub_fee_growth(fee_growth_global_0, fee_growth_below_0)?,
            fee_growth_above_0,
        )?;
        
        let fee_growth_inside_1 = FeeGrowthMath::sub_fee_growth(
            FeeGrowthMath::sub_fee_growth(fee_growth_global_1, fee_growth_below_1)?,
            fee_growth_above_1,
        )?;

        Ok((fee_growth_inside_0, fee_growth_inside_1))
    }
}

// ============================================================================
// TickArray Implementation
// ============================================================================

/// Business logic operations for TickArray management
impl TickArray {
    /// Calculate the seeds for tick array PDA derivation
    pub fn seeds(pool: &Pubkey, start_tick: i32) -> Vec<Vec<u8>> {
        vec![
            b"tick_array".to_vec(),
            pool.to_bytes().to_vec(),
            start_tick.to_le_bytes().to_vec(),
        ]
    }

    // ------------------------------------------------------------------------
    // Tick Access and Indexing
    // ------------------------------------------------------------------------

    /// Get a tick by its index within this array
    pub fn get_tick(&self, tick_index: i32) -> Result<&Tick> {
        let array_index = self.tick_index_to_array_index(tick_index)?;
        Ok(&self.ticks[array_index])
    }

    /// Get a mutable tick by its index within this array
    pub fn get_tick_mut(&mut self, tick_index: i32) -> Result<&mut Tick> {
        let array_index = self.tick_index_to_array_index(tick_index)?;
        Ok(&mut self.ticks[array_index])
    }

    /// Convert a global tick index to an array index within this tick array
    pub fn tick_index_to_array_index(&self, tick_index: i32) -> Result<usize> {
        let relative_index = tick_index - self.start_tick_index;
        
        require!(
            relative_index >= 0 && relative_index < TICK_ARRAY_SIZE as i32,
            PoolError::TickNotFound
        );

        Ok(relative_index as usize)
    }

    /// Check if a tick is within this array's range
    pub fn contains_tick(&self, tick_index: i32) -> bool {
        tick_index >= self.start_tick_index && 
        tick_index < self.start_tick_index + TICK_ARRAY_SIZE as i32
    }

    // ------------------------------------------------------------------------
    // Tick Initialization and Updates
    // ------------------------------------------------------------------------

    /// Initialize a tick within this array
    pub fn initialize_tick(&mut self, tick_index: i32, tick_spacing: i16) -> Result<()> {
        // Validate tick alignment
        require!(
            tick_index % tick_spacing as i32 == 0,
            PoolError::TickNotAligned
        );

        let array_index = self.tick_index_to_array_index(tick_index)?;
        let tick = &mut self.ticks[array_index];

        if tick.initialized == 0 {
            tick.initialized = 1;
            self.initialized_tick_count += 1;
        }

        Ok(())
    }

    /// Update liquidity values for a tick
    pub fn update_tick(
        &mut self,
        tick_index: i32,
        liquidity_delta: i128,
        upper: bool,
    ) -> Result<bool> {
        // Validate liquidity_delta magnitude is reasonable
        // Prevent DoS through extreme values that could cause overflow
        require!(
            liquidity_delta.abs() <= MAX_LIQUIDITY_DELTA,
            PoolError::InvalidLiquidityAmount
        );
        
        let array_index = self.tick_index_to_array_index(tick_index)?;
        let tick = &mut self.ticks[array_index];

        require!(tick.initialized == 1, PoolError::TickNotInitialized);

        let liquidity_gross_before = tick.liquidity_gross;

        // Update gross liquidity using safe arithmetic
        let current_liquidity_gross = tick.liquidity_gross;
        let new_liquidity_gross = add_liquidity_delta(current_liquidity_gross, liquidity_delta)?;
        tick.liquidity_gross = new_liquidity_gross;

        // Update net liquidity using safe arithmetic
        // For lower ticks: liquidity_net increases (liquidity added when price crosses up)
        // For upper ticks: liquidity_net decreases (liquidity removed when price crosses up)
        if upper {
            let current_liquidity_net = tick.liquidity_net;
            let new_liquidity_net = safe_sub_i128(current_liquidity_net, liquidity_delta)?;
            tick.liquidity_net = new_liquidity_net;
        } else {
            let current_liquidity_net = tick.liquidity_net;
            let new_liquidity_net = safe_add_i128(current_liquidity_net, liquidity_delta)?;
            tick.liquidity_net = new_liquidity_net;
        }

        // Return whether this tick flipped from zero to non-zero or vice versa
        let flipped = (liquidity_gross_before == 0) != (tick.liquidity_gross == 0);
        Ok(flipped)
    }

    // ------------------------------------------------------------------------
    // Tick Search and Navigation
    // ------------------------------------------------------------------------

    /// Get the next initialized tick in the array
    pub fn next_initialized_tick(&self, tick_index: i32, lte: bool) -> Option<i32> {
        // Validate that the tick is within this array's range
        if !self.contains_tick(tick_index) {
            return None;
        }
        
        let start_array_index = self.tick_index_to_array_index(tick_index).ok()?;

        if lte {
            // Search downward
            for i in (0..=start_array_index).rev() {
                if self.ticks[i].initialized == 1 {
                    return Some(self.start_tick_index + i as i32);
                }
            }
        } else {
            // Search upward
            for i in (start_array_index + 1)..TICK_ARRAY_SIZE {
                if self.ticks[i].initialized == 1 {
                    return Some(self.start_tick_index + i as i32);
                }
            }
        }

        None
    }

    // ------------------------------------------------------------------------
    // Validation
    // ------------------------------------------------------------------------

    /// Validate that this tick array is properly formed
    pub fn validate(&self) -> Result<()> {
        // Ensure start tick is aligned to array boundaries
        require!(
            self.start_tick_index % TICK_ARRAY_SIZE as i32 == 0,
            PoolError::InvalidTickArrayStart
        );
        
        // Ensure start tick is within valid tick range
        require!(
            self.start_tick_index >= crate::utils::MIN_TICK && 
            self.start_tick_index <= crate::utils::MAX_TICK - TICK_ARRAY_SIZE as i32,
            PoolError::TickOutOfBounds
        );

        // Count initialized ticks and verify the count is accurate
        let actual_count = self.ticks.iter()
            .filter(|tick| tick.initialized == 1)
            .count() as u8;

        require!(
            actual_count == self.initialized_tick_count,
            PoolError::InvalidTickArrayCount
        );

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Additional tick array utilities specific to array management
    // ------------------------------------------------------------------------
    
    /// Get the next tick array start index for a given direction
    pub fn next_array_start_index(&self, zero_for_one: bool) -> i32 {
        if zero_for_one {
            self.start_tick_index - TICK_ARRAY_SIZE as i32
        } else {
            self.start_tick_index + TICK_ARRAY_SIZE as i32
        }
    }
    
    /// Check if this array needs to be crossed during a swap
    pub fn needs_crossing(&self, target_tick: i32, zero_for_one: bool) -> bool {
        if zero_for_one {
            target_tick < self.start_tick_index
        } else {
            target_tick >= self.start_tick_index + TICK_ARRAY_SIZE as i32
        }
    }
    
    /// Get the boundary tick for array crossing
    pub fn get_boundary_tick(&self, zero_for_one: bool) -> i32 {
        if zero_for_one {
            self.start_tick_index
        } else {
            self.start_tick_index + TICK_ARRAY_SIZE as i32 - 1
        }
    }
}

// ============================================================================
// TickArrayManager Implementation
// ============================================================================

/// Tick array management utilities
pub struct TickArrayManager;

impl TickArrayManager {
    /// Calculate the start tick index for a tick array containing the given tick
    pub fn tick_to_array_start(tick_index: i32) -> i32 {
        (tick_index / TICK_ARRAY_SIZE as i32) * TICK_ARRAY_SIZE as i32
    }
    
    /// Derive tick array PDA for a given pool and start tick
    pub fn derive_tick_array_pda(
        pool: &Pubkey,
        start_tick_index: i32,
        program_id: &Pubkey,
    ) -> (Pubkey, u8) {
        crate::utils::CanonicalSeeds::derive_tick_array_pda(
            pool,
            start_tick_index,
            program_id,
        )
    }
    
    /// Check if a tick array is initialized based on pool's bitmap
    pub fn is_initialized(pool: &crate::state::Pool, start_tick_index: i32) -> bool {
        let array_index = start_tick_index / TICK_ARRAY_SIZE as i32;
        let word_index = (array_index / 64) as usize;
        let bit_index = (array_index % 64) as u8;
        
        if word_index >= 16 {
            return false;
        }
        
        (pool.tick_array_bitmap[word_index] & (1u64 << bit_index)) != 0
    }
    
    /// Update the pool's bitmap when a tick array is initialized or cleaned up
    pub fn update_bitmap(
        pool: &mut crate::state::Pool,
        start_tick_index: i32,
        initialized: bool,
    ) -> Result<()> {
        let array_index = start_tick_index / TICK_ARRAY_SIZE as i32;
        let word_index = (array_index / 64) as usize;
        let bit_index = (array_index % 64) as u8;
        
        require!(word_index < 16, PoolError::TickOutOfBounds);
        
        if initialized {
            pool.tick_array_bitmap[word_index] |= 1u64 << bit_index;
        } else {
            pool.tick_array_bitmap[word_index] &= !(1u64 << bit_index);
        }
        
        Ok(())
    }
    
    /// Ensure a tick array exists for the given tick, creating it if necessary
    pub fn ensure_tick_array_exists<'info>(
        pool: &mut crate::state::Pool,
        pool_key: &Pubkey,
        tick_index: i32,
        tick_array_account: &AccountInfo<'info>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
        program_id: &Pubkey,
    ) -> Result<()> {
        let start_tick = Self::tick_to_array_start(tick_index);
        
        // Ensure the tick aligns with pool's tick spacing
        require!(
            tick_index % pool.tick_spacing as i32 == 0,
            PoolError::InvalidTickSpacing
        );
        
        // Validate the array start aligns with TICK_ARRAY_SIZE boundaries
        require!(
            start_tick % TICK_ARRAY_SIZE as i32 == 0,
            PoolError::InvalidTickArrayBoundary
        );
        
        // Check if array is already initialized
        if Self::is_initialized(pool, start_tick) {
            return Ok(());
        }
        
        // Verify the PDA
        let (expected_pda, bump) = Self::derive_tick_array_pda(pool_key, start_tick, program_id);
        require!(
            tick_array_account.key() == expected_pda,
            PoolError::InvalidPool
        );
        
        // Race condition is handled by Solana runtime - if multiple transactions
        // try to create the same account simultaneously, only one will succeed and others
        // will fail with "account already exists" error, ensuring atomic creation
        // Create the account if it doesn't exist or is empty
        if tick_array_account.data_is_empty() {
            let rent = Rent::get()?;
            let space = TickArray::SIZE;
            let rent_exempt_balance = rent.minimum_balance(space);
            
            // Create PDA account
            anchor_lang::system_program::create_account(
                CpiContext::new_with_signer(
                    system_program.to_account_info(),
                    anchor_lang::system_program::CreateAccount {
                        from: payer.to_account_info(),
                        to: tick_array_account.to_account_info(),
                    },
                    &[&[
                        b"tick_array",
                        pool_key.as_ref(),
                        &start_tick.to_le_bytes(),
                        &[bump],
                    ]],
                ),
                rent_exempt_balance,
                space as u64,
                program_id,
            )?;
            
            // Initialize the tick array
            let mut tick_array_data = tick_array_account.try_borrow_mut_data()?;
            let tick_array = bytemuck::from_bytes_mut::<TickArray>(&mut tick_array_data[8..]);
            tick_array.pool = *pool_key;
            tick_array.start_tick_index = start_tick;
            tick_array.initialized_tick_count = 0;
        }
        
        // Update pool's bitmap
        Self::update_bitmap(pool, start_tick, true)?;
        
        Ok(())
    }
    
    /// Clean up an empty tick array and reclaim rent
    pub fn cleanup_empty_tick_array<'info>(
        pool: &mut crate::state::Pool,
        pool_key: &Pubkey,
        tick_array_account: &AccountInfo<'info>,
        beneficiary: &AccountInfo<'info>,
        program_id: &Pubkey,
    ) -> Result<()> {
        let tick_array_data = tick_array_account.try_borrow_data()?;
        let tick_array = bytemuck::from_bytes::<TickArray>(&tick_array_data[8..]);
        
        // Verify this is the correct tick array for this pool
        require!(tick_array.pool == *pool_key, PoolError::InvalidPool);
        
        // Verify PDA derivation
        let start_tick = tick_array.start_tick_index;
        let (expected_pda, _) = Self::derive_tick_array_pda(pool_key, start_tick, program_id);
        require!(
            tick_array_account.key() == expected_pda,
            PoolError::InvalidPool
        );
        
        // Only allow cleanup if no ticks are initialized
        require!(
            tick_array.initialized_tick_count == 0,
            PoolError::InvalidOperation
        );
        
        drop(tick_array_data);
        
        // Update bitmap to mark array as uninitialized
        Self::update_bitmap(pool, start_tick, false)?;
        
        // Transfer rent to beneficiary and close account
        let dest_lamports = beneficiary.lamports();
        let source_lamports = tick_array_account.lamports();
        
        **beneficiary.try_borrow_mut_lamports()? = dest_lamports
            .checked_add(source_lamports)
            .ok_or(PoolError::ArithmeticOverflow)?;
        **tick_array_account.try_borrow_mut_lamports()? = 0;
        
        // Zero out the account data
        tick_array_account.try_borrow_mut_data()?.fill(0);
        
        Ok(())
    }
    
    /// Calculate tick arrays needed for a position spanning multiple arrays
    pub fn calculate_tick_position_arrays(
        pool_key: &Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        program_id: &Pubkey,
    ) -> TickPositionArrays {
        let lower_array_start = Self::tick_to_array_start(tick_lower);
        let upper_array_start = Self::tick_to_array_start(tick_upper);
        
        let mut middle_arrays = Vec::new();
        let mut current_array = lower_array_start + TICK_ARRAY_SIZE as i32;
        
        while current_array < upper_array_start {
            let (pda, _) = Self::derive_tick_array_pda(pool_key, current_array, program_id);
            middle_arrays.push(pda);
            current_array += TICK_ARRAY_SIZE as i32;
        }
        
        let (lower_pda, _) = Self::derive_tick_array_pda(pool_key, lower_array_start, program_id);
        let (upper_pda, _) = Self::derive_tick_array_pda(pool_key, upper_array_start, program_id);
        
        TickPositionArrays {
            lower: lower_pda,
            upper: upper_pda,
            middle: middle_arrays,
        }
    }
}

/// Structure to hold tick arrays needed for a position
pub struct TickPositionArrays {
    pub lower: Pubkey,
    pub upper: Pubkey,
    pub middle: Vec<Pubkey>,
}

/// Tick update structure for batched operations
#[derive(Clone, Debug)]
pub struct TickUpdate {
    pub tick_index: i32,
    pub liquidity_net_delta: i128,
    pub liquidity_gross_delta: u128,
    pub fee_growth_outside_0: [u64; 4],
    pub fee_growth_outside_1: [u64; 4],
    pub initialized: bool,
}

// ============================================================================
// TickArrayRouter Implementation
// ============================================================================

use crate::state::{TickArrayRouter, RouterConfig};
use crate::utils::MAX_ROUTER_ARRAYS;

impl TickArrayRouter {
    /// Register a new tick array in the router
    pub fn register_array(
        &mut self,
        tick_array: Pubkey,
        start_tick: i32,
    ) -> Result<usize> {
        // Check if already registered
        if let Some(index) = self.contains_array(start_tick) {
            return Ok(index);
        }
        
        // Find first available slot
        for i in 0..MAX_ROUTER_ARRAYS {
            if !self.is_slot_active(i) {
                self.tick_arrays[i] = tick_array;
                self.start_indices[i] = start_tick;
                self.active_bitmap |= 1 << i;
                return Ok(i);
            }
        }
        
        Err(PoolError::RouterFull.into())
    }
    
    /// Unregister a tick array from the router
    pub fn unregister_array(&mut self, start_tick: i32) -> Result<()> {
        if let Some(index) = self.contains_array(start_tick) {
            self.tick_arrays[index] = Pubkey::default();
            self.start_indices[index] = i32::MIN;
            self.active_bitmap &= !(1 << index);
            Ok(())
        } else {
            Err(PoolError::TickNotFound.into())
        }
    }
    
    /// Get the optimal set of tick arrays for a price range
    pub fn get_arrays_for_range(
        &self,
        tick_lower: i32,
        tick_upper: i32,
        tick_spacing: i16,
    ) -> Vec<(usize, Pubkey)> {
        let mut arrays = Vec::new();
        
        // Calculate array boundaries - use checked arithmetic to prevent overflow
        let array_tick_size = match (TICK_ARRAY_SIZE as i32).checked_mul(tick_spacing as i32) {
            Some(size) => size,
            None => return arrays, // Return empty on overflow
        };
            
        let lower_array_start = match (tick_lower / array_tick_size).checked_mul(array_tick_size) {
            Some(start) => start,
            None => return arrays, // Return empty on overflow
        };
        let upper_array_start = match (tick_upper / array_tick_size).checked_mul(array_tick_size) {
            Some(start) => start,
            None => return arrays, // Return empty on overflow
        };
        
        // Find all arrays in range
        for i in 0..MAX_ROUTER_ARRAYS {
            if !self.is_slot_active(i) {
                continue;
            }
            
            let start = self.start_indices[i];
            if start >= lower_array_start && start <= upper_array_start {
                arrays.push((i, self.tick_arrays[i]));
            }
        }
        
        arrays.sort_by_key(|(i, _)| self.start_indices[*i]);
        arrays
    }
}

/// Initialize a tick array router for a pool
pub fn initialize_router(
    ctx: Context<crate::state::tick::InitializeRouter>,
    config: RouterConfig,
) -> Result<()> {
    let router = &mut ctx.accounts.router;
    let pool = &ctx.accounts.pool.load()?;
    
    router.pool = ctx.accounts.pool.key();
    router.tick_arrays = [Pubkey::default(); MAX_ROUTER_ARRAYS];
    router.start_indices = [i32::MIN; MAX_ROUTER_ARRAYS];
    router.active_bitmap = 0;
    router.last_update_slot = Clock::get()?.slot;
    router.authority = ctx.accounts.authority.key();
    router._reserved = [0; 64];
    
    // Pre-load arrays around current price
    let current_tick = pool.current_tick;
    let tick_spacing = pool.tick_spacing;
    let current_array_start = (current_tick / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
        * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    
    // Register current array and surrounding arrays
    let arrays_to_load = config.arrays_around_current as i32;
    for i in -arrays_to_load..=arrays_to_load {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        
        // Derive tick array PDA using helper
        let (tick_array_pda, _) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
            &ctx.accounts.pool.key(),
            start_tick,
            ctx.program_id,
        );
        
        // The PDA derivation above ensures the address is owned by our program
        // Additional validation happens in TickArrayManager::is_initialized which
        // checks the pool's tick bitmap to ensure only valid tick arrays are registered
        
        // Register if it exists (check bitmap)
        if TickArrayManager::is_initialized(pool, start_tick) {
            router.register_array(tick_array_pda, start_tick)?;
        }
    }
    
    Ok(())
}

/// Update router with new tick arrays based on current price
pub fn update_router_arrays(
    ctx: Context<crate::state::tick::UpdateRouter>,
    config: RouterConfig,
) -> Result<()> {
    let router_key = ctx.accounts.router.key();
    let router = &mut ctx.accounts.router;
    let pool = &ctx.accounts.pool.load()?;
    let clock = Clock::get()?;
    
    // Validate that caller has authority to update router
    require!(
        ctx.accounts.authority.key() == router.authority,
        PoolError::InvalidAuthority
    );
    
    // Only update if enough time has passed (use config.update_frequency)
    require!(
        clock.slot >= router.last_update_slot + config.update_frequency,
        PoolError::InvalidOperation
    );
    
    // Create a temporary copy of the router state
    // Build new bitmap atomically to avoid race condition
    let mut new_bitmap = 0u8;
    let mut new_arrays = [Pubkey::default(); MAX_ROUTER_ARRAYS];
    let mut new_indices = [i32::MIN; MAX_ROUTER_ARRAYS];
    let mut new_count = 0usize;
    
    // Re-populate based on current price
    let current_tick = pool.current_tick;
    let tick_spacing = pool.tick_spacing;
    let current_array_start = (current_tick / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32)) 
        * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
    
    // Load arrays around current price using config.arrays_around_current
    let arrays_to_load = config.arrays_around_current as i32;
    for i in -arrays_to_load..=arrays_to_load {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        
        // Derive tick array PDA using helper
        let (tick_array_pda, _) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
            &ctx.accounts.pool.key(),
            start_tick,
            ctx.program_id,
        );
        
        // The PDA derivation above ensures the address is owned by our program
        // Additional validation happens in TickArrayManager::is_initialized
        
        if TickArrayManager::is_initialized(pool, start_tick) {
            // Add to temporary arrays
            if new_count < MAX_ROUTER_ARRAYS {
                new_arrays[new_count] = tick_array_pda;
                new_indices[new_count] = start_tick;
                new_bitmap |= 1u8 << new_count;
                new_count += 1;
            }
        }
    }
    
    // Atomically update the router state
    router.active_bitmap = new_bitmap;
    router.tick_arrays = new_arrays;
    router.start_indices = new_indices;
    
    router.last_update_slot = clock.slot;
    
    emit!(crate::logic::event::RouterUpdatedEvent {
        pool: ctx.accounts.pool.key(),
        previous_router: None,
        new_router: router_key,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}