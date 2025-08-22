/// Handles tick-level operations for concentrated liquidity including initialization,
/// updates, and liquidity crossing logic. Manages fee growth tracking outside ticks
/// and net liquidity changes. Critical for efficient price discovery and liquidity
/// utilization as swaps move through different price ranges.

use anchor_lang::prelude::*;
use crate::state::{TickArray, Tick, PoolError, TICK_ARRAY_SIZE};
use crate::utils::{SafeMath, LiquiditySafeMath, FeeGrowthMath};

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
        // V2.1 Fix: Validate liquidity_delta magnitude is reasonable
        // Prevent DoS through extreme values that could cause overflow
        const MAX_LIQUIDITY_DELTA: i128 = i128::MAX / 2; // Half of max to leave room for operations
        require!(
            liquidity_delta.abs() <= MAX_LIQUIDITY_DELTA,
            PoolError::InvalidLiquidityAmount
        );
        
        let array_index = self.tick_index_to_array_index(tick_index)?;
        let tick = &mut self.ticks[array_index];

        require!(tick.initialized == 1, PoolError::TickNotInitialized);

        let liquidity_gross_before = tick.liquidity_gross;

        // Update gross liquidity using safe arithmetic
        tick.liquidity_gross = tick.liquidity_gross.safe_add_liquidity(liquidity_delta)?;

        // Update net liquidity using safe arithmetic
        if upper {
            tick.liquidity_net = tick.liquidity_net.safe_sub(liquidity_delta)?;
        } else {
            tick.liquidity_net = tick.liquidity_net.safe_add(liquidity_delta)?;
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
}

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