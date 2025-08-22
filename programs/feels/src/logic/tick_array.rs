/// Tick array management functionality for efficient tick range operations.
/// Provides optimized access patterns and routing between adjacent tick arrays.
/// Handles batch tick updates and cross-array navigation for complex swaps.

use anchor_lang::prelude::*;
use crate::state::{TickArray, TICK_ARRAY_SIZE, Pool, PoolError};

// ============================================================================
// TickArray Implementation
// ============================================================================

/// Additional tick array utilities specific to array management
impl TickArray {
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
        Pubkey::find_program_address(
            &[
                b"tick_array",
                pool.as_ref(),
                &start_tick_index.to_le_bytes(),
            ],
            program_id,
        )
    }
    
    /// Check if a tick array is initialized based on pool's bitmap
    pub fn is_initialized(pool: &Pool, start_tick_index: i32) -> bool {
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
        pool: &mut Pool,
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
        pool: &mut Pool,
        pool_key: &Pubkey,
        tick_index: i32,
        tick_array_account: &AccountInfo<'info>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
        program_id: &Pubkey,
    ) -> Result<()> {
        let start_tick = Self::tick_to_array_start(tick_index);
        
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
        pool: &mut Pool,
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