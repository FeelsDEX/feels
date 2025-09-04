//! # Unified Order System
//! 
//! This module provides a single, authoritative OrderManager that handles all
//! order types (swaps, liquidity operations, positions) using the WorkUnit
//! pattern for atomic state modifications.
//! 
//! ## Design Principles
//! 
//! 1. **Single Source of Truth**: One OrderManager handles all order logic
//! 2. **WorkUnit Pattern**: All state changes are atomic and reversible
//! 3. **Physics Integration**: Thermodynamic calculations for all operations
//! 4. **Hub-and-Spoke Routing**: All routes go through FeelsSOL

use anchor_lang::prelude::*;
use crate::state::{
    FeelsProtocolError, MarketField, MarketManager, UnifiedOracle,
    TickArray, TickArrayRouter, BufferAccount, FieldCommitment,
};
use crate::error::FeelsError;
use feels_core::constants::*;
use crate::utils::math::{safe, amm};
use crate::logic::{
    state_access::StateContext,
    unit_of_work::{WorkUnit, UnitOfWork},
    thermodynamics,
    tick::TickManager,
    concentrated_liquidity::ConcentratedLiquidityMath,
};

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
#[derive(Debug)]
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
// Unified Order Manager
// ============================================================================

/// Single, authoritative order manager for all trading operations
pub struct OrderManager<'info> {
    /// State context for accessing accounts
    state: StateContext<'info>,
    /// Market field for physics parameters
    market_field: &'info Account<'info, MarketField>,
    /// Market manager for AMM state
    market_manager: &'info Account<'info, MarketManager>,
    /// Oracle for price data
    oracle: &'info Account<'info, UnifiedOracle>,
    /// Buffer for fees/rebates
    buffer: &'info Account<'info, BufferAccount>,
    /// Field commitment for physics
    commitment: Option<&'info Account<'info, FieldCommitment>>,
    /// Current timestamp
    current_time: i64,
    /// Unit of work for atomic operations
    work_unit: UnitOfWork,
}

impl<'info> OrderManager<'info> {
    /// Create new order manager
    pub fn new(
        state: StateContext<'info>,
        market_field: &'info Account<'info, MarketField>,
        market_manager: &'info Account<'info, MarketManager>,
        oracle: &'info Account<'info, UnifiedOracle>,
        buffer: &'info Account<'info, BufferAccount>,
        commitment: Option<&'info Account<'info, FieldCommitment>>,
        current_time: i64,
    ) -> Self {
        Self {
            state,
            market_field,
            market_manager,
            oracle,
            buffer,
            commitment,
            current_time,
            work_unit: UnitOfWork::new(),
        }
    }
    
    // ========================================================================
    // Swap Operations
    // ========================================================================
    
    /// Execute a swap through hub-and-spoke routing
    pub fn execute_swap(
        &mut self,
        route: HubRoute,
        amount_in: u64,
        min_amount_out: u64,
        exact_input: bool,
    ) -> Result<OrderResult> {
        // Validate route
        route.validate()?;
        
        // Start work unit
        self.work_unit.begin()?;
        
        let mut current_amount = amount_in;
        let mut total_work = 0i128;
        let mut total_fees = 0u64;
        
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
        }
        
        // Validate output
        if exact_input {
            require!(
                current_amount >= min_amount_out,
                FeelsProtocolError::SlippageExceeded
            );
        }
        
        // Commit work unit
        self.work_unit.commit()?;
        
        Ok(OrderResult {
            order_type: OrderType::Swap,
            amount_primary: amount_in,
            amount_secondary: current_amount,
            fee_amount: total_fees,
            rebate_amount: 0, // Calculated in hop
            work: total_work,
            final_price: self.market_manager.sqrt_price,
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
        // Create atomic operation
        let operation = WorkUnit::new_swap(
            pool,
            amount_in,
            zero_for_one,
            exact_input,
        );
        
        // Calculate swap amounts using concentrated liquidity math
        let (amount_out, fee_amount, sqrt_price_after) = self.calculate_swap_amounts(
            amount_in,
            zero_for_one,
            exact_input,
        )?;
        
        // Calculate work performed
        let work = self.calculate_swap_work(
            self.market_manager.sqrt_price,
            sqrt_price_after,
            amount_in,
        )?;
        
        // Calculate fees using thermodynamic fee module
        let fee_params = thermodynamics::ThermodynamicFeeParams {
            work,
            amount_in,
            execution_price: sqrt_price_after,
            oracle_price: self.oracle.get_safe_twap_a(),
            base_fee_bps: self.market_field.base_fee_rate,
            kappa: self.market_field.kappa_fee as u32,
            max_rebate_bps: amount_out / 100, // 1% cap
            is_buy: !zero_for_one,
            buffer: Some(self.buffer.clone()),
        };
        
        let fee_result = thermodynamics::calculate_thermodynamic_fee(fee_params)?;
        
        // Apply fees to output
        let final_amount_out = amount_out
            .saturating_sub(fee_result.fee_amount)
            .saturating_add(fee_result.rebate_amount);
        
        // Update state through work unit
        operation.execute(&mut self.state)?;
        
        // Update price
        self.market_manager.sqrt_price = sqrt_price_after;
        
        // Distribute fees to buffer
        if fee_result.fee_amount > 0 {
            fees::distribute_fees_to_buffer(
                &mut self.buffer,
                fee_result.fee_amount,
                if zero_for_one { 1 } else { 0 },
                self.current_time,
            )?;
        }
        
        // Process rebate if any
        if fee_result.rebate_amount > 0 {
            fees::process_rebate_payment(
                &mut self.buffer,
                fee_result.rebate_amount,
                self.current_time,
            )?;
        }
        
        Ok(OrderResult {
            order_type: OrderType::Swap,
            amount_primary: amount_in,
            amount_secondary: final_amount_out,
            fee_amount: fee_result.fee_amount,
            rebate_amount: fee_result.rebate_amount,
            work,
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
        self.validate_tick_range(tick_lower, tick_upper)?;
        require!(liquidity > 0, FeelsError::InvalidLiquidity);
        
        // Start work unit
        self.work_unit.begin()?;
        
        // Create atomic operation
        let operation = WorkUnit::new_add_liquidity(
            self.market_field.pool,
            tick_lower,
            tick_upper,
            liquidity,
        );
        
        // Calculate token amounts
        let (amount0, amount1) = self.calculate_liquidity_amounts(
            self.market_manager.sqrt_price,
            tick_lower,
            tick_upper,
            liquidity,
            true, // round up for adding
        )?;
        
        // Execute operation
        operation.execute(&mut self.state)?;
        
        // Update ticks
        self.update_ticks_for_liquidity(
            tick_lower,
            tick_upper,
            liquidity as i128,
        )?;
        
        // Commit work unit
        self.work_unit.commit()?;
        
        Ok(OrderResult {
            order_type: OrderType::AddLiquidity,
            amount_primary: amount0,
            amount_secondary: amount1,
            fee_amount: 0,
            rebate_amount: 0,
            work: 0, // No work for liquidity operations
            final_price: self.market_manager.sqrt_price,
        })
    }
    
    /// Remove liquidity from a position
    pub fn remove_liquidity(
        &mut self,
        position_id: Pubkey,
        liquidity: u128,
    ) -> Result<OrderResult> {
        // Start work unit
        self.work_unit.begin()?;
        
        // Load position (would come from state)
        let (tick_lower, tick_upper) = self.load_position_ticks(position_id)?;
        
        // Create atomic operation
        let operation = WorkUnit::new_remove_liquidity(
            position_id,
            liquidity,
        );
        
        // Calculate token amounts
        let (amount0, amount1) = self.calculate_liquidity_amounts(
            self.market_manager.sqrt_price,
            tick_lower,
            tick_upper,
            liquidity,
            false, // round down for removing
        )?;
        
        // Execute operation
        operation.execute(&mut self.state)?;
        
        // Update ticks
        self.update_ticks_for_liquidity(
            tick_lower,
            tick_upper,
            -(liquidity as i128),
        )?;
        
        // Commit work unit
        self.work_unit.commit()?;
        
        Ok(OrderResult {
            order_type: OrderType::RemoveLiquidity,
            amount_primary: amount0,
            amount_secondary: amount1,
            fee_amount: 0,
            rebate_amount: 0,
            work: 0,
            final_price: self.market_manager.sqrt_price,
        })
    }
    
    // ========================================================================
    // Position Operations (Entry/Exit)
    // ========================================================================
    
    /// Enter a position (FeelsSOL -> Position Token)
    pub fn enter_position(
        &mut self,
        feelssol_in: u64,
        position_type: crate::instructions::PositionType,
        min_tokens_out: u64,
    ) -> Result<OrderResult> {
        // Start work unit
        self.work_unit.begin()?;
        
        // Create atomic operation
        let operation = WorkUnit::new_enter_position(
            feelssol_in,
            position_type,
        );
        
        // Calculate exchange rate and fees
        let (tokens_out, work) = self.calculate_position_entry(
            feelssol_in,
            position_type,
        )?;
        
        // Calculate fees
        let fee_params = thermodynamics::ThermodynamicFeeParams {
            work,
            amount_in: feelssol_in,
            execution_price: self.market_manager.sqrt_price,
            oracle_price: self.oracle.get_safe_twap_a(),
            base_fee_bps: self.market_field.base_fee_rate,
            kappa: self.market_field.kappa_fee as u32,
            max_rebate_bps: 0, // No rebates on entry
            is_buy: true,
            buffer: Some(self.buffer.clone()),
        };
        
        let fee_result = thermodynamics::calculate_thermodynamic_fee(fee_params)?;
        
        // Apply fees
        let final_tokens_out = tokens_out.saturating_sub(fee_result.fee_amount);
        
        require!(
            final_tokens_out >= min_tokens_out,
            FeelsProtocolError::SlippageExceeded
        );
        
        // Execute operation
        operation.execute(&mut self.state)?;
        
        // Distribute fees
        if fee_result.fee_amount > 0 {
            fees::distribute_fees_to_buffer(
                &mut self.buffer,
                fee_result.fee_amount,
                0, // FeelsSOL is token 0
                self.current_time,
            )?;
        }
        
        // Commit work unit
        self.work_unit.commit()?;
        
        Ok(OrderResult {
            order_type: OrderType::EnterPosition,
            amount_primary: feelssol_in,
            amount_secondary: final_tokens_out,
            fee_amount: fee_result.fee_amount,
            rebate_amount: 0,
            work,
            final_price: self.market_manager.sqrt_price,
        })
    }
    
    /// Exit a position (Position Token -> FeelsSOL)
    pub fn exit_position(
        &mut self,
        position_tokens_in: u64,
        position_type: crate::instructions::PositionType,
        min_feelssol_out: u64,
    ) -> Result<OrderResult> {
        // Similar to enter_position but in reverse
        // Start work unit
        self.work_unit.begin()?;
        
        // Create atomic operation
        let operation = WorkUnit::new_exit_position(
            position_tokens_in,
            position_type,
        );
        
        // Calculate exchange and fees
        let (feelssol_out, work) = self.calculate_position_exit(
            position_tokens_in,
            position_type,
        )?;
        
        // For exits, work might be negative (downhill)
        let fee_params = thermodynamics::ThermodynamicFeeParams {
            work,
            amount_in: position_tokens_in,
            execution_price: self.market_manager.sqrt_price,
            oracle_price: self.oracle.get_safe_twap_a(),
            base_fee_bps: self.market_field.base_fee_rate,
            kappa: self.market_field.kappa_fee as u32,
            max_rebate_bps: feelssol_out / 100, // 1% cap
            is_buy: false,
            buffer: Some(self.buffer.clone()),
        };
        
        let fee_result = thermodynamics::calculate_thermodynamic_fee(fee_params)?;
        
        // Apply fees and rebates
        let final_feelssol_out = feelssol_out
            .saturating_sub(fee_result.fee_amount)
            .saturating_add(fee_result.rebate_amount);
        
        require!(
            final_feelssol_out >= min_feelssol_out,
            FeelsProtocolError::SlippageExceeded
        );
        
        // Execute operation
        operation.execute(&mut self.state)?;
        
        // Handle fees/rebates
        if fee_result.fee_amount > 0 {
            fees::distribute_fees_to_buffer(
                &mut self.buffer,
                fee_result.fee_amount,
                0,
                self.current_time,
            )?;
        }
        
        if fee_result.rebate_amount > 0 {
            fees::process_rebate_payment(
                &mut self.buffer,
                fee_result.rebate_amount,
                self.current_time,
            )?;
        }
        
        // Commit work unit
        self.work_unit.commit()?;
        
        Ok(OrderResult {
            order_type: OrderType::ExitPosition,
            amount_primary: position_tokens_in,
            amount_secondary: final_feelssol_out,
            fee_amount: fee_result.fee_amount,
            rebate_amount: fee_result.rebate_amount,
            work,
            final_price: self.market_manager.sqrt_price,
        })
    }
    
    // ========================================================================
    // Helper Methods
    // ========================================================================
    
    /// Validate tick range
    fn validate_tick_range(&self, tick_lower: i32, tick_upper: i32) -> Result<()> {
        require!(tick_lower < tick_upper, FeelsError::InvalidRange);
        require!(tick_lower >= MIN_TICK, FeelsError::InvalidTick);
        require!(tick_upper <= MAX_TICK, FeelsError::InvalidTick);
        require!(tick_lower % TICK_SPACING == 0, FeelsError::InvalidTick);
        require!(tick_upper % TICK_SPACING == 0, FeelsError::InvalidTick);
        Ok(())
    }
    
    /// Calculate swap amounts using concentrated liquidity math
    fn calculate_swap_amounts(
        &self,
        amount: u64,
        zero_for_one: bool,
        exact_input: bool,
    ) -> Result<(u64, u64, u128)> {
        // Delegate to concentrated liquidity module
        ConcentratedLiquidityMath::compute_swap_step(
            self.market_manager.sqrt_price,
            self.market_manager.liquidity,
            amount,
            zero_for_one,
            exact_input,
        )
    }
    
    /// Calculate work performed in a swap
    fn calculate_swap_work(
        &self,
        sqrt_price_start: u128,
        sqrt_price_end: u128,
        amount: u64,
    ) -> Result<i128> {
        // Use simplified work calculation from fees module
        let work = fees::calculate_swap_work(
            sqrt_price_start,
            sqrt_price_end,
            self.market_manager.liquidity,
            amount,
        )? as i128;
        
        // Work is positive if price moved against trader
        if sqrt_price_end > sqrt_price_start {
            Ok(work)
        } else {
            Ok(-work)
        }
    }
    
    /// Calculate token amounts for liquidity
    fn calculate_liquidity_amounts(
        &self,
        sqrt_price: u128,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
        round_up: bool,
    ) -> Result<(u64, u64)> {
        ConcentratedLiquidityMath::get_amounts_for_liquidity(
            sqrt_price,
            tick_lower,
            tick_upper,
            liquidity,
            round_up,
        )
    }
    
    /// Update ticks for liquidity change
    fn update_ticks_for_liquidity(
        &mut self,
        tick_lower: i32,
        tick_upper: i32,
        liquidity_delta: i128,
    ) -> Result<()> {
        // Would use TickManager through state context
        Ok(())
    }
    
    /// Load position tick range (stub)
    fn load_position_ticks(&self, position_id: Pubkey) -> Result<(i32, i32)> {
        // Would load from actual position state
        Ok((0, 100))
    }
    
    /// Calculate position entry amounts (stub)
    fn calculate_position_entry(
        &self,
        feelssol_in: u64,
        position_type: crate::instructions::PositionType,
    ) -> Result<(u64, i128)> {
        // Would calculate based on position type and market state
        Ok((feelssol_in * 95 / 100, 1000)) // 5% spread, positive work
    }
    
    /// Calculate position exit amounts (stub)
    fn calculate_position_exit(
        &self,
        tokens_in: u64,
        position_type: crate::instructions::PositionType,
    ) -> Result<(u64, i128)> {
        // Would calculate based on position type and market state
        Ok((tokens_in * 105 / 100, -1000)) // 5% improvement, negative work (rebate eligible)
    }
}