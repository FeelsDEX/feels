//! # WorkUnit-Based State Context
//! 
//! This module provides a StateContext that is exclusively built from a WorkUnit,
//! ensuring all state mutations are atomic and go through the WorkUnit pattern.
//! This is the gateway for all business logic to access and modify state.

use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::FeelsProtocolError;
use crate::logic::unit_of_work::WorkUnit;
use std::collections::HashMap;

// ============================================================================
// State Context - Built from WorkUnit
// ============================================================================

/// Unified state context that provides controlled access to WorkUnit state
/// This ensures all mutations go through the WorkUnit pattern
pub struct StateContext<'a, 'info> {
    /// Reference to the work unit containing all state
    work_unit: &'a mut WorkUnit<'info>,
    /// Cached keys for quick access
    market_field_key: Pubkey,
    buffer_key: Pubkey,
    market_manager_key: Pubkey,
    oracle_key: Option<Pubkey>,
}

impl<'a, 'info> StateContext<'a, 'info> {
    /// Create state context from a WorkUnit
    /// The WorkUnit must have already loaded all required accounts
    pub fn from_work_unit(
        work_unit: &'a mut WorkUnit<'info>,
        market_field_key: Pubkey,
        buffer_key: Pubkey,
        market_manager_key: Pubkey,
        oracle_key: Option<Pubkey>,
    ) -> Result<Self> {
        // Verify all required accounts are loaded in the WorkUnit
        work_unit.get_market_field(&market_field_key)?;
        work_unit.get_buffer(&buffer_key)?;
        work_unit.get_market_manager(&market_manager_key)?;
        
        Ok(Self {
            work_unit,
            market_field_key,
            buffer_key,
            market_manager_key,
            oracle_key,
        })
    }
    
    // ========================================================================
    // Market Field Access (Immutable)
    // ========================================================================
    
    /// Get market field parameters (immutable)
    pub fn market_field(&self) -> Result<&MarketField> {
        self.work_unit.get_market_field(&self.market_field_key)
    }
    
    /// Get market scalars
    pub fn market_scalars(&self) -> Result<(u128, u128, u128)> {
        let field = self.market_field()?;
        Ok((field.S, field.T, field.L))
    }
    
    /// Get domain weights
    pub fn domain_weights(&self) -> Result<(u32, u32, u32, u32)> {
        let field = self.market_field()?;
        Ok((field.w_s, field.w_t, field.w_l, field.w_tau))
    }
    
    /// Get fee parameters
    pub fn fee_params(&self) -> Result<(u16, u32)> {
        let field = self.market_field()?;
        Ok((field.base_fee_rate, field.kappa_fee as u32))
    }
    
    // ========================================================================
    // Market Manager Access (Mutable)
    // ========================================================================
    
    /// Get current price
    pub fn current_sqrt_price(&self) -> Result<u128> {
        let manager = self.work_unit.get_market_manager(&self.market_manager_key)?;
        Ok(manager.sqrt_price)
    }
    
    /// Get current tick
    pub fn current_tick(&self) -> Result<i32> {
        let manager = self.work_unit.get_market_manager(&self.market_manager_key)?;
        Ok(manager.current_tick)
    }
    
    /// Get current liquidity
    pub fn current_liquidity(&self) -> Result<u128> {
        let manager = self.work_unit.get_market_manager(&self.market_manager_key)?;
        Ok(manager.liquidity)
    }
    
    /// Update price and tick
    pub fn update_price(&mut self, sqrt_price: u128, tick: i32) -> Result<()> {
        let manager = self.work_unit.get_market_manager_mut(&self.market_manager_key)?;
        manager.sqrt_price = sqrt_price;
        manager.current_tick = tick;
        Ok(())
    }
    
    /// Update liquidity
    pub fn update_liquidity(&mut self, liquidity: u128) -> Result<()> {
        let manager = self.work_unit.get_market_manager_mut(&self.market_manager_key)?;
        manager.liquidity = liquidity;
        Ok(())
    }
    
    /// Update fee growth
    pub fn update_fee_growth(&mut self, token_0: bool, fee_growth: u128) -> Result<()> {
        let manager = self.work_unit.get_market_manager_mut(&self.market_manager_key)?;
        if token_0 {
            manager.fee_growth_global_0 = fee_growth;
        } else {
            manager.fee_growth_global_1 = fee_growth;
        }
        Ok(())
    }
    
    /// Record volume
    pub fn record_volume(&mut self, amount_0: u64, amount_1: u64) -> Result<()> {
        let manager = self.work_unit.get_market_manager_mut(&self.market_manager_key)?;
        manager.total_volume_0 = manager.total_volume_0
            .checked_add(amount_0 as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        manager.total_volume_1 = manager.total_volume_1
            .checked_add(amount_1 as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        manager.total_volume_usd = manager.total_volume_usd
            .checked_add((amount_0 + amount_1) as u128) // Simplified
            .ok_or(FeelsProtocolError::MathOverflow)?;
        Ok(())
    }
    
    // ========================================================================
    // Buffer Account Access (Mutable)
    // ========================================================================
    
    /// Get available rebate capacity
    pub fn available_rebate(&self) -> Result<u64> {
        let buffer = self.work_unit.get_buffer(&self.buffer_key)?;
        Ok(buffer.available_rebate())
    }
    
    /// Collect fees into buffer
    pub fn collect_fee(&mut self, amount: u64, token_index: u8, current_time: i64) -> Result<()> {
        let buffer = self.work_unit.get_buffer_mut(&self.buffer_key)?;
        crate::state::fees::collect_fee(buffer, amount, token_index, current_time)
    }
    
    /// Pay rebate from buffer
    pub fn pay_rebate(&mut self, amount: u64, current_time: i64) -> Result<()> {
        let buffer = self.work_unit.get_buffer_mut(&self.buffer_key)?;
        crate::state::fees::pay_rebate(buffer, amount, current_time)
    }
    
    /// Update buffer participation
    pub fn update_buffer_participation(
        &mut self,
        token_0_amount: u64,
        token_1_amount: u64,
        participation_factor: u32,
    ) -> Result<()> {
        let buffer = self.work_unit.get_buffer_mut(&self.buffer_key)?;
        buffer.accumulated_fees_0 += token_0_amount;
        buffer.accumulated_fees_1 += token_1_amount;
        // Apply participation factor
        let tau_contribution_0 = (token_0_amount as u128 * participation_factor as u128) / 10000;
        let tau_contribution_1 = (token_1_amount as u128 * participation_factor as u128) / 10000;
        buffer.tau_contribution_0 += tau_contribution_0 as u64;
        buffer.tau_contribution_1 += tau_contribution_1 as u64;
        Ok(())
    }
    
    // ========================================================================
    // Oracle Access (Read-only)
    // ========================================================================
    
    /// Get oracle TWAP price
    pub fn get_oracle_twap(&self) -> Result<u128> {
        if let Some(oracle_key) = self.oracle_key {
            let oracle = self.work_unit.get_twap_oracle(&oracle_key)?;
            Ok(oracle.get_safe_twap_a())
        } else {
            Ok(0) // No oracle available
        }
    }
    
    /// Get oracle confidence
    pub fn get_oracle_confidence(&self) -> Result<u64> {
        if let Some(oracle_key) = self.oracle_key {
            let oracle = self.work_unit.get_twap_oracle(&oracle_key)?;
            Ok(oracle.token_a_confidence)
        } else {
            Ok(u64::MAX) // No confidence if no oracle
        }
    }
    
    // ========================================================================
    // Tick Array Access (Mutable)
    // ========================================================================
    
    /// Load tick array for a specific tick
    pub fn load_tick_array(&mut self, tick: i32, tick_array_key: Pubkey) -> Result<()> {
        // This would be called to ensure a tick array is loaded before use
        // The actual loading happens through WorkUnit
        Ok(())
    }
    
    /// Update tick
    pub fn update_tick(
        &mut self,
        tick: i32,
        liquidity_delta: i128,
        fee_growth_0: u128,
        fee_growth_1: u128,
        tick_array_key: Pubkey,
    ) -> Result<i128> {
        let tick_array = self.work_unit.get_tick_array_mut(&tick_array_key)?;
        let array_start_index = tick_array.start_tick_index;
        let tick_index = ((tick - array_start_index) / TICK_SPACING as i32) as usize;
        
        require!(
            tick_index < TICK_ARRAY_SIZE,
            FeelsProtocolError::InvalidTickIndex
        );
        
        let tick_data = &mut tick_array.ticks[tick_index];
        let liquidity_before = tick_data.liquidity_net;
        
        // Update liquidity
        tick_data.liquidity_net = tick_data.liquidity_net.saturating_add(liquidity_delta);
        
        if liquidity_delta > 0 {
            tick_data.liquidity_gross = tick_data.liquidity_gross
                .checked_add(liquidity_delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            tick_data.liquidity_gross = tick_data.liquidity_gross
                .checked_sub((-liquidity_delta) as u128)
                .ok_or(FeelsProtocolError::MathUnderflow)?;
        }
        
        // Initialize if needed
        if !tick_data.initialized && tick_data.liquidity_gross > 0 {
            tick_data.initialized = true;
            tick_data.fee_growth_outside_0 = fee_growth_0;
            tick_data.fee_growth_outside_1 = fee_growth_1;
        }
        
        Ok(liquidity_before)
    }
    
    /// Cross tick
    pub fn cross_tick(
        &mut self,
        tick: i32,
        fee_growth_global_0: u128,
        fee_growth_global_1: u128,
        tick_array_key: Pubkey,
    ) -> Result<i128> {
        let tick_array = self.work_unit.get_tick_array_mut(&tick_array_key)?;
        let array_start_index = tick_array.start_tick_index;
        let tick_index = ((tick - array_start_index) / TICK_SPACING as i32) as usize;
        
        require!(
            tick_index < TICK_ARRAY_SIZE,
            FeelsProtocolError::InvalidTickIndex
        );
        
        let tick_data = &mut tick_array.ticks[tick_index];
        
        // Flip fee growth
        tick_data.fee_growth_outside_0 = fee_growth_global_0
            .wrapping_sub(tick_data.fee_growth_outside_0);
        tick_data.fee_growth_outside_1 = fee_growth_global_1
            .wrapping_sub(tick_data.fee_growth_outside_1);
        
        Ok(tick_data.liquidity_net)
    }
    
    // ========================================================================
    // Position Access (Mutable)
    // ========================================================================
    
    /// Update position fees
    pub fn update_position_fees(
        &mut self,
        position_key: Pubkey,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> Result<(u64, u64)> {
        let position = self.work_unit.get_position_mut(&position_key)?;
        
        // Calculate fees owed
        let fee_growth_delta_0 = fee_growth_inside_0.wrapping_sub(position.fee_growth_inside_0_last);
        let fee_growth_delta_1 = fee_growth_inside_1.wrapping_sub(position.fee_growth_inside_1_last);
        
        let fees_0 = ((position.liquidity as u128 * fee_growth_delta_0) >> 64) as u64;
        let fees_1 = ((position.liquidity as u128 * fee_growth_delta_1) >> 64) as u64;
        
        position.tokens_owed_0 += fees_0;
        position.tokens_owed_1 += fees_1;
        position.fee_growth_inside_0_last = fee_growth_inside_0;
        position.fee_growth_inside_1_last = fee_growth_inside_1;
        
        Ok((fees_0, fees_1))
    }
    
    /// Update position liquidity
    pub fn update_position_liquidity(
        &mut self,
        position_key: Pubkey,
        liquidity_delta: i128,
    ) -> Result<()> {
        let position = self.work_unit.get_position_mut(&position_key)?;
        
        if liquidity_delta > 0 {
            position.liquidity = position.liquidity
                .checked_add(liquidity_delta as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            position.liquidity = position.liquidity
                .checked_sub((-liquidity_delta) as u128)
                .ok_or(FeelsProtocolError::MathUnderflow)?;
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Convenience Methods
    // ========================================================================
    
    /// Get all market state for calculations
    pub fn get_market_state(&self) -> Result<MarketStateSnapshot> {
        let field = self.market_field()?;
        let manager = self.work_unit.get_market_manager(&self.market_manager_key)?;
        let buffer = self.work_unit.get_buffer(&self.buffer_key)?;
        
        Ok(MarketStateSnapshot {
            // Field parameters
            S: field.S,
            T: field.T,
            L: field.L,
            w_s: field.w_s,
            w_t: field.w_t,
            w_l: field.w_l,
            w_tau: field.w_tau,
            base_fee_rate: field.base_fee_rate,
            kappa_fee: field.kappa_fee,
            
            // Manager state
            sqrt_price: manager.sqrt_price,
            current_tick: manager.current_tick,
            liquidity: manager.liquidity,
            fee_growth_global_0: manager.fee_growth_global_0,
            fee_growth_global_1: manager.fee_growth_global_1,
            
            // Buffer state
            available_rebate: buffer.available_rebate(),
        })
    }
    
    /// Check if market is paused
    pub fn is_market_paused(&self) -> Result<bool> {
        let field = self.market_field()?;
        Ok(field.is_paused())
    }
    
    /// Check if fallback mode is active
    pub fn is_fallback_mode(&self) -> Result<bool> {
        let field = self.market_field()?;
        Ok(field.is_fallback())
    }
}

// ============================================================================
// Market State Snapshot
// ============================================================================

/// Snapshot of market state for calculations
#[derive(Debug, Clone)]
pub struct MarketStateSnapshot {
    // Field parameters
    pub S: u128,
    pub T: u128,
    pub L: u128,
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    pub base_fee_rate: u16,
    pub kappa_fee: u32,
    
    // Manager state
    pub sqrt_price: u128,
    pub current_tick: i32,
    pub liquidity: u128,
    pub fee_growth_global_0: u128,
    pub fee_growth_global_1: u128,
    
    // Buffer state
    pub available_rebate: u64,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a StateContext from accounts by first loading them into a WorkUnit
pub fn create_state_context<'a, 'info>(
    work_unit: &'a mut WorkUnit<'info>,
    market_field: &'info Account<'info, MarketField>,
    buffer_account: &'info Account<'info, BufferAccount>,
    market_manager: &'info AccountLoader<'info, MarketManager>,
    oracle: Option<&'info AccountLoader<'info, UnifiedOracle>>,
) -> Result<StateContext<'a, 'info>> {
    // Load all accounts into the WorkUnit
    work_unit.load_market_field(market_field)?;
    work_unit.load_buffer(buffer_account)?;
    work_unit.load_market_manager(market_manager)?;
    
    let oracle_key = if let Some(oracle_loader) = oracle {
        work_unit.load_twap_oracle(oracle_loader)?;
        Some(oracle_loader.key())
    } else {
        None
    };
    
    // Create StateContext from the loaded WorkUnit
    StateContext::from_work_unit(
        work_unit,
        market_field.key(),
        buffer_account.key(),
        market_manager.key(),
        oracle_key,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_context_creation() {
        // Would require mock accounts and WorkUnit
    }
}