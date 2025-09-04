//! # WorkUnit-Based Order Manager
//! 
//! This module provides the unified OrderManager that handles all order types
//! using the WorkUnit pattern exclusively. All state mutations go through the
//! StateContext built from a WorkUnit, ensuring atomic operations.
//! 
//! ## Design Principles
//! 
//! 1. **WorkUnit Gateway**: All state access through StateContext from WorkUnit
//! 2. **Atomic Operations**: Complete success or complete rollback
//! 3. **Physics Integration**: Thermodynamic calculations for all operations
//! 4. **Hub-and-Spoke Routing**: All routes go through FeelsSOL

use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::FeelsProtocolError;
use crate::logic::{
    state_context::StateContext,
    unit_of_work::WorkUnit,
    thermodynamics::{self, ThermodynamicFeeParams},
    calculate_path_work, PathSegment, WorkResult,
    concentrated_liquidity::ConcentratedLiquidityMath,
    tick::TickManager,
};
use feels_core::constants::*;
use feels_core::types::{Position3D, TradeDimension};

// ============================================================================
// Order Types
// ============================================================================

/// Order types supported by the unified system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
    EnterPosition,
    ExitPosition,
}

/// Result of any order operation
#[derive(Debug, Default)]
pub struct OrderResult {
    /// Order type executed
    pub order_type: OrderType,
    /// Primary amount (in/deposited)
    pub amount_primary: u64,
    /// Secondary amount (out/withdrawn)
    pub amount_secondary: u64,
    /// Fee charged
    pub fee_amount: u64,
    /// Rebate paid (if any)
    pub rebate_amount: u64,
    /// Work performed (thermodynamic)
    pub work: i128,
    /// Final price after execution
    pub final_price: u128,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Swap
    }
}

/// Hub-and-spoke route configuration
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct HubRoute {
    /// Pool keys in order of execution (max 2)
    pub pools: Vec<Pubkey>,
    /// Direction for each pool (true = token0->token1)
    pub zero_for_one: Vec<bool>,
}

impl HubRoute {
    pub const MAX_HOPS: usize = 2;
    
    pub fn validate(&self) -> Result<()> {
        require!(
            self.pools.len() <= Self::MAX_HOPS,
            FeelsProtocolError::InvalidRoute
        );
        require!(
            self.pools.len() == self.zero_for_one.len(),
            FeelsProtocolError::InvalidRoute
        );
        Ok(())
    }
}

// ============================================================================
// WorkUnit-Based Order Manager
// ============================================================================

/// Order manager that exclusively uses WorkUnit for state access
pub struct OrderManager<'a, 'info> {
    /// State context built from WorkUnit
    state: StateContext<'a, 'info>,
    /// Current timestamp
    current_time: i64,
}

impl<'a, 'info> OrderManager<'a, 'info> {
    /// Create new order manager from a StateContext
    pub fn new(
        state: StateContext<'a, 'info>,
        current_time: i64,
    ) -> Self {
        Self {
            state,
            current_time,
        }
    }
    
    // ========================================================================
    // Swap Operations
    // ========================================================================
    
    /// Execute a swap through hub-and-spoke routing
    pub fn execute_swap(
        &mut self,
        route: HubRoute,
        amount: u64,
        other_amount_threshold: u64,
        exact_input: bool,
    ) -> Result<OrderResult> {
        // Validate route
        route.validate()?;
        
        // Check market status
        require!(
            !self.state.is_market_paused()?,
            FeelsProtocolError::MarketPaused
        );
        
        let mut current_amount = amount;
        let mut total_work = 0i128;
        let mut total_fees = 0u64;
        let mut total_rebates = 0u64;
        
        // Execute each hop
        for (i, (pool, zero_for_one)) in route.pools.iter().zip(route.zero_for_one.iter()).enumerate() {
            let hop_result = self.execute_swap_hop(
                *pool,
                current_amount,
                *zero_for_one,
                exact_input,
                i == route.pools.len() - 1, // is_final_hop
            )?;
            
            current_amount = hop_result.amount_secondary;
            total_work = total_work.saturating_add(hop_result.work);
            total_fees = total_fees.saturating_add(hop_result.fee_amount);
            total_rebates = total_rebates.saturating_add(hop_result.rebate_amount);
        }
        
        // Validate output
        if exact_input {
            require!(
                current_amount >= other_amount_threshold,
                FeelsProtocolError::SlippageExceeded
            );
        } else {
            require!(
                current_amount <= other_amount_threshold,
                FeelsProtocolError::SlippageExceeded
            );
        }
        
        Ok(OrderResult {
            order_type: OrderType::Swap,
            amount_primary: amount,
            amount_secondary: current_amount,
            fee_amount: total_fees,
            rebate_amount: total_rebates,
            work: total_work,
            final_price: self.state.current_sqrt_price()?,
        })
    }
    
    /// Execute a single swap hop
    fn execute_swap_hop(
        &mut self,
        pool: Pubkey,
        amount_in: u64,
        zero_for_one: bool,
        exact_input: bool,
        is_final_hop: bool,
    ) -> Result<OrderResult> {
        // Get current market state
        let sqrt_price_before = self.state.current_sqrt_price()?;
        let liquidity_before = self.state.current_liquidity()?;
        
        // Calculate swap using concentrated liquidity math
        let (amount_0, amount_1, sqrt_price_after) = self.calculate_swap_amounts(
            amount_in,
            sqrt_price_before,
            liquidity_before,
            zero_for_one,
            exact_input,
        )?;
        
        // Calculate work performed
        let work_segment = PathSegment {
            start: Position3D { 
                S: sqrt_price_before, 
                T: 0, 
                L: 0 
            },
            end: Position3D { 
                S: sqrt_price_after, 
                T: 0, 
                L: 0 
            },
            liquidity: liquidity_before,
            distance: amount_in as u128,
            dimension: TradeDimension::Spot,
        };
        
        let market_snapshot = self.state.get_market_state()?;
        let work_result = calculate_path_work(&[work_segment], &market_snapshot)?;
        
        // Calculate fees
        let fee_params = ThermodynamicFeeParams {
            work: work_result.total_work as i128,
            amount_in,
            execution_price: sqrt_price_after,
            oracle_price: self.state.get_oracle_twap()?,
            base_fee_bps: market_snapshot.base_fee_rate,
            kappa: market_snapshot.kappa_fee as u32,
            max_rebate_bps: if work_result.total_work < 0 { 100 } else { 0 }, // 1% max rebate
            is_buy: !zero_for_one,
            buffer: None, // Buffer accessed through state context
        };
        
        let fee_result = thermodynamics::calculate_thermodynamic_fee(fee_params)?;
        
        // Update state through context
        self.state.update_price(sqrt_price_after, 
            crate::utils::math::get_tick_at_sqrt_price(sqrt_price_after)?)?;
        
        // Update fee growth
        let fee_growth_delta = ((fee_result.fee_amount as u128) << 64) / liquidity_before;
        if zero_for_one {
            let current_fee_growth = market_snapshot.fee_growth_global_0;
            self.state.update_fee_growth(true, current_fee_growth + fee_growth_delta)?;
        } else {
            let current_fee_growth = market_snapshot.fee_growth_global_1;
            self.state.update_fee_growth(false, current_fee_growth + fee_growth_delta)?;
        }
        
        // Record volume
        if zero_for_one {
            self.state.record_volume(amount_0 as u64, 0)?;
        } else {
            self.state.record_volume(0, amount_1 as u64)?;
        }
        
        // Handle fees and rebates
        if fee_result.fee_amount > 0 {
            self.state.collect_fee(
                fee_result.fee_amount,
                if zero_for_one { 0 } else { 1 },
                self.current_time,
            )?;
        }
        
        if fee_result.rebate_amount > 0 {
            self.state.pay_rebate(fee_result.rebate_amount, self.current_time)?;
        }
        
        Ok(OrderResult {
            order_type: OrderType::Swap,
            amount_primary: amount_in,
            amount_secondary: if zero_for_one { amount_1 as u64 } else { amount_0 as u64 },
            fee_amount: fee_result.fee_amount,
            rebate_amount: fee_result.rebate_amount,
            work: work_result.total_work as i128,
            final_price: sqrt_price_after,
        })
    }
    
    // ========================================================================
    // Liquidity Operations
    // ========================================================================
    
    /// Add liquidity to a position
    pub fn add_liquidity(
        &mut self,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> Result<OrderResult> {
        // Validate ticks
        require!(
            tick_lower < tick_upper,
            FeelsProtocolError::InvalidTick
        );
        require!(
            tick_lower >= MIN_TICK && tick_upper <= MAX_TICK,
            FeelsProtocolError::InvalidTick
        );
        
        // Get current state
        let sqrt_price = self.state.current_sqrt_price()?;
        let current_tick = self.state.current_tick()?;
        
        // Calculate amounts needed
        let (amount_0, amount_1) = self.calculate_liquidity_amounts(
            sqrt_price,
            tick_lower,
            tick_upper,
            liquidity,
        )?;
        
        // Update ticks
        let fee_growth_global_0 = self.state.get_market_state()?.fee_growth_global_0;
        let fee_growth_global_1 = self.state.get_market_state()?.fee_growth_global_1;
        
        // Note: tick_array_key would need to be calculated based on tick
        let lower_tick_array_key = self.calculate_tick_array_key(tick_lower);
        let upper_tick_array_key = self.calculate_tick_array_key(tick_upper);
        
        self.state.update_tick(
            tick_lower,
            liquidity as i128,
            fee_growth_global_0,
            fee_growth_global_1,
            lower_tick_array_key,
        )?;
        
        self.state.update_tick(
            tick_upper,
            -(liquidity as i128),
            fee_growth_global_0,
            fee_growth_global_1,
            upper_tick_array_key,
        )?;
        
        // Update global liquidity if position is active
        if tick_lower <= current_tick && current_tick < tick_upper {
            let new_liquidity = self.state.current_liquidity()? + liquidity;
            self.state.update_liquidity(new_liquidity)?;
        }
        
        // Calculate work (simplified - would need full physics calculation)
        let work = 0; // Liquidity operations typically have zero work
        
        Ok(OrderResult {
            order_type: OrderType::AddLiquidity,
            amount_primary: amount_0 as u64,
            amount_secondary: amount_1 as u64,
            fee_amount: 0,
            rebate_amount: 0,
            work,
            final_price: sqrt_price,
        })
    }
    
    /// Remove liquidity from a position
    pub fn remove_liquidity(
        &mut self,
        position_id: Pubkey,
        liquidity: u128,
    ) -> Result<OrderResult> {
        // This would need position information loaded through WorkUnit
        // For now, returning a simplified version
        
        // Get current state
        let sqrt_price = self.state.current_sqrt_price()?;
        
        // Would calculate actual amounts based on position ticks
        let amount_0 = 0u64;
        let amount_1 = 0u64;
        
        Ok(OrderResult {
            order_type: OrderType::RemoveLiquidity,
            amount_primary: amount_0,
            amount_secondary: amount_1,
            fee_amount: 0,
            rebate_amount: 0,
            work: 0,
            final_price: sqrt_price,
        })
    }
    
    // ========================================================================
    // Helper Functions
    // ========================================================================
    
    /// Calculate swap amounts using concentrated liquidity math
    fn calculate_swap_amounts(
        &self,
        amount: u64,
        sqrt_price: u128,
        liquidity: u128,
        zero_for_one: bool,
        exact_input: bool,
    ) -> Result<(u128, u128, u128)> {
        // Simplified calculation - would use full concentrated liquidity math
        let sqrt_price_after = if zero_for_one {
            // Price decreases when selling token 0
            sqrt_price - ((amount as u128 * Q64) / liquidity).min(sqrt_price / 2)
        } else {
            // Price increases when selling token 1
            sqrt_price + ((amount as u128 * Q64) / liquidity).min(sqrt_price)
        };
        
        // Calculate amounts based on price movement
        let amount_0 = if zero_for_one {
            amount as u128
        } else {
            liquidity * (sqrt_price_after - sqrt_price) / Q64
        };
        
        let amount_1 = if zero_for_one {
            liquidity * (sqrt_price - sqrt_price_after) / Q64
        } else {
            amount as u128
        };
        
        Ok((amount_0, amount_1, sqrt_price_after))
    }
    
    /// Calculate amounts for adding liquidity
    fn calculate_liquidity_amounts(
        &self,
        sqrt_price: u128,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> Result<(u128, u128)> {
        let sqrt_price_lower = crate::utils::math::get_sqrt_price_at_tick(tick_lower)?;
        let sqrt_price_upper = crate::utils::math::get_sqrt_price_at_tick(tick_upper)?;
        
        let amount_0 = if sqrt_price < sqrt_price_lower {
            // Current price below range
            liquidity * (sqrt_price_upper - sqrt_price_lower) / Q64
        } else if sqrt_price < sqrt_price_upper {
            // Current price in range
            liquidity * (sqrt_price_upper - sqrt_price) / Q64
        } else {
            // Current price above range
            0
        };
        
        let amount_1 = if sqrt_price < sqrt_price_lower {
            // Current price below range
            0
        } else if sqrt_price < sqrt_price_upper {
            // Current price in range
            liquidity * (sqrt_price - sqrt_price_lower) / Q64
        } else {
            // Current price above range
            liquidity * (sqrt_price_upper - sqrt_price_lower) / Q64
        };
        
        Ok((amount_0, amount_1))
    }
    
    /// Calculate tick array key for a given tick
    fn calculate_tick_array_key(&self, tick: i32) -> Pubkey {
        // This would calculate the actual PDA for the tick array
        // For now, returning a dummy value
        Pubkey::default()
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create an OrderManager from accounts by loading them into a WorkUnit
pub fn create_order_manager<'a, 'info>(
    work_unit: &'a mut WorkUnit<'info>,
    market_field: &'info Account<'info, MarketField>,
    buffer_account: &'info Account<'info, BufferAccount>,
    market_manager: &'info AccountLoader<'info, MarketManager>,
    oracle: Option<&'info AccountLoader<'info, UnifiedOracle>>,
    current_time: i64,
) -> Result<OrderManager<'a, 'info>> {
    // Create state context from WorkUnit
    let state_context = crate::logic::state_context::create_state_context(
        work_unit,
        market_field,
        buffer_account,
        market_manager,
        oracle,
    )?;
    
    Ok(OrderManager::new(state_context, current_time))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hub_route_validation() {
        let route = HubRoute {
            pools: vec![Pubkey::default()],
            zero_for_one: vec![true],
        };
        assert!(route.validate().is_ok());
        
        let invalid_route = HubRoute {
            pools: vec![Pubkey::default(), Pubkey::default(), Pubkey::default()],
            zero_for_one: vec![true, false, true],
        };
        assert!(invalid_route.validate().is_err());
    }
}