/// Business logic layer for pool operations including fee calculations, validations,
/// and complex tick search algorithms. Handles complex operations like fee configuration,
/// swap fee calculations, and bitmap-based tick array navigation.

use anchor_lang::prelude::*;
use crate::state::{Pool, PoolError};
use crate::utils::TICK_ARRAY_SIZE;
use crate::utils::{FeeMath, FeeBreakdown, FeeConfig};

// ============================================================================
// Core Implementation
// ============================================================================

/// Business logic operations for Pool management
impl Pool {

    // ------------------------------------------------------------------------
    // Pool Validation
    // ------------------------------------------------------------------------

    /// Validate that this pool conforms to Phase 1 requirements
    pub fn validate_phase1(&self) -> Result<()> {
        // Ensure we have a valid version
        require!(self.version == 1, PoolError::InvalidVersion);

        // Ensure tick spacing is valid
        require!(
            matches!(self.tick_spacing, 1 | 10 | 60 | 200),
            PoolError::InvalidTickSpacing
        );

        // Ensure fee rate is valid
        require!(
            matches!(self.fee_rate, 1 | 5 | 30 | 100),
            PoolError::InvalidFeeRate
        );

        Ok(())
    }

    /// Validates that token_b_mint is FeelsSOL
    pub fn validate_feelssol_pair(&self, feelssol_mint: &Pubkey) -> bool {
        self.token_b_mint == *feelssol_mint
    }

    // ------------------------------------------------------------------------
    // Fee Calculations
    // ------------------------------------------------------------------------

    /// Calculate complete fee breakdown for a swap amount
    /// This is the single source of truth for all fee calculations in the pool
    pub fn calculate_swap_fees(&self, amount_in: u64) -> Result<FeeBreakdown> {
        FeeMath::calculate_swap_fees(amount_in, self.fee_rate, self.protocol_fee_rate)
    }

    /// Calculate just the total fee amount (used in swap calculations)
    pub fn calculate_total_fee(&self, amount_in: u64) -> Result<u64> {
        FeeMath::calculate_total_fee(amount_in, self.fee_rate)
    }

    /// Validate that this pool's fee configuration is consistent and valid
    pub fn validate_fee_configuration(&self) -> Result<()> {
        FeeConfig::validate_pool_fees(self.fee_rate, self.protocol_fee_rate, self.tick_spacing)
    }

    // ------------------------------------------------------------------------
    // Fee Management and Updates
    // ------------------------------------------------------------------------

    /// Update protocol fees after a swap (delegates to state method)
    pub fn accumulate_protocol_fees_from_breakdown(&mut self, fee_breakdown: &FeeBreakdown, zero_for_one: bool) -> Result<()> {
        self.accumulate_protocol_fees(fee_breakdown.protocol_fee, zero_for_one)
    }

    /// Get the amount after deducting fees
    pub fn calculate_amount_after_fee(&self, amount_in: u64) -> Result<u64> {
        let total_fee = self.calculate_total_fee(amount_in)?;
        amount_in.checked_sub(total_fee).ok_or(PoolError::ArithmeticUnderflow.into())
    }

    /// Initialize fee configuration for a new pool
    pub fn initialize_fees(&mut self, fee_rate: u16) -> Result<()> {
        let (validated_fee_rate, protocol_fee_rate, tick_spacing) = FeeConfig::create_for_pool(fee_rate)?;
        
        self.fee_rate = validated_fee_rate;
        self.protocol_fee_rate = protocol_fee_rate;
        self.tick_spacing = tick_spacing;
        
        Ok(())
    }

    /// Get effective fee rate (for future dynamic fee implementation)
    pub fn get_effective_fee_rate(&self) -> Result<u16> {
        // Phase 1: Return base fee rate
        // Phase 2+: Implement dynamic adjustments based on volume/volatility
        FeeMath::calculate_effective_fee_rate(self.fee_rate, self.total_volume_0, 0)
    }

    // ------------------------------------------------------------------------
    // Global Fee Growth
    // ------------------------------------------------------------------------

    /// Update global fee growth (delegates to state method)
    pub fn update_fee_growth_from_fees(&mut self, fee_amount: u64, is_token_a: bool) -> Result<()> {
        self.update_fee_growth(fee_amount, is_token_a)
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

    /// Check if a specific tick is initialized
    /// Helper method to check individual tick initialization
    pub fn is_tick_initialized(&self, tick: i32) -> bool {
        // Calculate which array and position within array
        let array_index = tick / TICK_ARRAY_SIZE as i32;
        let _tick_offset = (tick % TICK_ARRAY_SIZE as i32) as usize;
        
        // Check if array is initialized first
        if !self.is_tick_array_initialized(array_index * TICK_ARRAY_SIZE as i32) {
            return false;
        }
        
        // For now, assume individual ticks need to be checked from actual TickArray account
        // This would require loading the account, which is beyond scope of this bitmap search
        // In practice, callers should use this for array-level checks and then load specific arrays
        // to check individual tick initialization
        
        // Return true if array is initialized (conservative check)
        // Real implementation would load TickArray and check ticks[tick_offset].initialized
        true
    }

    /// Find the next initialized tick by searching tick arrays
    /// Returns the tick index and whether it's initialized
    pub fn find_next_initialized_tick(
        &self,
        tick: i32,
        search_direction: bool,
    ) -> Option<i32> {
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
                    let start_tick_in_array = if tick >= array_start_tick { tick } else { array_start_tick };
                    for i in start_tick_in_array..=array_end_tick {
                        if self.is_tick_initialized(i) {
                            return Some(i);
                        }
                    }
                } else {
                    // Search backward from the given tick
                    let end_tick_in_array = if tick <= array_end_tick { tick } else { array_end_tick };
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