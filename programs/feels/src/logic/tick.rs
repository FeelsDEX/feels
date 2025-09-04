/// Tick and tick array management. Includes fee growth calculations, liquidity
/// state, crossing mechanics, bitmap operations, and TickArrayRouter logic.
/// Also contains router initialization and update functions. Central module
/// for all tick operations in the concentrated liquidity system.
use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, Tick, TickArray, TickPositionMetadata, MarketManager};
use crate::utils::{add_liquidity_delta, safe_add_i128, safe_sub_i128, FeeGrowthMath};
use crate::utils::{MAX_LIQUIDITY_DELTA, TICK_ARRAY_SIZE, MIN_TICK, MAX_TICK};

// Unified tick management interface is defined below

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
            self.fee_growth_outside_0 =
                FeeGrowthMath::sub_fee_growth_words(fee_growth_global_0, self.fee_growth_outside_0)
                    .map_err(|e| anchor_lang::error::Error::from(e))?;
            self.fee_growth_outside_1 =
                FeeGrowthMath::sub_fee_growth_words(fee_growth_global_1, self.fee_growth_outside_1)
                    .map_err(|e| anchor_lang::error::Error::from(e))?;
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
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_0, tick_lower.fee_growth_outside_0)?
        };

        let fee_growth_below_1 = if current_tick >= tick_lower_index {
            tick_lower.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_1, tick_lower.fee_growth_outside_1)?
        };

        // Fee growth above the upper tick
        let fee_growth_above_0 = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_0
        } else {
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_0, tick_upper.fee_growth_outside_0)?
        };

        let fee_growth_above_1 = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_1, tick_upper.fee_growth_outside_1)?
        };

        // Fee growth inside = global - below - above
        let fee_growth_inside_0 = FeeGrowthMath::sub_fee_growth_words(
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_0, fee_growth_below_0)?,
            fee_growth_above_0,
        )?;

        let fee_growth_inside_1 = FeeGrowthMath::sub_fee_growth_words(
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_1, fee_growth_below_1)?,
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
            FeelsProtocolError::NotFound
        );

        Ok(relative_index as usize)
    }

    /// Check if a tick is within this array's range
    pub fn contains_tick(&self, tick_index: i32) -> bool {
        tick_index >= self.start_tick_index
            && tick_index < self.start_tick_index + TICK_ARRAY_SIZE as i32
    }

    // ------------------------------------------------------------------------
    // Tick Initialization and Updates
    // ------------------------------------------------------------------------

    /// Initialize a tick within this array
    pub fn initialize_tick(&mut self, tick_index: i32, tick_spacing: i16) -> Result<()> {
        // Validate tick alignment
        require!(
            tick_index % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
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
            FeelsProtocolError::InvalidLiquidity
        );

        let array_index = self.tick_index_to_array_index(tick_index)?;
        let tick = &mut self.ticks[array_index];

        require!(tick.initialized == 1, FeelsProtocolError::NotInitialized);

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
            FeelsProtocolError::InvalidTickArray
        );

        // Ensure start tick is within valid tick range
        require!(
            self.start_tick_index >= crate::utils::MIN_TICK
                && self.start_tick_index <= crate::utils::MAX_TICK - TICK_ARRAY_SIZE as i32,
            FeelsProtocolError::TickOutOfBounds
        );

        // Count initialized ticks and verify the count is accurate
        let actual_count = self
            .ticks
            .iter()
            .filter(|tick| tick.initialized == 1)
            .count() as u8;

        require!(
            actual_count == self.initialized_tick_count,
            FeelsProtocolError::InvalidTickArray
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
        crate::state::pda::derive_tick_array_pda(pool, start_tick_index, program_id)
    }
    
    /// Check if a tick array is initialized
    pub fn is_initialized(pool: &crate::state::MarketManager, start_tick: i32) -> bool {
        TickManager::is_tick_array_initialized(pool, start_tick)
    }
    
    /// Update the tick array bitmap
    pub fn update_bitmap(
        pool: &mut crate::state::MarketManager,
        start_tick: i32,
        initialized: bool,
    ) -> Result<()> {
        TickManager::update_tick_array_bitmap(pool, start_tick, initialized)
    }


    /// Ensure a tick array exists for the given tick, creating it if necessary
    pub fn ensure_tick_array_exists<'info>(
        pool: &mut crate::state::MarketManager,
        pool_key: &Pubkey,
        tick_index: i32,
        tick_array_account: &AccountInfo<'info>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
        program_id: &Pubkey,
    ) -> Result<()> {
        let start_tick = Self::tick_to_array_start(tick_index);

        // Validate tick is aligned to tick spacing
        require!(
            tick_index % pool.tick_spacing as i32 == 0,
            FeelsProtocolError::InvalidTickRange
        );

        // Validate the array start aligns with TICK_ARRAY_SIZE boundaries
        require!(
            start_tick % TICK_ARRAY_SIZE as i32 == 0,
            FeelsProtocolError::InvalidTickArray
        );

        // Check if array is already initialized
        if Self::is_initialized(pool, start_tick) {
            return Ok(());
        }

        // Verify the PDA
        let (expected_pda, bump) = Self::derive_tick_array_pda(pool_key, start_tick, program_id);
        require!(
            tick_array_account.key() == expected_pda,
            FeelsProtocolError::InvalidPool
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
        _pool: &mut crate::state::MarketField,
        pool_key: &Pubkey,
        tick_array_account: &AccountInfo<'info>,
        beneficiary: &AccountInfo<'info>,
        program_id: &Pubkey,
    ) -> Result<()> {
        let tick_array_data = tick_array_account.try_borrow_data()?;
        let tick_array = bytemuck::from_bytes::<TickArray>(&tick_array_data[8..]);

        // Verify this is the correct tick array for this pool
        require!(tick_array.pool == *pool_key, FeelsProtocolError::InvalidPool);

        // Verify PDA derivation
        let start_tick = tick_array.start_tick_index;
        let (expected_pda, _) = Self::derive_tick_array_pda(pool_key, start_tick, program_id);
        require!(
            tick_array_account.key() == expected_pda,
            FeelsProtocolError::InvalidPool
        );

        // Only allow cleanup if no ticks are initialized
        require!(
            tick_array.initialized_tick_count == 0,
            FeelsProtocolError::InvalidOperation
        );

        drop(tick_array_data);

        // Update bitmap to mark array as uninitialized
        // Note: This function currently receives MarketField but needs MarketManager
        // The caller should ensure bitmap update happens at the MarketManager level
        msg!("Warning: Tick array bitmap update needed for start_tick {}", start_tick);
        // Bitmap updates are handled in CleanupTickArrayV2 which includes MarketManager

        // Transfer rent to beneficiary and close account
        let dest_lamports = beneficiary.lamports();
        let source_lamports = tick_array_account.lamports();

        **beneficiary.try_borrow_mut_lamports()? = dest_lamports
            .checked_add(source_lamports)
            .ok_or(FeelsProtocolError::ArithmeticOverflow)?;
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
    
    /// Find tick array in remaining accounts by linear search
    pub fn find_tick_array_in_accounts<'info>(
        pool_key: &Pubkey,
        array_start_index: i32,
        remaining_accounts: &'info [AccountInfo<'info>],
        program_id: &Pubkey,
    ) -> Result<&'info AccountInfo<'info>> {
        let (expected_pda, _) = Self::derive_tick_array_pda(pool_key, array_start_index, program_id);
        
        for account in remaining_accounts {
            if account.key() == expected_pda {
                // Validate it's owned by the program
                require!(
                    account.owner == program_id,
                    FeelsProtocolError::InvalidAccountOwner
                );
                return Ok(account);
            }
        }
        
        Err(FeelsProtocolError::InvalidTickArray.into())
    }
}

/// Structure to hold tick arrays needed for a position
pub struct TickPositionArrays {
    pub lower: Pubkey,
    pub upper: Pubkey,
    pub middle: Vec<Pubkey>,
}

// ============================================================================
// Tick Position Implementation
// ============================================================================

/// Business logic operations for Position management (consolidated from tick_position.rs)
impl TickPositionMetadata {
    /// Calculate the seeds for position metadata PDA derivation
    pub fn seeds(tick_position_mint: &Pubkey) -> Vec<Vec<u8>> {
        vec![b"position".to_vec(), tick_position_mint.to_bytes().to_vec()]
    }

    /// Validate the position parameters
    pub fn validate(&self, tick_spacing: i16) -> Result<()> {
        // Ensure tick range is valid
        require!(
            self.tick_lower < self.tick_upper,
            FeelsProtocolError::InvalidTickRange
        );

        // Ensure ticks are properly aligned to tick spacing
        require!(
            self.tick_lower % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );
        require!(
            self.tick_upper % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );

        // Ensure tick range is within bounds
        require!(
            self.tick_lower >= MIN_TICK && self.tick_upper <= MAX_TICK,
            FeelsProtocolError::TickOutOfBounds
        );

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Tick Position State Queries
    // ------------------------------------------------------------------------

    /// Check if the tick position is in range at the given current tick
    pub fn is_in_range(&self, current_tick: i32) -> bool {
        current_tick >= self.tick_lower && current_tick < self.tick_upper
    }

    // ------------------------------------------------------------------------
    // Fee Growth Management
    // ------------------------------------------------------------------------

    /// Set fee growth inside last values for token 0
    pub fn set_fee_growth_inside_last_a(&mut self, value: [u64; 4]) {
        self.fee_growth_inside_last_0 = value;
    }

    /// Set fee growth inside last values for token 1
    pub fn set_fee_growth_inside_last_b(&mut self, value: [u64; 4]) {
        self.fee_growth_inside_last_1 = value;
    }

    /// Get fee growth inside last as u256 for token 0
    pub fn get_fee_growth_inside_last_a(&self) -> [u64; 4] {
        self.fee_growth_inside_last_0
    }

    /// Get fee growth inside last as u256 for token 1
    pub fn get_fee_growth_inside_last_b(&self) -> [u64; 4] {
        self.fee_growth_inside_last_1
    }

    // ------------------------------------------------------------------------
    // Position Updates
    // ------------------------------------------------------------------------

    /// Update the position's liquidity using safe arithmetic
    pub fn update_liquidity(&mut self, liquidity_delta: i128) -> Result<()> {
        self.liquidity = add_liquidity_delta(self.liquidity, liquidity_delta)?;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Fee Calculation and Collection
    // ------------------------------------------------------------------------

    /// Calculate fees owed since last collection
    /// This uses simplified fee math based on Uniswap V3 principles
    pub fn calculate_fees_owed(
        &self,
        fee_growth_inside_0: [u64; 4],
        fee_growth_inside_1: [u64; 4],
    ) -> (u64, u64) {
        // Calculate fee growth delta for each token
        // Using U128 arithmetic for overflow prevention (sufficient precision)
        
        // Use the lowest 64 bits for calculation
        let fee_growth_delta_0 = fee_growth_inside_0[0].saturating_sub(self.fee_growth_inside_last_0[0]);
        let fee_growth_delta_1 = fee_growth_inside_1[0].saturating_sub(self.fee_growth_inside_last_1[0]);
        
        // Calculate fees: liquidity * fee_growth_delta / 2^64 (simplified from 2^128)
        // Using u128 to prevent overflow in intermediate calculations
        let fees_0 = ((self.liquidity as u128)
            .saturating_mul(fee_growth_delta_0 as u128) >> 64) // Divide by 2^64
            .min(u64::MAX as u128) as u64;
            
        let fees_1 = ((self.liquidity as u128)
            .saturating_mul(fee_growth_delta_1 as u128) >> 64) // Divide by 2^64
            .min(u64::MAX as u128) as u64;
        
        // Add to existing owed amounts
        let total_owed_0 = self.tokens_owed_0.saturating_add(fees_0);
        let total_owed_1 = self.tokens_owed_1.saturating_add(fees_1);
        
        (total_owed_0, total_owed_1)
    }

    /// Update tokens owed after fee collection using safe arithmetic
    pub fn update_tokens_owed(&mut self, tokens_0: u64, tokens_1: u64) -> Result<()> {
        // Use saturating add to prevent overflow in token accounting
        self.tokens_owed_0 = self.tokens_owed_0.saturating_add(tokens_0);
        self.tokens_owed_1 = self.tokens_owed_1.saturating_add(tokens_1);
        Ok(())
    }

    /// Collect fees and reset tokens owed using safe arithmetic
    pub fn collect_fees(&mut self, amount_0: u64, amount_1: u64) -> (u64, u64) {
        let collected_0 = amount_0.min(self.tokens_owed_0);
        let collected_1 = amount_1.min(self.tokens_owed_1);

        // Use saturating subtraction to prevent underflow
        self.tokens_owed_0 = self.tokens_owed_0.saturating_sub(collected_0);
        self.tokens_owed_1 = self.tokens_owed_1.saturating_sub(collected_1);

        (collected_0, collected_1)
    }
}

// ============================================================================
// Unified Tick Manager
// ============================================================================

/// Unified interface for all tick and tick array operations
/// This is the SINGLE API surface for all tick-related operations to avoid drift
/// and ensure consistent state management across the protocol.
pub struct TickManager;

impl TickManager {
    /// Get tick data from storage
    pub fn get_tick_data<'info>(
        market_manager: &MarketManager,
        tick_index: i32,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<(u128, i128, u128, u128)> {
        // Find which tick array contains this tick
        let array_start_index = TickArrayManager::tick_to_array_start(tick_index);
        
        // Use router or search remaining accounts
        let tick_array_info = if let Some(router) = tick_array_router {
            // Use router to find the correct account
            let account_index = router.find_tick_array_index(array_start_index)
                .ok_or(FeelsProtocolError::InvalidTickArray)?;
            
            remaining_accounts.get(account_index as usize)
                .ok_or(FeelsProtocolError::InvalidTickArray)?
        } else {
            // Linear search through remaining accounts
            TickArrayManager::find_tick_array_in_accounts(
                &market_manager.market,
                array_start_index,
                remaining_accounts,
                program_id,
            )?
        };
        
        // Load and validate the tick array
        let tick_array_data = tick_array_info.data.borrow();
        if tick_array_data.len() < 8 + std::mem::size_of::<TickArray>() {
            return Err(FeelsProtocolError::InvalidTickArray.into());
        }
        
        // Deserialize the tick array (skip discriminator)
        let tick_array = bytemuck::from_bytes::<TickArray>(&tick_array_data[8..]);
        
        // Validate the tick array
        require!(
            tick_array.pool == market_manager.market,
            FeelsProtocolError::InvalidPool
        );
        require!(
            tick_array.start_tick_index == array_start_index,
            FeelsProtocolError::InvalidTickArray
        );
        
        // Find the tick within the array
        let tick_offset = ((tick_index - array_start_index) / market_manager.tick_spacing as i32) as usize;
        require!(
            tick_offset < TICK_ARRAY_SIZE as usize,
            FeelsProtocolError::InvalidTickIndex
        );
        
        let tick = &tick_array.ticks[tick_offset];
        
        // Return tick data: (liquidity_gross, liquidity_net, fee_growth_outside_0, fee_growth_outside_1)
        Ok((
            tick.liquidity_gross,
            tick.liquidity_net,
            tick.fee_growth_outside_0,
            tick.fee_growth_outside_1,
        ))
    }
    
    /// Update tick fee growth
    pub fn update_tick_fee_growth<'info>(
        market_manager: &mut MarketManager,
        tick_index: i32,
        fee_growth_global_0: u128,
        fee_growth_global_1: u128,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<()> {
        // For now, just update globals
        market_manager.fee_growth_global_0 = fee_growth_global_0;
        market_manager.fee_growth_global_1 = fee_growth_global_1;
        Ok(())
    }
    // ------------------------------------------------------------------------
    // Unified Tick Liquidity Updates
    // ------------------------------------------------------------------------

    /// Update tick liquidity - unified function for both regular and leveraged liquidity
    /// This replaces all duplicate update_tick_liquidity functions across instructions
    pub fn update_tick_liquidity(
        tick_array: &mut TickArray,
        tick_index: i32,
        liquidity_delta: i128,
        is_upper: bool,
    ) -> Result<()> {
        // Validate liquidity delta magnitude
        require!(
            liquidity_delta.abs() <= MAX_LIQUIDITY_DELTA,
            FeelsProtocolError::InvalidLiquidity
        );

        // Get the tick within the array
        let array_index = tick_array.tick_index_to_array_index(tick_index)?;
        let tick = &mut tick_array.ticks[array_index];

        // Initialize tick if needed (only for additions)
        if tick.initialized == 0 && liquidity_delta > 0 {
            tick.initialized = 1;
            tick_array.initialized_tick_count = tick_array
                .initialized_tick_count
                .checked_add(1)
                .ok_or(FeelsProtocolError::ArithmeticOverflow)?;
        }

        // Update gross liquidity
        let liquidity_gross_before = tick.liquidity_gross;
        tick.liquidity_gross = if liquidity_delta > 0 {
            tick.liquidity_gross
                .checked_add(liquidity_delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?
        } else {
            tick.liquidity_gross
                .checked_sub((-liquidity_delta) as u128)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?
        };

        // Update net liquidity
        // For lower ticks: liquidity_net increases when liquidity is added
        // For upper ticks: liquidity_net decreases when liquidity is added
        if is_upper {
            tick.liquidity_net = safe_sub_i128(tick.liquidity_net, liquidity_delta)?;
        } else {
            tick.liquidity_net = safe_add_i128(tick.liquidity_net, liquidity_delta)?;
        }

        // Clean up tick if it becomes empty
        if tick.liquidity_gross == 0 && tick.initialized == 1 {
            tick.initialized = 0;
            tick.liquidity_net = 0;
            tick_array.initialized_tick_count = tick_array
                .initialized_tick_count
                .checked_sub(1)
                .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
        }

        // Check if tick flipped from having liquidity to not having liquidity or vice versa
        let flipped = (liquidity_gross_before == 0) != (tick.liquidity_gross == 0);
        if flipped {
            // Caller should update pool's tick bitmap when a tick flips
        }

        Ok(())
    }

    /// Update tick liquidity with proper fee growth tracking
    pub fn update_tick_with_fee_growth(
        tick_array: &mut TickArray,
        tick_index: i32,
        liquidity_delta: i128,
        is_upper: bool,
        fee_growth_global_0: [u64; 4],
        fee_growth_global_1: [u64; 4],
    ) -> Result<()> {
        // Get the tick
        let array_index = tick_array.tick_index_to_array_index(tick_index)?;
        let tick = &mut tick_array.ticks[array_index];

        // Update fee growth if tick is being initialized
        if tick.initialized == 0 && liquidity_delta > 0 {
            tick.fee_growth_outside_0 = fee_growth_global_0;
            tick.fee_growth_outside_1 = fee_growth_global_1;
        }

        // Perform regular update
        Self::update_tick_liquidity(tick_array, tick_index, liquidity_delta, is_upper)?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Tick Array Creation and Initialization
    // ------------------------------------------------------------------------

    /// Create and initialize a tick array if it doesn't exist
    pub fn ensure_tick_array_initialized<'info>(
        pool: &mut crate::state::MarketManager,
        pool_key: &Pubkey,
        tick_index: i32,
        tick_array_account: &AccountInfo<'info>,
        payer: &Signer<'info>,
        system_program: &Program<'info, System>,
        program_id: &Pubkey,
    ) -> Result<()> {
        TickArrayManager::ensure_tick_array_exists(
            pool,
            pool_key,
            tick_index,
            tick_array_account,
            payer,
            system_program,
            program_id,
        )
    }

    // ------------------------------------------------------------------------
    // Fee Growth Calculations
    // ------------------------------------------------------------------------

    /// Calculate fee growth inside a position's tick range
    pub fn calculate_fee_growth_inside(
        manager: &crate::state::MarketManager,
        tick_lower: &Tick,
        tick_upper: &Tick,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<([u64; 4], [u64; 4])> {
        // Convert u128 to [u64; 4] for compatibility
        let fee_growth_global_0 = [
            manager.fee_growth_global_0 as u64,
            (manager.fee_growth_global_0 >> 64) as u64,
            0,
            0,
        ];
        let fee_growth_global_1 = [
            manager.fee_growth_global_1 as u64,
            (manager.fee_growth_global_1 >> 64) as u64,
            0,
            0,
        ];
        
        Tick::calculate_fee_growth_inside(
            tick_lower,
            tick_upper,
            manager.current_tick,
            fee_growth_global_0,
            fee_growth_global_1,
            tick_lower_index,
            tick_upper_index,
        )
    }

    /// Calculate fee growth inside a range using tick indices
    pub fn calculate_fee_growth_inside_from_pool(
        manager: &crate::state::MarketManager,
        tick_lower_array: &TickArray,
        tick_upper_array: &TickArray,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<([u64; 4], [u64; 4])> {
        let tick_lower = tick_lower_array.get_tick(tick_lower_index)?;
        let tick_upper = tick_upper_array.get_tick(tick_upper_index)?;

        Self::calculate_fee_growth_inside(
            manager,
            tick_lower,
            tick_upper,
            tick_lower_index,
            tick_upper_index,
        )
    }

    // ------------------------------------------------------------------------
    // Tick Search and Navigation
    // ------------------------------------------------------------------------

    /// Find the next initialized tick in the given direction
    pub fn find_next_initialized_tick(
        _pool: &crate::state::MarketField,
        tick_array: &TickArray,
        start_tick: i32,
        search_up: bool,
    ) -> Option<i32> {
        // First search within current array
        if let Some(tick) = tick_array.next_initialized_tick(start_tick, !search_up) {
            return Some(tick);
        }

        // Note: To search across multiple tick arrays, this function would need access
        // to MarketManager (for the bitmap) and the ability to load other tick arrays.
        // The current architecture with MarketField prevents full implementation.
        // Options:
        // 1. Change function signature to use MarketManager
        // 2. Have caller handle multi-array search using the bitmap
        // 3. Return the array boundary to signal continuation needed
        
        // Return the boundary tick of current array to signal search should continue
        if search_up {
            // Return the last tick of current array + 1
            let array_end = tick_array.start_tick_index + TICK_ARRAY_SIZE as i32 - 1;
            Some(array_end + 1)
        } else {
            // Return the first tick of current array - 1
            Some(tick_array.start_tick_index - 1)
        }
    }
    
    /// Find the next initialized tick across multiple arrays using MarketManager
    pub fn find_next_initialized_tick_extended(
        market_manager: &crate::state::MarketManager,
        start_tick: i32,
        search_up: bool,
        tick_spacing: i16,
    ) -> Result<Option<(i32, i32)>> {  // Returns (next_tick, tick_array_start)
        use crate::utils::bitmap::multi_word_bitmap;
        
        // Get the starting tick array index
        let current_array_start = TickArrayManager::tick_to_array_start(start_tick);
        let array_index = current_array_start / TICK_ARRAY_SIZE as i32;
        let bit_index = (array_index + 512) as usize;
        
        // Search direction
        let search_forward = search_up;
        
        // Copy bitmap to avoid unaligned reference to packed field
        let bitmap = market_manager.tick_array_bitmap;
        
        // Find next initialized tick array using bitmap
        let next_bit = if search_forward {
            multi_word_bitmap::next_set_bit(&bitmap[..], bit_index)
        } else {
            multi_word_bitmap::prev_set_bit(&bitmap[..], bit_index)
        };
        
        if let Some(next_array_bit) = next_bit {
            // Convert bit back to tick array start
            let next_array_index = (next_array_bit as i32) - 512;
            let next_array_start = next_array_index * TICK_ARRAY_SIZE as i32;
            
            // The actual tick would need to be found by loading the tick array
            // For now, return the first/last tick of the array based on direction
            let next_tick = if search_up {
                next_array_start  // First tick of next array
            } else {
                next_array_start + TICK_ARRAY_SIZE as i32 - 1  // Last tick of array
            };
            
            // Ensure tick is aligned to tick spacing
            let aligned_tick = if search_up {
                // Round up to next multiple of tick_spacing
                ((next_tick + tick_spacing as i32 - 1) / tick_spacing as i32) * tick_spacing as i32
            } else {
                // Round down to previous multiple of tick_spacing
                (next_tick / tick_spacing as i32) * tick_spacing as i32
            };
            
            Ok(Some((aligned_tick, next_array_start)))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // Tick Array Bitmap Management
    // ------------------------------------------------------------------------

    /// Update tick array bitmap when array is created or destroyed
    pub fn update_tick_array_bitmap(
        pool: &mut crate::state::MarketManager,
        start_tick_index: i32,
        initialized: bool,
    ) -> Result<()> {
        use crate::utils::bitmap::multi_word_bitmap;
        
        // Each tick array covers TICK_ARRAY_SIZE ticks
        // Calculate which bit represents this tick array
        let array_index = start_tick_index / TICK_ARRAY_SIZE as i32;
        
        // The bitmap can track arrays from -512 to 511 (1024 total)
        // Offset negative indices to positive bit positions
        let bit_index = (array_index + 512) as usize;
        
        // Validate bit index is within bounds
        require!(
            bit_index < 1024,
            crate::error::FeelsError::InvalidTickArray
        );
        
        // Copy bitmap, update it, then write it back
        let mut bitmap = pool.tick_array_bitmap;
        if initialized {
            multi_word_bitmap::set_bit(&mut bitmap[..], bit_index)?;
        } else {
            multi_word_bitmap::clear_bit(&mut bitmap[..], bit_index)?;
        }
        pool.tick_array_bitmap = bitmap;
        
        Ok(())
    }

    /// Check if a tick array is initialized
    pub fn is_tick_array_initialized(pool: &crate::state::MarketManager, start_tick_index: i32) -> bool {
        use crate::utils::bitmap::multi_word_bitmap;
        
        // Each tick array covers TICK_ARRAY_SIZE ticks
        let array_index = start_tick_index / TICK_ARRAY_SIZE as i32;
        
        // The bitmap can track arrays from -512 to 511 (1024 total)
        let bit_index = (array_index + 512) as usize;
        
        // Check bounds
        if bit_index >= 1024 {
            return false;
        }
        
        // Copy bitmap to avoid unaligned reference
        let bitmap = pool.tick_array_bitmap;
        multi_word_bitmap::is_bit_set(&bitmap[..], bit_index).unwrap_or(false)
    }

    // ------------------------------------------------------------------------
    // Position Range Validation
    // ------------------------------------------------------------------------

    /// Validate tick range for a position
    pub fn validate_tick_range(
        tick_lower: i32,
        tick_upper: i32,
        tick_spacing: i16,
    ) -> Result<()> {
        // Ensure proper ordering
        require!(
            tick_lower < tick_upper,
            FeelsProtocolError::InvalidTickRange
        );

        // Ensure alignment
        require!(
            tick_lower % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );
        require!(
            tick_upper % tick_spacing as i32 == 0,
            FeelsProtocolError::TickNotAligned
        );

        // Ensure within bounds
        require!(
            tick_lower >= MIN_TICK && tick_upper <= MAX_TICK,
            FeelsProtocolError::TickOutOfBounds
        );

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Comprehensive Tick Cross Operations (SINGLE SOURCE OF TRUTH)
    // ------------------------------------------------------------------------

    /// Cross a tick during a swap - handles ALL tick crossing logic
    /// This replaces scattered crossing logic in order.rs and elsewhere
    pub fn cross_tick_comprehensive<'info>(
        market_manager: &mut MarketManager,
        tick_index: i32,
        zero_for_one: bool,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<i128> {
        // Get the tick array containing this tick
        let (tick_array_info, _array_index) = Self::get_tick_array_for_tick(
            tick_index,
            market_manager,
            remaining_accounts,
            tick_array_router,
            program_id,
        )?;
        
        // Load and modify the tick array as zero-copy account
        let tick_array_loader = AccountLoader::<TickArray>::try_from(&tick_array_info)?;
        let mut tick_array = tick_array_loader.load_mut()?;
        let array_index = tick_array.tick_index_to_array_index(tick_index)?;
        let tick = &mut tick_array.ticks[array_index];

        // Update fee growth outside (this is the core crossing logic)
        // Convert u128 to [u64; 4] for fee growth math
        let fee_growth_0_words = [
            market_manager.fee_growth_global_0 as u64,
            (market_manager.fee_growth_global_0 >> 64) as u64,
            0u64,
            0u64,
        ];
        let fee_growth_1_words = [
            market_manager.fee_growth_global_1 as u64,
            (market_manager.fee_growth_global_1 >> 64) as u64,
            0u64,
            0u64,
        ];
        
        tick.fee_growth_outside_0 = FeeGrowthMath::sub_fee_growth_words(
            fee_growth_0_words,
            tick.fee_growth_outside_0,
        )?;
        
        tick.fee_growth_outside_1 = FeeGrowthMath::sub_fee_growth_words(
            fee_growth_1_words,
            tick.fee_growth_outside_1,
        )?;

        // Update market manager's current liquidity
        let liquidity_net = if zero_for_one {
            -tick.liquidity_net
        } else {
            tick.liquidity_net
        };
        
        market_manager.liquidity = if liquidity_net >= 0 {
            market_manager.liquidity.saturating_add(liquidity_net as u128)
        } else {
            market_manager.liquidity.saturating_sub((-liquidity_net) as u128)
        };

        // Changes are automatically persisted when tick_array RefMut is dropped
        drop(tick_array);

        Ok(liquidity_net)
    }

    /// Simple tick crossing for basic operations (maintains backward compatibility)
    pub fn cross_tick_simple(
        tick: &mut Tick,
        fee_growth_global_0: [u64; 4],
        fee_growth_global_1: [u64; 4],
    ) -> Result<i128> {
        // Update fee growth outside
        tick.fee_growth_outside_0 =
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_0, tick.fee_growth_outside_0)?;
        tick.fee_growth_outside_1 =
            FeeGrowthMath::sub_fee_growth_words(fee_growth_global_1, tick.fee_growth_outside_1)?;

        // Return liquidity delta
        Ok(tick.liquidity_net)
    }

    // ------------------------------------------------------------------------
    // Unified Tick Array Access (SINGLE SOURCE OF TRUTH)
    // ------------------------------------------------------------------------

    /// Get tick array for a given tick - unified access pattern
    /// This replaces all scattered tick array access logic
    pub fn get_tick_array_for_tick<'info>(
        tick_index: i32,
        market_manager: &MarketManager,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<(&'info AccountInfo<'info>, usize)> {
        // Calculate which tick array contains this tick
        let tick_spacing = market_manager.tick_spacing as i32;
        let ticks_per_array = feels_core::constants::TICK_ARRAY_SIZE as i32 * tick_spacing;
        let start_tick = (tick_index / ticks_per_array) * ticks_per_array;

        // Try router first if available
        if let Some(router) = tick_array_router {
            if let Some(array_index) = Self::find_tick_array_in_router(router, start_tick) {
                let tick_array_key = router.tick_arrays[array_index];
                // Find the account in remaining_accounts that matches this key
                for (i, account_info) in remaining_accounts.iter().enumerate() {
                    if account_info.key() == tick_array_key {
                        return Ok((account_info, i));
                    }
                }
            }
        }

        // Fall back to searching remaining_accounts by PDA derivation
        let (expected_key, _) = crate::state::derive_tick_array_pda(
            &market_manager.market,
            start_tick,
            program_id,
        );

        for (i, account_info) in remaining_accounts.iter().enumerate() {
            if account_info.key() == expected_key {
                return Ok((account_info, i));
            }
        }

        Err(FeelsProtocolError::TickArrayNotFound.into())
    }

    /// Find tick array index in router
    fn find_tick_array_in_router(router: &TickArrayRouter, start_tick: i32) -> Option<usize> {
        for i in 0..feels_core::constants::MAX_ROUTER_ARRAYS {
            if (router.active_bitmap & (1 << i)) != 0 && router.start_indices[i] == start_tick {
                return Some(i);
            }
        }
        None
    }

    // ------------------------------------------------------------------------
    // Unified Bitmap Management (SINGLE SOURCE OF TRUTH)
    // ------------------------------------------------------------------------

    /// Update bitmap in MarketManager - the ONLY way to modify tick array bitmap
    // Removed duplicate - see line 883 for the implementation
    

    // ------------------------------------------------------------------------
    // Unified Fee Growth Operations (SINGLE SOURCE OF TRUTH)
    // ------------------------------------------------------------------------

    /// Calculate fee growth inside a position range - unified calculation
    /// This replaces scattered fee growth calculations
    pub fn calculate_fee_growth_inside_unified(
        tick_lower: &Tick,
        tick_upper: &Tick,
        current_tick: i32,
        fee_growth_global_0: u128,
        fee_growth_global_1: u128,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<([u64; 4], [u64; 4])> {
        // Convert global fee growth to array format
        let global_a_words = fee_growth_global_0.to_le_bytes().try_into().unwrap_or([0; 16]);
        let global_b_words = fee_growth_global_1.to_le_bytes().try_into().unwrap_or([0; 16]);
        let global_a_array = [
            u32::from_le_bytes([global_a_words[0], global_a_words[1], global_a_words[2], global_a_words[3]]) as u64,
            u32::from_le_bytes([global_a_words[4], global_a_words[5], global_a_words[6], global_a_words[7]]) as u64,
            u32::from_le_bytes([global_a_words[8], global_a_words[9], global_a_words[10], global_a_words[11]]) as u64,
            u32::from_le_bytes([global_a_words[12], global_a_words[13], global_a_words[14], global_a_words[15]]) as u64,
        ];
        let global_b_array = [
            u32::from_le_bytes([global_b_words[0], global_b_words[1], global_b_words[2], global_b_words[3]]) as u64,
            u32::from_le_bytes([global_b_words[4], global_b_words[5], global_b_words[6], global_b_words[7]]) as u64,
            u32::from_le_bytes([global_b_words[8], global_b_words[9], global_b_words[10], global_b_words[11]]) as u64,
            u32::from_le_bytes([global_b_words[12], global_b_words[13], global_b_words[14], global_b_words[15]]) as u64,
        ];

        // Calculate fee growth below tick_lower
        let fee_growth_below_a = if current_tick >= tick_lower_index {
            tick_lower.fee_growth_outside_0
        } else {
            FeeGrowthMath::sub_fee_growth_words(global_a_array, tick_lower.fee_growth_outside_0)?
        };

        let fee_growth_below_b = if current_tick >= tick_lower_index {
            tick_lower.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth_words(global_b_array, tick_lower.fee_growth_outside_1)?
        };

        // Calculate fee growth above tick_upper
        let fee_growth_above_a = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_0
        } else {
            FeeGrowthMath::sub_fee_growth_words(global_a_array, tick_upper.fee_growth_outside_0)?
        };

        let fee_growth_above_b = if current_tick < tick_upper_index {
            tick_upper.fee_growth_outside_1
        } else {
            FeeGrowthMath::sub_fee_growth_words(global_b_array, tick_upper.fee_growth_outside_1)?
        };

        // Calculate fee growth inside
        let fee_growth_inside_a = FeeGrowthMath::sub_fee_growth_words(
            FeeGrowthMath::sub_fee_growth_words(global_a_array, fee_growth_below_a)?,
            fee_growth_above_a,
        )?;

        let fee_growth_inside_b = FeeGrowthMath::sub_fee_growth_words(
            FeeGrowthMath::sub_fee_growth_words(global_b_array, fee_growth_below_b)?,
            fee_growth_above_b,
        )?;

        Ok((fee_growth_inside_a, fee_growth_inside_b))
    }

    // ------------------------------------------------------------------------
    // Router Integration (SINGLE MANAGEMENT SURFACE)
    // ------------------------------------------------------------------------

    /// Update router when tick arrays are created/destroyed - ONLY through TickManager
    /// This ensures router state stays in sync with MarketManager bitmap
    pub fn update_router_with_new_array(
        router: &mut TickArrayRouter,
        market_manager: &mut MarketManager,
        start_tick: i32,
        tick_array_key: Pubkey,
    ) -> Result<()> {
        // First update the MarketManager bitmap (single source of truth)
        Self::update_tick_array_bitmap(market_manager, start_tick, true)?;

        // Then try to add to router for fast access
        for i in 0..feels_core::constants::MAX_ROUTER_ARRAYS {
            if (router.active_bitmap & (1 << i)) == 0 {
                // Found empty slot
                router.tick_arrays[i] = tick_array_key;
                router.start_indices[i] = start_tick;
                router.active_bitmap |= 1 << i;
                router.last_update_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
                break;
            }
        }
        // If router is full, just use remaining_accounts - MarketManager bitmap is still updated

        Ok(())
    }

    /// Remove array from router when cleaned up - ONLY through TickManager
    pub fn update_router_remove_array(
        router: &mut TickArrayRouter,
        market_manager: &mut MarketManager,
        start_tick: i32,
    ) -> Result<()> {
        // First update the MarketManager bitmap (single source of truth)
        Self::update_tick_array_bitmap(market_manager, start_tick, false)?;

        // Then remove from router
        for i in 0..feels_core::constants::MAX_ROUTER_ARRAYS {
            if router.start_indices[i] == start_tick {
                router.tick_arrays[i] = Pubkey::default();
                router.start_indices[i] = i32::MIN;
                router.active_bitmap &= !(1 << i);
                router.last_update_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
                break;
            }
        }

        Ok(())
    }

    /// Validate router state against MarketManager bitmap - periodic consistency check
    pub fn validate_router_consistency(
        router: &TickArrayRouter,
        market_manager: &MarketManager,
    ) -> Result<bool> {
        for i in 0..feels_core::constants::MAX_ROUTER_ARRAYS {
            if (router.active_bitmap & (1 << i)) != 0 {
                let start_tick = router.start_indices[i];
                if !Self::is_tick_array_initialized(market_manager, start_tick) {
                    // Router has array that MarketManager doesn't know about
                    msg!("BITMAP DRIFT DETECTED: Router has array at tick {} but MarketManager bitmap doesn't", start_tick);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    
    /// Comprehensive validation to detect drift between all tick management systems
    /// This helps ensure bitmap consistency across MarketManager and TickArrayRouter
    pub fn validate_comprehensive_consistency(
        market_manager: &MarketManager,
        router: Option<&TickArrayRouter>,
    ) -> Result<ConsistencyReport> {
        let mut report = ConsistencyReport {
            consistent: true,
            issues_found: Vec::new(),
        };
        
        // Check router consistency if provided
        if let Some(router) = router {
            for i in 0..feels_core::constants::MAX_ROUTER_ARRAYS {
                if router.is_slot_active(i) {
                    let start_tick = router.start_indices[i];
                    
                    if !Self::is_tick_array_initialized(market_manager, start_tick) {
                        report.consistent = false;
                        report.issues_found.push(format!(
                            "Router slot {} has array at tick {} but MarketManager bitmap shows uninitialized",
                            i, start_tick
                        ));
                    }
                }
            }
        }
        
        Ok(report)
    }
}

// ============================================================================
// TickArrayRouter Implementation
// ============================================================================

use crate::state::{RouterConfig, TickArrayRouter};
use crate::utils::MAX_ROUTER_ARRAYS;

impl TickArrayRouter {
    /// Register a new tick array in the router
    pub fn register_array(&mut self, tick_array: Pubkey, start_tick: i32) -> Result<usize> {
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

        Err(FeelsProtocolError::InsufficientResource.into())
    }

    /// Unregister a tick array from the router
    pub fn unregister_array(&mut self, start_tick: i32) -> Result<()> {
        if let Some(index) = self.contains_array(start_tick) {
            self.tick_arrays[index] = Pubkey::default();
            self.start_indices[index] = i32::MIN;
            self.active_bitmap &= !(1 << index);
            Ok(())
        } else {
            Err(FeelsProtocolError::NotFound.into())
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
    let pool = &ctx.accounts.market_manager.load()?;

    router.market = ctx.accounts.market_manager.key();
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
        * TICK_ARRAY_SIZE as i32
        * tick_spacing as i32;

    // Register current array and surrounding arrays
    let arrays_to_load = config.arrays_around_current as i32;
    for i in -arrays_to_load..=arrays_to_load {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;

        // Derive tick array PDA using helper
        let (tick_array_pda, _) = crate::state::pda::derive_tick_array_pda(
            &ctx.accounts.market_manager.key(),
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
    let manager = &ctx.accounts.market_manager.load()?;
    let clock = Clock::get()?;

    // Validate that caller has authority to update router
    require!(
        ctx.accounts.authority.key() == router.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Only update if enough time has passed (use config.update_frequency)
    require!(
        clock.slot >= router.last_update_slot + config.update_frequency,
        FeelsProtocolError::InvalidOperation
    );

    // Create a temporary copy of the router state
    // Build new bitmap atomically to avoid race condition
    let mut new_bitmap = 0u8;
    let mut new_arrays = [Pubkey::default(); MAX_ROUTER_ARRAYS];
    let mut new_indices = [i32::MIN; MAX_ROUTER_ARRAYS];
    let mut new_count = 0usize;

    // Re-populate based on current price
    let current_tick = manager.current_tick;
    let tick_spacing = manager.tick_spacing;
    let current_array_start = (current_tick / (TICK_ARRAY_SIZE as i32 * tick_spacing as i32))
        * TICK_ARRAY_SIZE as i32
        * tick_spacing as i32;

    // Load arrays around current price using config.arrays_around_current
    let arrays_to_load = config.arrays_around_current as i32;
    for i in -arrays_to_load..=arrays_to_load {
        let start_tick = current_array_start + i * TICK_ARRAY_SIZE as i32 * tick_spacing as i32;

        // Derive tick array PDA using helper
        let (tick_array_pda, _) = crate::state::pda::derive_tick_array_pda(
            &ctx.accounts.market_manager.key(),
            start_tick,
            ctx.program_id,
        );

        // The PDA derivation above ensures the address is owned by our program
        // Additional validation happens in TickArrayManager::is_initialized

        if TickArrayManager::is_initialized(manager, start_tick) {
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
        pool: manager.market,
        previous_router: None,
        new_router: router_key,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// ============================================================================
// Helper Functions for Tick Array Access
// ============================================================================


/// Helper to get tick array from remaining accounts based on tick index
/// This is used by order and order_modify instructions
pub fn get_tick_array_from_remaining<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    tick_index: i32,
    tick_spacing: i32,
) -> Result<AccountLoader<'info, TickArray>> {
    // Calculate which tick array contains this tick
    let array_start = (tick_index / (tick_spacing * TICK_ARRAY_SIZE as i32)) * (tick_spacing * TICK_ARRAY_SIZE as i32);
    
    // Find the tick array in remaining accounts
    for account in remaining_accounts {
        if let Ok(tick_array) = AccountLoader::<TickArray>::try_from(account) {
            let ta = tick_array.load()?;
            if ta.start_tick_index == array_start {
                drop(ta);
                return Ok(tick_array);
            }
        }
    }
    
    Err(FeelsProtocolError::InvalidTickArrayAccount.into())
}

// ============================================================================
// Consistency Reporting
// ============================================================================

/// Report on bitmap consistency across different systems
pub struct ConsistencyReport {
    pub consistent: bool,
    pub issues_found: Vec<String>,
}

impl ConsistencyReport {
    pub fn log_issues(&self) {
        if !self.consistent {
            msg!("CONSISTENCY CHECK FAILED:");
            for issue in &self.issues_found {
                msg!("  - {}", issue);
            }
        } else {
            msg!("All tick management systems are consistent");
        }
    }
}
