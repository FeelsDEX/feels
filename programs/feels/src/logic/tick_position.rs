/// Manages liquidity position lifecycle including validation, fee tracking, and updates.
/// Handles position-specific calculations like fee accumulation within price ranges
/// and ensures positions remain valid within pool constraints. Provides business logic
/// for position NFT metadata management and ownership tracking.

use anchor_lang::prelude::*;
use crate::state::{TickPositionMetadata, PoolError};
use crate::utils::{add_liquidity_delta, MIN_TICK, MAX_TICK};

// ============================================================================
// Core Implementation
// ============================================================================

/// Business logic operations for Position management
impl TickPositionMetadata {
    /// Calculate the seeds for position metadata PDA derivation
    pub fn seeds(tick_position_mint: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"position".to_vec(),
            tick_position_mint.to_bytes().to_vec(),
        ]
    }

    /// Validate the position parameters
    pub fn validate(&self, tick_spacing: i16) -> Result<()> {
        // Ensure tick range is valid
        require!(
            self.tick_lower < self.tick_upper,
            PoolError::InvalidTickRange
        );

        // Ensure ticks are properly aligned to tick spacing
        require!(
            self.tick_lower % tick_spacing as i32 == 0,
            PoolError::TickNotAligned
        );
        require!(
            self.tick_upper % tick_spacing as i32 == 0,
            PoolError::TickNotAligned
        );

        // Ensure tick range is within bounds
        require!(
            self.tick_lower >= MIN_TICK && self.tick_upper <= MAX_TICK,
            PoolError::TickOutOfBounds
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
    pub fn set_fee_growth_inside_last_0(&mut self, value: [u64; 4]) {
        self.fee_growth_inside_last_0 = value;
    }

    /// Set fee growth inside last values for token 1
    pub fn set_fee_growth_inside_last_1(&mut self, value: [u64; 4]) {
        self.fee_growth_inside_last_1 = value;
    }

    /// Get fee growth inside last as u256 for token 0
    pub fn get_fee_growth_inside_last_0(&self) -> [u64; 4] {
        self.fee_growth_inside_last_0
    }

    /// Get fee growth inside last as u256 for token 1
    pub fn get_fee_growth_inside_last_1(&self) -> [u64; 4] {
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
    /// This would use the complex fee math from Uniswap V3
    pub fn calculate_fees_owed(
        &self,
        _fee_growth_inside_0: [u64; 4],
        _fee_growth_inside_1: [u64; 4],
    ) -> (u64, u64) {
        // Simplified fee calculation - in a real implementation this would
        // use proper u256 arithmetic for fee growth calculations
        
        // fees_owed = liquidity * (fee_growth_inside - fee_growth_inside_last) / 2^128
        // For now, return current tokens owed (simplified)
        (self.tokens_owed_0, self.tokens_owed_1)
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