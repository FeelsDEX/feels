/// Business logic layer for pool operations including fee calculations, validations,
/// and complex tick search algorithms. Handles complex operations like fee configuration,
/// swap fee calculations, and bitmap-based tick array navigation.
/// Delegates fee operations to FeeManager for centralized fee logic.

use anchor_lang::prelude::*;
use crate::state::{Pool, FeelsProtocolError};
use crate::utils::TICK_ARRAY_SIZE;
use crate::utils::FeeBreakdown;

// ============================================================================
// Core Implementation
// ============================================================================

/// Business logic operations for Pool management
impl Pool {
    // ------------------------------------------------------------------------
    // Pool Validation
    // ------------------------------------------------------------------------

    /// Validate pool configuration
    pub fn validate(&self) -> Result<()> {
        // Ensure tick spacing is valid
        require!(
            matches!(self.tick_spacing, 1 | 10 | 60 | 200),
            FeelsProtocolError::InvalidTickSpacing
        );

        // Fee validation is now done through FeeConfig account
        
        Ok(())
    }

    /// Validates that token_b_mint is FeelsSOL
    pub fn validate_feelssol_pair(&self, feelssol_mint: &Pubkey) -> bool {
        self.token_b_mint == *feelssol_mint
    }

    // ------------------------------------------------------------------------
    // Fee Calculations - Must use FeeConfig account
    // ------------------------------------------------------------------------
    
    // Note: All fee calculations have been moved to FeeManager and require
    // the FeeConfig account to be passed. Direct pool fee calculations are
    // no longer supported to ensure consistency.

    // ------------------------------------------------------------------------
    // Fee Management and Updates - Delegates to FeeManager
    // ------------------------------------------------------------------------

    /// Update protocol fees after a swap
    pub fn accumulate_protocol_fees_from_breakdown(
        &mut self,
        fee_breakdown: &FeeBreakdown,
        zero_for_one: bool,
    ) -> Result<()> {
        self.accumulate_protocol_fees(fee_breakdown.protocol_fee, zero_for_one)
    }

    // ------------------------------------------------------------------------
    // Tick Search and Navigation
    // ------------------------------------------------------------------------

    /// Find the next initialized tick array using bitmap-guided search
    /// This is an efficient O(1) operation per word searched
    pub fn find_next_initialized_tick_array(
        &self,
        start_array_index: i32,
        search_direction: bool, // true = search up, false = search down
    ) -> Option<i32> {
        let mut current_word = (start_array_index / 64) as usize;
        let mut bit_pos = (start_array_index % 64) as u8;

        // Ensure we're within valid bounds
        if current_word >= 16 {
            return None;
        }

        loop {
            let word = self.tick_array_bitmap[current_word];

            // Create mask to search only relevant bits
            let mask = if search_direction {
                // Search upward: mask out bits below current position
                if bit_pos >= 64 {
                    0u64
                } else {
                    !((1u64 << bit_pos) - 1)
                }
            } else {
                // Search downward: mask out bits above current position
                if bit_pos >= 64 {
                    u64::MAX
                } else {
                    (1u64 << (bit_pos + 1)) - 1
                }
            };

            let masked_word = word & mask;

            // Check if there are any set bits in the masked word
            if masked_word != 0 {
                let next_bit = if search_direction {
                    // Find the lowest set bit (rightmost)
                    masked_word.trailing_zeros() as u8
                } else {
                    // Find the highest set bit (leftmost)
                    63 - masked_word.leading_zeros() as u8
                };

                // Calculate the array index
                let array_index = (current_word * 64 + next_bit as usize) as i32;

                // Convert to tick index
                let tick_index = array_index * TICK_ARRAY_SIZE as i32;

                return Some(tick_index);
            }

            // Move to next word
            if search_direction {
                if current_word >= 15 {
                    break; // Reached the end
                }
                current_word += 1;
                bit_pos = 0;
            } else {
                if current_word == 0 {
                    break; // Reached the beginning
                }
                current_word -= 1;
                bit_pos = 63;
            }
        }

        None
    }

    /// Check if a specific tick is initialized (logic layer)
    /// Helper method to check individual tick initialization
    pub fn check_tick_initialized(&self, tick: i32) -> bool {
        // Calculate which array and position within array
        let array_index = tick / TICK_ARRAY_SIZE as i32;
        let _tick_offset = (tick % TICK_ARRAY_SIZE as i32) as usize;

        // Check if array is initialized first
        if !self.is_tick_array_initialized(array_index * TICK_ARRAY_SIZE as i32) {
            return false;
        }

        // TODO: For now, assume individual ticks need to be checked from actual TickArray account
        // This would require loading the account, which is beyond scope of this bitmap search
        // In practice, callers should use this for array-level checks and then load specific arrays
        // to check individual tick initialization

        // Return true if array is initialized (conservative check)
        // Real implementation would load TickArray and check ticks[tick_offset].initialized
        true
    }

    /// Find the next initialized tick by searching tick arrays
    /// Returns the tick index and whether it's initialized
    pub fn find_next_initialized_tick(&self, tick: i32, search_direction: bool) -> Option<i32> {
        // Calculate which array contains the starting tick
        let start_array_index = tick / TICK_ARRAY_SIZE as i32;

        // Search for initialized arrays starting from current position
        let mut current_array_index = start_array_index;

        loop {
            // Check if this array is initialized
            let array_start_tick = current_array_index * TICK_ARRAY_SIZE as i32;

            if self.is_tick_array_initialized(array_start_tick) {
                // Search within the tick array for actually initialized ticks
                // Calculate starting position within the array
                let array_end_tick = array_start_tick + TICK_ARRAY_SIZE as i32 - 1;

                if search_direction {
                    // Search forward from the given tick
                    let start_tick_in_array = if tick >= array_start_tick {
                        tick
                    } else {
                        array_start_tick
                    };
                    for i in start_tick_in_array..=array_end_tick {
                        if self.is_tick_initialized(i) {
                            return Some(i);
                        }
                    }
                } else {
                    // Search backward from the given tick
                    let end_tick_in_array = if tick <= array_end_tick {
                        tick
                    } else {
                        array_end_tick
                    };
                    for i in (array_start_tick..=end_tick_in_array).rev() {
                        if self.is_tick_initialized(i) {
                            return Some(i);
                        }
                    }
                }
                // No initialized ticks found in this array, continue to next array
            }

            // Use bitmap search to find next initialized array
            match self.find_next_initialized_tick_array(current_array_index, search_direction) {
                Some(next_array_start_tick) => {
                    current_array_index = next_array_start_tick / TICK_ARRAY_SIZE as i32;
                }
                None => return None, // No more initialized arrays in this direction
            }
        }
    }
}
