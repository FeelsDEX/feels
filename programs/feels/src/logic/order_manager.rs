/// Unified Order Manager with Physics Integration
/// This is the single authoritative source for all order execution logic
/// Integrates thermodynamic physics calculations with base concentrated liquidity operations
use anchor_lang::prelude::*;
use crate::logic::state_access::StateContext;
use crate::logic::{work_calculation, instantaneous_fee, field_update, conservation_check};
use crate::state::MarketField;
use crate::error::FeelsError;
use crate::constant::*;
use crate::instructions::PositionType;
use crate::state::{Duration, RiskProfile};
use crate::utils::math::safe;

// ============================================================================
// Core Order Manager
// ============================================================================

/// Unified manager for all order operations with physics integration
pub struct OrderManager<'info> {
    /// State context handles all state access
    state: StateContext<'info>,
    /// Market field for physics parameters
    market_field: &'info Account<'info, MarketField>,
}

impl<'info> OrderManager<'info> {
    /// Create new unified order manager with physics support
    pub fn new(
        state: StateContext<'info>,
        market_field: &'info Account<'info, MarketField>,
    ) -> Self {
        Self { state, market_field }
    }
    
    // ========================================================================
    // Swap Execution
    // ========================================================================
    
    /// Execute a token swap with physics-based fees and work calculation
    pub fn execute_swap(
        mut self,
        amount_in: u64,
        min_amount_out: u64,
        zero_for_one: bool,
        sqrt_price_limit: Option<u128>,
    ) -> Result<SwapResult> {
        // Validate inputs
        require!(amount_in > 0, FeelsError::InvalidAmount);
        require!(min_amount_out > 0, FeelsError::InvalidAmount);
        
        // Set price limit
        let sqrt_price_limit = sqrt_price_limit.unwrap_or(
            if zero_for_one { 
                MIN_SQRT_PRICE_U128 + 1 
            } else { 
                MAX_SQRT_PRICE_U128 - 1 
            }
        );
        
        // Get initial market position for physics calculations
        let initial_position = self.get_market_position()?;
        
        // Initialize swap state with path tracking
        let mut amount_remaining = amount_in as i128;
        let mut amount_calculated = 0i128;
        let mut total_fee_amount = 0u64;
        let mut path_segments = Vec::new();
        
        // Main swap loop with physics tracking
        while amount_remaining > 0 && self.state.market.sqrt_price() != sqrt_price_limit {
            let segment_start = self.get_market_position()?;
            // Find next initialized tick
            let (next_tick, initialized) = self.state.ticks.find_next_initialized(
                self.state.market.current_tick(),
                zero_for_one,
            )?;
            
            // Get tick boundary price
            let sqrt_price_next = self.tick_math_sqrt_price(next_tick)?;
            let sqrt_price_target = if zero_for_one {
                sqrt_price_next.max(sqrt_price_limit)
            } else {
                sqrt_price_next.min(sqrt_price_limit)
            };
            
            // Compute swap step within current tick range
            let (amount_in_step, amount_out_step, sqrt_price_next_step) = 
                self.compute_swap_step(
                    self.state.market.sqrt_price(),
                    sqrt_price_target,
                    self.state.market.liquidity(),
                    amount_remaining,
                    zero_for_one,
                )?;
            
            // Update amounts
            amount_remaining -= amount_in_step as i128;
            amount_calculated = if zero_for_one {
                amount_calculated.saturating_sub(amount_out_step as i128)
            } else {
                amount_calculated.saturating_add(amount_out_step as i128)
            };
            
            // Update price
            self.state.market.update_price(sqrt_price_next_step, self.state.market.current_tick());
            
            // Cross tick if we reached it
            if sqrt_price_next_step == sqrt_price_next && initialized {
                let liquidity_delta = self.state.ticks.cross_tick(
                    next_tick,
                    self.state.market.state.fee_growth_global_0,
                    self.state.market.state.fee_growth_global_1,
                )?;
                
                // Update liquidity
                let new_liquidity = if liquidity_delta >= 0 {
                    self.state.market.liquidity()
                        .checked_add(liquidity_delta as u128)
                        .ok_or(FeelsError::MathOverflow)?
                } else {
                    self.state.market.liquidity()
                        .checked_sub((-liquidity_delta) as u128)
                        .ok_or(FeelsError::InsufficientLiquidity)?
                };
                self.state.market.update_liquidity(new_liquidity);
                
                // Move tick
                let new_tick = if zero_for_one { next_tick - 1 } else { next_tick };
                self.state.market.update_price(sqrt_price_next_step, new_tick);
            } else {
                // Update tick to match price
                let new_tick = self.tick_math_get_tick(sqrt_price_next_step)?;
                self.state.market.update_price(sqrt_price_next_step, new_tick);
            }
            
            // Record path segment for work calculation
            let segment_end = self.get_market_position()?;
            path_segments.push(work_calculation::PathSegment {
                start: segment_start,
                end: segment_end,
                liquidity: self.state.market.liquidity(),
                distance: amount_in_step as u128,
            });
        }
        
        // Calculate work done along path using physics
        let work_result = work_calculation::calculate_path_work(
            &path_segments,
            self.market_field,
        )?;
        
        // Calculate physics-based instantaneous fees
        let fee_params = instantaneous_fee::InstantaneousFeeParams {
            amount_in,
            work: work_result.weighted_work,
            base_fee_rate: self.market_field.base_fee_rate,
        };
        
        let fee_result = instantaneous_fee::calculate_instantaneous_fee(fee_params)?;
        total_fee_amount = fee_result.fee_amount;
        
        // Calculate final output amount after fees
        let amount_out = (amount_calculated.unsigned_abs() as u64)
            .saturating_sub(fee_result.fee_amount);
        
        // Validate slippage
        require!(
            amount_out >= min_amount_out,
            FeelsError::InvalidSlippageLimit
        );
        
        // Update field data with physics
        self.update_field_data()?;
        
        // Record volume
        self.state.market.record_volume(zero_for_one, amount_in)?;
        self.state.market.record_volume(!zero_for_one, amount_out)?;
        
        // Collect fees to buffer
        self.state.buffer.collect_fees(zero_for_one, fee_result.fee_amount)?;
        
        // Pay rebate if applicable
        if fee_result.rebate_amount > 0 {
            self.state.buffer.pay_rebate(zero_for_one, fee_result.rebate_amount)?;
        }
        
        // Update fee growth globally
        if self.state.market.liquidity() > 0 && total_fee_amount > 0 {
            let fee_growth_delta = safe::div_u128(
                safe::mul_u128(total_fee_amount as u128, Q128)?,
                self.state.market.liquidity()
            )?;
            
            let current_fee_growth = if zero_for_one {
                self.state.market.state.fee_growth_global_0
            } else {
                self.state.market.state.fee_growth_global_1
            };
            
            self.state.market.update_fee_growth(
                zero_for_one,
                current_fee_growth.wrapping_add(fee_growth_delta)
            );
        }
        
        // Commit all state changes
        self.state.commit()?;
        
        Ok(SwapResult {
            amount_in: amount_in - (amount_remaining as u64),
            amount_out,
            sqrt_price_after: self.state.market.sqrt_price(),
            tick_after: self.state.market.current_tick(),
            fee_amount: total_fee_amount,
            fee_growth: if zero_for_one {
                self.state.market.state.fee_growth_global_0
            } else {
                self.state.market.state.fee_growth_global_1
            },
        })
    }
    
    /// Execute two-hop swap through FeelsSOL hub
    pub fn execute_two_hop_swap(
        mut self,
        amount_in: u64,
        min_amount_out: u64,
        pool1: &Pubkey,
        pool2: &Pubkey,
        zero_for_one_hop1: bool,
        zero_for_one_hop2: bool,
    ) -> Result<SwapResult> {
        msg!("Executing two-hop swap: {} -> FeelsSOL -> target", amount_in);
        
        // First hop: Token A -> FeelsSOL
        let hop1_result = self.execute_swap(
            amount_in,
            0, // No min for intermediate
            zero_for_one_hop1,
            None,
        )?;
        
        // Second hop: FeelsSOL -> Token B
        // Note: In production, would need to reload state for second pool
        let hop2_result = self.execute_swap(
            hop1_result.amount_out,
            min_amount_out,
            zero_for_one_hop2,
            None,
        )?;
        
        // Combine results
        Ok(SwapResult {
            amount_in,
            amount_out: hop2_result.amount_out,
            sqrt_price_after: hop2_result.sqrt_price_after,
            tick_after: hop2_result.tick_after,
            fee_amount: hop1_result.fee_amount + hop2_result.fee_amount,
            fee_growth: hop2_result.fee_growth,
        })
    }
    
    // ========================================================================
    // Position Management
    // ========================================================================
    
    /// Enter a position from FeelsSOL with physics integration
    pub fn enter_position(
        mut self,
        amount_in: u64,
        position_type: &PositionType,
        min_position_tokens: u64,
    ) -> Result<PositionResult> {
        require!(amount_in > 0, FeelsError::InvalidAmount);
        msg!("Entering {:?} position with {} FeelsSOL", position_type, amount_in);
        
        // Calculate exchange rate based on position type and market physics
        let exchange_rate = match position_type {
            PositionType::Time { duration } => {
                // Time positions may have different rates based on duration
                1u128 << 64 // 1.0 for now, would use physics calculation
            },
            PositionType::Leverage { risk_profile } => {
                // Leverage positions may have different rates based on risk
                1u128 << 64 // 1.0 for now, would use physics calculation
            },
        };
        
        let tokens_out = safe::div_u128(
            safe::mul_u128(amount_in as u128, exchange_rate)?,
            1u128 << 64
        )? as u64;
        
        require!(
            tokens_out >= min_position_tokens,
            FeelsError::InvalidSlippageLimit
        );
        
        // TODO: Calculate work for position entry using physics
        // TODO: Apply physics-based fees
        // TODO: Mint position tokens
        // TODO: Transfer FeelsSOL to vault
        
        // Update field data
        self.update_field_data()?;
        
        // Commit state
        self.state.commit()?;
        
        Ok(PositionResult {
            tokens_out,
            exchange_rate,
            fee_amount: 0, // Position entry uses physics-based fee calculation
        })
    }
    
    /// Exit a position to FeelsSOL with physics integration
    pub fn exit_position(
        mut self,
        position_mint: &Pubkey,
        amount_in: u64,
        min_feelssol_out: u64,
    ) -> Result<PositionResult> {
        require!(amount_in > 0, FeelsError::InvalidAmount);
        msg!("Exiting position {} with {} tokens", position_mint, amount_in);
        
        // Calculate exchange rate (would be based on position state and physics)
        let exchange_rate = 1u128 << 64; // 1.0 for now, would use physics calculation
        let feelssol_out = safe::div_u128(
            safe::mul_u128(amount_in as u128, exchange_rate)?,
            1u128 << 64
        )? as u64;
        
        require!(
            feelssol_out >= min_feelssol_out,
            FeelsError::InvalidSlippageLimit
        );
        
        // TODO: Calculate work for position exit using physics
        // TODO: Apply physics-based fees
        // TODO: Burn position tokens
        // TODO: Transfer FeelsSOL from vault
        
        // Update field data
        self.update_field_data()?;
        
        // Commit state
        self.state.commit()?;
        
        Ok(PositionResult {
            tokens_out: feelssol_out,
            exchange_rate,
            fee_amount: 0, // Position exit uses physics-based fee calculation
        })
    }
    
    // ========================================================================
    // Liquidity Management
    // ========================================================================
    
    /// Add liquidity with conservation verification
    pub fn add_liquidity(
        mut self,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> Result<LiquidityResult> {
        // Validate ticks
        require!(tick_lower < tick_upper, FeelsError::InvalidRange);
        require!(tick_lower >= MIN_TICK, FeelsError::InvalidTick);
        require!(tick_upper <= MAX_TICK, FeelsError::InvalidTick);
        require!(tick_lower % TICK_SPACING == 0, FeelsError::InvalidTick);
        require!(tick_upper % TICK_SPACING == 0, FeelsError::InvalidTick);
        require!(liquidity > 0, FeelsError::InvalidLiquidity);
        
        // Take conservation snapshot
        let snapshot_before = self.take_conservation_snapshot()?;
        
        // Calculate token amounts needed
        let (amount0, amount1) = self.calculate_token_amounts(
            self.state.market.sqrt_price(),
            tick_lower,
            tick_upper,
            liquidity,
            true, // rounding up
        )?;
        
        // Update ticks
        let fee_growth_0 = self.state.market.state.fee_growth_global_0;
        let fee_growth_1 = self.state.market.state.fee_growth_global_1;
        
        self.state.ticks.update_tick(tick_lower, liquidity as i128, fee_growth_0, fee_growth_1)?;
        self.state.ticks.update_tick(tick_upper, -(liquidity as i128), fee_growth_0, fee_growth_1)?;
        
        // Update global liquidity if position is in range
        let current_tick = self.state.market.current_tick();
        if current_tick >= tick_lower && current_tick < tick_upper {
            let new_liquidity = self.state.market.liquidity()
                .checked_add(liquidity)
                .ok_or(FeelsError::MathOverflow)?;
            self.state.market.update_liquidity(new_liquidity);
        }
        
        // Create position
        let position_id = self.state.positions.create_position(
            Pubkey::default(), // Would come from context
            tick_lower,
            tick_upper,
            liquidity,
        );
        
        // Update field data
        self.update_field_data()?;
        
        // Verify conservation
        let snapshot_after = self.take_conservation_snapshot()?;
        let conservation_proof = conservation_check::build_conservation_proof(
            &snapshot_before,
            &snapshot_after,
            conservation_check::RebaseOperationType::Liquidity,
        )?;
        
        conservation_check::verify_conservation(&conservation_proof)?;
        
        // Commit state
        self.state.commit()?;
        
        Ok(LiquidityResult {
            position_id,
            amount0,
            amount1,
            liquidity,
        })
    }
    
    /// Remove liquidity from position with physics integration
    pub fn remove_liquidity(
        mut self,
        position_id: u64,
        liquidity_to_remove: u128,
    ) -> Result<LiquidityResult> {
        require!(liquidity_to_remove > 0, FeelsError::InvalidLiquidity);
        
        // Take conservation snapshot
        let snapshot_before = self.take_conservation_snapshot()?;
        
        // TODO: Load position from actual state
        let tick_lower = 0; // Placeholder - would load from position
        let tick_upper = 100; // Placeholder - would load from position
        
        // Calculate token amounts to return
        let (amount0, amount1) = self.calculate_token_amounts(
            self.state.market.sqrt_price(),
            tick_lower,
            tick_upper,
            liquidity_to_remove,
            false, // rounding down
        )?;
        
        // Update ticks
        let fee_growth_0 = self.state.market.state.fee_growth_global_0;
        let fee_growth_1 = self.state.market.state.fee_growth_global_1;
        
        self.state.ticks.update_tick(tick_lower, -(liquidity_to_remove as i128), fee_growth_0, fee_growth_1)?;
        self.state.ticks.update_tick(tick_upper, liquidity_to_remove as i128, fee_growth_0, fee_growth_1)?;
        
        // Update global liquidity if position is in range
        let current_tick = self.state.market.current_tick();
        if current_tick >= tick_lower && current_tick < tick_upper {
            let new_liquidity = self.state.market.liquidity()
                .checked_sub(liquidity_to_remove)
                .ok_or(FeelsError::InsufficientLiquidity)?;
            self.state.market.update_liquidity(new_liquidity);
        }
        
        // Update field data
        self.update_field_data()?;
        
        // Verify conservation
        let snapshot_after = self.take_conservation_snapshot()?;
        let conservation_proof = conservation_check::build_conservation_proof(
            &snapshot_before,
            &snapshot_after,
            conservation_check::RebaseOperationType::Liquidity,
        )?;
        
        conservation_check::verify_conservation(&conservation_proof)?;
        
        // Commit state
        self.state.commit()?;
        
        Ok(LiquidityResult {
            position_id,
            amount0,
            amount1,
            liquidity: 0, // Remaining liquidity after removal
        })
    }
    
    // ========================================================================
    // Limit Orders
    // ========================================================================
    
    /// Place a limit order at specified price with physics integration
    pub fn place_limit_order(
        mut self,
        amount: u64,
        sqrt_price_limit: u128,
        zero_for_one: bool,
        expiration: &Option<i64>,
    ) -> Result<LimitOrderResult> {
        require!(amount > 0, FeelsError::InvalidAmount);
        require!(sqrt_price_limit > 0, FeelsError::InvalidAmount);
        
        // Validate expiration
        if let Some(exp) = expiration {
            let now = Clock::get()?.unix_timestamp;
            require!(*exp > now, FeelsError::InvalidExpiration);
        }
        
        // Convert sqrt price to tick
        let limit_tick = self.tick_math_get_tick(sqrt_price_limit)?;
        require!(limit_tick % TICK_SPACING == 0, FeelsError::InvalidTick);
        
        // Calculate liquidity to add at limit tick
        let liquidity = if zero_for_one {
            // Selling token0 - add liquidity below current price
            require!(limit_tick > self.state.market.current_tick(), FeelsError::InvalidRange);
            self.calculate_liquidity_from_amount0(amount, limit_tick)?
        } else {
            // Selling token1 - add liquidity above current price
            require!(limit_tick < self.state.market.current_tick(), FeelsError::InvalidRange);
            self.calculate_liquidity_from_amount1(amount, limit_tick)?
        };
        
        // Update tick with physics awareness
        let fee_growth_0 = self.state.market.state.fee_growth_global_0;
        let fee_growth_1 = self.state.market.state.fee_growth_global_1;
        self.state.ticks.update_tick(limit_tick, liquidity as i128, fee_growth_0, fee_growth_1)?;
        
        // Update field data
        self.update_field_data()?;
        
        // Create order ID
        let order_id = Clock::get()?.unix_timestamp as u64;
        
        // Commit state
        self.state.commit()?;
        
        Ok(LimitOrderResult {
            order_id,
            placed_at_tick: limit_tick,
            liquidity,
            expiration: expiration.clone(),
        })
    }
    
    // ========================================================================
    // Helper Functions
    // ========================================================================
    
    /// Compute single swap step within tick range (physics version)
    fn compute_swap_step(
        &self,
        sqrt_price_current: u128,
        sqrt_price_target: u128,
        liquidity: u128,
        amount_remaining: i128,
        zero_for_one: bool,
    ) -> Result<(u64, u64, u128)> {
        if liquidity == 0 {
            return Ok((0, 0, sqrt_price_target, 0));
        }
        
        // Calculate max amounts that can be swapped to reach target price
        let (amount_in_max, amount_out_max) = if zero_for_one {
            self.get_amount0_delta(sqrt_price_target, sqrt_price_current, liquidity, true)
                .zip(self.get_amount1_delta(sqrt_price_target, sqrt_price_current, liquidity, false))
                .ok_or(FeelsError::MathError)?
        } else {
            self.get_amount1_delta(sqrt_price_current, sqrt_price_target, liquidity, true)
                .zip(self.get_amount0_delta(sqrt_price_current, sqrt_price_target, liquidity, false))
                .ok_or(FeelsError::MathError)?
        };
        
        let amount_remaining_abs = amount_remaining.unsigned_abs() as u64;
        
        // Determine actual swap amounts
        let (amount_in, amount_out, sqrt_price_next) = if amount_remaining_abs >= amount_in_max {
            // Can swap to target price
            (amount_in_max, amount_out_max, sqrt_price_target)
        } else {
            // Partial swap within tick
            let amount_in = amount_remaining_abs;
            
            // Calculate new sqrt price after swap
            let sqrt_price_next = if zero_for_one {
                self.get_next_sqrt_price_from_input_amount0(
                    sqrt_price_current,
                    liquidity,
                    amount_in,
                )?
            } else {
                self.get_next_sqrt_price_from_input_amount1(
                    sqrt_price_current,
                    liquidity,
                    amount_in,
                )?
            };
            
            // Calculate output amount
            let amount_out = if zero_for_one {
                self.get_amount1_delta(sqrt_price_next, sqrt_price_current, liquidity, false)
                    .ok_or(FeelsError::MathError)?
            } else {
                self.get_amount0_delta(sqrt_price_current, sqrt_price_next, liquidity, false)
                    .ok_or(FeelsError::MathError)?
            };
            
            (amount_in, amount_out, sqrt_price_next)
        };
        
        // Fee calculation is now done at the physics level using work
        Ok((amount_in, amount_out, sqrt_price_next))
    }
    
    /// Calculate position parameters
    fn calculate_position_params(&self, position_type: &PositionType) -> Result<(u128, u16)> {
        match position_type {
            PositionType::Spot => Ok((RATE_PRECISION, self.state.market.params().base_fee_rate)),
            PositionType::Time { duration } => {
                let time_fee = match duration {
                    Duration::Flash => 50, // 0.5%
                    Duration::Short => 100, // 1%
                    Duration::Medium => 200, // 2%
                    Duration::Long => 300, // 3%
                };
                Ok((RATE_PRECISION, time_fee))
            },
            PositionType::Leverage { risk_profile } => {
                let leverage_fee = match risk_profile {
                    RiskProfile::Conservative => 100, // 1%
                    RiskProfile::Balanced => 200, // 2%
                    RiskProfile::Aggressive => 400, // 4%
                };
                Ok((RATE_PRECISION, leverage_fee))
            },
        }
    }
    
    /// Calculate token amounts for liquidity
    fn calculate_token_amounts(
        &self,
        sqrt_price_current: u128,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
        round_up: bool,
    ) -> Result<(u64, u64)> {
        let sqrt_price_lower = self.tick_math_sqrt_price(tick_lower)?;
        let sqrt_price_upper = self.tick_math_sqrt_price(tick_upper)?;
        
        let (amount0, amount1) = if sqrt_price_current <= sqrt_price_lower {
            // Current price below range
            (
                self.get_amount0_delta(sqrt_price_lower, sqrt_price_upper, liquidity, round_up)
                    .ok_or(FeelsError::MathError)?,
                0
            )
        } else if sqrt_price_current < sqrt_price_upper {
            // Current price within range
            (
                self.get_amount0_delta(sqrt_price_current, sqrt_price_upper, liquidity, round_up)
                    .ok_or(FeelsError::MathError)?,
                self.get_amount1_delta(sqrt_price_lower, sqrt_price_current, liquidity, round_up)
                    .ok_or(FeelsError::MathError)?
            )
        } else {
            // Current price above range
            (
                0,
                self.get_amount1_delta(sqrt_price_lower, sqrt_price_upper, liquidity, round_up)
                    .ok_or(FeelsError::MathError)?
            )
        };
        
        Ok((amount0, amount1))
    }
    
    // Concentrated liquidity math helpers (simplified)
    fn get_amount0_delta(&self, sqrt_price_a: u128, sqrt_price_b: u128, liquidity: u128, round_up: bool) -> Option<u64> {
        if sqrt_price_a > sqrt_price_b {
            return self.get_amount0_delta(sqrt_price_b, sqrt_price_a, liquidity, round_up);
        }
        
        let numerator = (liquidity as u256) * ((sqrt_price_b - sqrt_price_a) as u256);
        let denominator = (sqrt_price_b as u256) * (sqrt_price_a as u256);
        
        let result = if round_up {
            (numerator + denominator - 1) / denominator
        } else {
            numerator / denominator
        };
        
        Some(result as u64)
    }
    
    fn get_amount1_delta(&self, sqrt_price_a: u128, sqrt_price_b: u128, liquidity: u128, round_up: bool) -> Option<u64> {
        if sqrt_price_a > sqrt_price_b {
            return self.get_amount1_delta(sqrt_price_b, sqrt_price_a, liquidity, round_up);
        }
        
        let delta = sqrt_price_b - sqrt_price_a;
        let result = (liquidity as u256) * (delta as u256) / Q96;
        
        Some(result as u64)
    }
    
    fn get_next_sqrt_price_from_input_amount0(
        &self,
        sqrt_price: u128,
        liquidity: u128,
        amount: u64,
    ) -> Result<u128> {
        // Simplified - would use full precision math
        Ok(sqrt_price - (amount as u128 * sqrt_price / liquidity))
    }
    
    fn get_next_sqrt_price_from_input_amount1(
        &self,
        sqrt_price: u128,
        liquidity: u128,
        amount: u64,
    ) -> Result<u128> {
        // Simplified - would use full precision math
        Ok(sqrt_price + (amount as u128 * Q96 / liquidity))
    }
    
    fn calculate_liquidity_from_amount0(&self, amount: u64, tick: i32) -> Result<u128> {
        // Simplified
        Ok(amount as u128)
    }
    
    fn calculate_liquidity_from_amount1(&self, amount: u64, tick: i32) -> Result<u128> {
        // Simplified
        Ok(amount as u128)
    }
    
    fn tick_math_sqrt_price(&self, tick: i32) -> Result<u128> {
        // Simplified tick math
        Ok(MIN_SQRT_PRICE_U128 + (tick.abs() as u128 * 1000))
    }
    
    fn tick_math_get_tick(&self, sqrt_price: u128) -> Result<i32> {
        // Simplified inverse
        Ok(((sqrt_price - MIN_SQRT_PRICE_U128) / 1000) as i32)
    }
    
    // ========================================================================
    // Physics Helper Functions
    // ========================================================================
    
    /// Get current market position in 3D space
    fn get_market_position(&self) -> Result<work_calculation::Position3D> {
        Ok(work_calculation::Position3D {
            S: self.market_field.spot_scalar,
            T: self.market_field.time_scalar,
            L: self.market_field.leverage_scalar,
        })
    }
    
    /// Update market field data after operations
    fn update_field_data(&mut self) -> Result<()> {
        let update_context = field_update::FieldUpdateContext {
            market_manager: &self.state.market,
            tick_arrays: vec![], // Would include actual tick arrays
            buffer_account: &self.state.buffer,
        };
        
        field_update::update_market_field_data(
            self.market_field,
            &update_context,
        )?;
        
        Ok(())
    }
    
    /// Take snapshot for conservation verification
    fn take_conservation_snapshot(&self) -> Result<conservation_check::ConservationSnapshot> {
        Ok(conservation_check::ConservationSnapshot {
            spot_value: self.market_field.spot_scalar,
            time_value: self.market_field.time_scalar,
            leverage_value: self.market_field.leverage_scalar,
            buffer_value: self.state.buffer.state.accumulated_fees_0
                .saturating_add(self.state.buffer.state.accumulated_fees_1),
            timestamp: Clock::get()?.unix_timestamp,
        })
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Default)]
pub struct SwapResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub sqrt_price_after: u128,
    pub tick_after: i32,
    pub fee_amount: u64,
    pub fee_growth: u128, // For physics compatibility
}

#[derive(Debug, Default)]
pub struct PositionResult {
    pub tokens_out: u64,
    pub exchange_rate: u128,
    pub fee_amount: u64,
}

#[derive(Debug, Default)]
pub struct LiquidityResult {
    pub position_id: u64,
    pub amount0: u64,
    pub amount1: u64,
    pub liquidity: u128,
}

#[derive(Debug, Default)]
pub struct LimitOrderResult {
    pub order_id: u64,
    pub placed_at_tick: i32,
    pub liquidity: u128,
    pub expiration: Option<i64>,
}

// Type aliases for clarity
type u256 = u128; // Would use proper u256 in production

// Constants (simplified)
const Q96: u128 = 1 << 96;
const Q128: u128 = 1 << 128;