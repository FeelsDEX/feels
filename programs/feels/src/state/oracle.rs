//! Oracle state account for TWAP price tracking

use crate::error::FeelsError;
use anchor_lang::prelude::*;

/// Maximum number of observations to store
/// Reduced from 65 to 12 to help with stack size issues
pub const MAX_OBSERVATIONS: usize = 12;

/// Single price observation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct Observation {
    /// Block timestamp of this observation
    pub block_timestamp: i64, // 8 bytes
    /// Cumulative tick value (tick * time)
    pub tick_cumulative: i128, // 16 bytes
    /// Whether this observation has been initialized
    pub initialized: bool,
    /// Padding for alignment
    pub _padding: [u8; 7],
}

/// Oracle state account that stores price observations
#[account]
pub struct OracleState {
    /// Pool ID this oracle belongs to
    pub pool_id: Pubkey, // 32 bytes
    /// Index of the most recent observation
    pub observation_index: u16, // 2 bytes
    /// Current number of observations (grows from 1 to MAX)
    pub observation_cardinality: u16, // 2 bytes
    /// Next observation cardinality (for future expansion)
    pub observation_cardinality_next: u16, // 2 bytes
    /// Bump seed for the oracle PDA
    pub oracle_bump: u8,
    /// Array of observations
    pub observations: [Observation; MAX_OBSERVATIONS],
    /// Reserved for future use
    pub _reserved: [u8; 4],
}

impl OracleState {
    /// Size of the oracle account
    pub const LEN: usize = 8 + // discriminator
        32 + // pool_id
        2 + // observation_index
        2 + // observation_cardinality
        2 + // observation_cardinality_next
        1 + // oracle_bump
        (32 * MAX_OBSERVATIONS) + // observations (8+16+1+7 = 32 bytes each)
        4 + // _reserved
        5; // padding added by Rust compiler for alignment
}

impl Default for OracleState {
    fn default() -> Self {
        Self {
            pool_id: Pubkey::default(),
            observation_index: 0,
            observation_cardinality: 0,
            observation_cardinality_next: 0,
            oracle_bump: 0,
            observations: [Observation::default(); MAX_OBSERVATIONS],
            _reserved: [0; 4],
        }
    }
}

// Methods for zero-copy oracle operations
impl OracleState {
    /// Initialize a new oracle (called through AccountLoader)
    pub fn initialize(
        &mut self,
        pool_id: Pubkey,
        oracle_bump: u8,
        _current_tick: i32,
        current_timestamp: i64,
    ) -> Result<()> {
        self.pool_id = pool_id;
        self.oracle_bump = oracle_bump;
        self.observation_index = 0;
        self.observation_cardinality = 1;
        self.observation_cardinality_next = 1;

        // Initialize first observation
        self.observations[0] = Observation {
            block_timestamp: current_timestamp,
            tick_cumulative: 0,
            initialized: true,
            _padding: [0; 7],
        };

        Ok(())
    }

    /// Update the oracle with a new observation
    pub fn update(&mut self, tick: i32, block_timestamp: i64) -> Result<()> {
        let last_observation = &self.observations[self.observation_index as usize];

        // Only update if time has passed
        if block_timestamp > last_observation.block_timestamp {
            let time_delta = block_timestamp
                .checked_sub(last_observation.block_timestamp)
                .ok_or(FeelsError::MathOverflow)?;

            let tick_cumulative = last_observation
                .tick_cumulative
                .checked_add(
                    (tick as i128)
                        .checked_mul(time_delta as i128)
                        .ok_or(FeelsError::MathOverflow)?,
                )
                .ok_or(FeelsError::MathOverflow)?;

            // Move to next observation slot
            // Ensure observation_cardinality is 1 or greater to avoid division by zero
            let cardinality = self.observation_cardinality.max(1);
            self.observation_index = (self.observation_index + 1) % cardinality;

            // Write new observation
            self.observations[self.observation_index as usize] = Observation {
                block_timestamp,
                tick_cumulative,
                initialized: true,
                _padding: [0; 7],
            };

            // Expand cardinality if needed and not at max
            if self.observation_cardinality < MAX_OBSERVATIONS as u16 && self.observation_index == 0
            {
                self.observation_cardinality += 1;
            }
        }

        Ok(())
    }

    /// Get two observations for TWAP calculation
    pub fn get_observations(
        &self,
        current_timestamp: i64,
        seconds_ago: u32,
    ) -> Result<(Observation, Observation)> {
        let target_timestamp = current_timestamp
            .checked_sub(seconds_ago as i64)
            .ok_or(FeelsError::MathOverflow)?;

        // Find the observation closest to target timestamp
        let mut old_observation_index = 0;
        let mut found = false;

        // Search through initialized observations
        for i in 0..self.observation_cardinality as usize {
            let obs = &self.observations[i];
            if obs.initialized
                && obs.block_timestamp <= target_timestamp
                && (!found
                    || obs.block_timestamp
                        > self.observations[old_observation_index].block_timestamp)
            {
                old_observation_index = i;
                found = true;
            }
        }

        require!(found, FeelsError::OracleInsufficientData);

        let old_observation = self.observations[old_observation_index];
        let new_observation = self.observations[self.observation_index as usize];

        Ok((old_observation, new_observation))
    }

    /// Calculate TWAP tick over a given period
    pub fn get_twap_tick(&self, current_timestamp: i64, seconds_ago: u32) -> Result<i32> {
        // SECURITY: Enforce minimum TWAP duration to prevent manipulation
        // Increased from 15 to 60 seconds to make timestamp manipulation less impactful
        const MIN_TWAP_DURATION: u32 = 60; // 60 seconds minimum
        let effective_seconds_ago = seconds_ago.max(MIN_TWAP_DURATION);

        let (old_observation, new_observation) =
            self.get_observations(current_timestamp, effective_seconds_ago)?;

        let time_delta = new_observation
            .block_timestamp
            .checked_sub(old_observation.block_timestamp)
            .ok_or(FeelsError::MathOverflow)?;

        require!(time_delta > 0, FeelsError::InvalidTimestamp);

        // Additional check to ensure we have a meaningful time period
        // This prevents extremely short TWAPs that could be manipulated
        require!(
            time_delta >= MIN_TWAP_DURATION as i64,
            FeelsError::InsufficientTWAPDuration
        );

        let tick_delta = new_observation
            .tick_cumulative
            .checked_sub(old_observation.tick_cumulative)
            .ok_or(FeelsError::MathOverflow)?;

        let avg_tick = (tick_delta / time_delta as i128) as i32;

        Ok(avg_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_size() {
        assert_eq!(std::mem::size_of::<Observation>(), 32);
        // OracleState size calculation: discriminator + pool_id + observation_index + 
        // observation_cardinality + observation_cardinality_next + oracle_bump + observations + _reserved + padding
        assert_eq!(OracleState::LEN, 8 + 32 + 2 + 2 + 2 + 1 + (32 * 12) + 4 + 5);
        // Verify against actual struct size (excluding discriminator)
        assert_eq!(std::mem::size_of::<OracleState>(), OracleState::LEN - 8);
    }
}
