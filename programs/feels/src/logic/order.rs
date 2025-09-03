/// Unified 3D order management system implementing the three-dimensional trading model.
/// All trading activity (swaps, liquidity provision, leveraged positions) is expressed
/// as orders in 3D space with dimensions: Rate (price/interest), Duration (time commitment),
/// and Leverage (risk level). Includes secure execution with TWAP oracles and reentrancy protection.

use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, MarketManager, TickArray, TickArrayRouter, RiskProfile};
use crate::state::{TwapOracle, FieldCommitment};
use crate::utils::FeeConfig;
// use crate::state::metrics_price::calculate_volatility_safe; // Module removed
use crate::logic::concentrated_liquidity::ConcentratedLiquidityMath;
// use crate::logic::fee_manager::FeeManager; // Replaced by physics-based fees
use crate::utils::{
    add_liquidity_delta, get_amount_0_delta, get_amount_1_delta,
};
use crate::utils::{TickMath, FeeBreakdown};
use crate::state::{Tick3D, duration::Duration};
use crate::logic::tick::TickManager;

// ============================================================================
// Type Definitions
// ============================================================================

// ============================================================================
// Hub-and-Spoke Routing Types
// ============================================================================

/// Simple routing structure for hub-and-spoke architecture
/// All pools must include FeelsSOL, limiting routes to max 2 hops
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct HubRoute {
    /// Pool keys in order of execution
    pub pools: Vec<Pubkey>,
    /// Direction for each pool (true = token0->token1)
    pub zero_for_one: Vec<bool>,
}

impl HubRoute {
    /// Build a route for token pair following hub-and-spoke constraint
    pub fn build(
        token_in: Pubkey,
        token_out: Pubkey,
        feelssol_mint: Pubkey,
        pool_lookup: &dyn Fn(&Pubkey, &Pubkey) -> Option<Pubkey>,
    ) -> Result<Self> {
        // Case 1: Direct swap (one token is FeelsSOL)
        if token_in == feelssol_mint || token_out == feelssol_mint {
            if let Some(pool) = pool_lookup(&token_in, &token_out) {
                let zero_for_one = token_in < token_out; // Canonical ordering
                return Ok(HubRoute {
                    pools: vec![pool],
                    zero_for_one: vec![zero_for_one],
                });
            }
        }
        
        // Case 2: Two-hop through FeelsSOL hub
        if token_in != feelssol_mint && token_out != feelssol_mint {
            let pool1 = pool_lookup(&token_in, &feelssol_mint)
                .ok_or(FeelsProtocolError::PoolNotFound)?;
            let pool2 = pool_lookup(&feelssol_mint, &token_out)
                .ok_or(FeelsProtocolError::PoolNotFound)?;
                
            let zero_for_one_1 = token_in < feelssol_mint;
            let zero_for_one_2 = feelssol_mint < token_out;
            
            return Ok(HubRoute {
                pools: vec![pool1, pool2],
                zero_for_one: vec![zero_for_one_1, zero_for_one_2],
            });
        }
        
        Err(FeelsProtocolError::InvalidRoute.into())
    }
    
    /// Validate route complies with hub constraints
    pub fn validate(&self, feelssol_mint: &Pubkey) -> Result<()> {
        // Check hop limit
        require!(
            self.pools.len() <= crate::constant::MAX_ROUTE_HOPS,
            FeelsProtocolError::RouteTooLong
        );
        
        // Check matching lengths
        require!(
            self.pools.len() == self.zero_for_one.len(),
            FeelsProtocolError::InvalidRoute
        );
        
        // TODO: Validate each pool includes FeelsSOL
        // This requires loading pool state which should be done at execution
        
        Ok(())
    }
}

// ============================================================================
// Legacy code below - TO BE REMOVED
// The sections below contain legacy routing logic that will be removed
// once the unified order system is fully integrated
// ============================================================================

/// DEPRECATED: Legacy routing logic
pub struct RoutingLogic;

impl RoutingLogic {
    /// DEPRECATED: Calculate the optimal route for a given token pair
    pub fn calculate_route(
        token_0: Pubkey,
        token_1: Pubkey,
        feelssol_mint: Pubkey,
        program_id: &Pubkey,
    ) -> OrderRoute {
        OrderRoute::find(token_0, token_1, feelssol_mint, program_id)
    }

    /// Estimate gas costs for different routing strategies
    pub fn estimate_gas_cost(route: &OrderRoute) -> u64 {
        match route {
            OrderRoute::Direct(_) => 50_000,    // Single order compute units
            OrderRoute::TwoHop(_, _) => 95_000, // Two order compute units
        }
    }

    /// Validate that a route is executable
    pub fn validate_route(route: &OrderRoute) -> bool {
        match route {
            OrderRoute::Direct(pool) => *pool != Pubkey::default(),
            OrderRoute::TwoHop(pool1, pool2) => {
                *pool1 != Pubkey::default() && *pool2 != Pubkey::default()
            }
        }
    }
}

/// Derive pool address using proper PDA derivation
/// Uses canonical token ordering to ensure deterministic pool addresses
pub fn derive_pool_address(
    token_0: Pubkey,
    token_1: Pubkey,
    program_id: &Pubkey,
) -> Result<Pubkey> {
    // Use canonical token ordering to ensure deterministic pool addresses
    let (token_a_sorted, token_b_sorted) = crate::utils::CanonicalSeeds::sort_token_mints(&token_0, &token_1);

    // Use proper PDA derivation with program ownership
    let seeds = &[b"pool", token_a_sorted.as_ref(), token_b_sorted.as_ref()];

    let (pool_address, _bump) = Pubkey::find_program_address(seeds, program_id);
    Ok(pool_address)
}

/// Route analysis for client-side optimization
pub struct RouteAnalysis {
    pub route: OrderRoute,
    pub estimated_gas: u64,
    pub estimated_slippage: u16,   // basis points
    pub liquidity_utilization: u8, // percentage
}

impl RouteAnalysis {
    /// Analyze a route for efficiency metrics
    pub fn analyze(route: OrderRoute) -> Self {
        let (estimated_gas, estimated_slippage) = match route.hop_count() {
            1 => (50_000, 30),   // Single hop: lower gas, lower slippage
            2 => (95_000, 60),   // Two hop: higher gas, higher slippage
            _ => (150_000, 100), // Fallback
        };

        RouteAnalysis {
            route,
            estimated_gas,
            estimated_slippage,
            liquidity_utilization: 85, // Default value when pool data not available
        }
    }

    /// Analyze a route with actual pool data for accurate liquidity metrics
    pub fn analyze_with_pools(
        route: OrderRoute,
        pool_liquidity: Vec<u128>,
        swap_amount: u64,
    ) -> Self {
        let (estimated_gas, estimated_slippage) = match route.hop_count() {
            1 => (50_000, 30),   // Single hop: lower gas, lower slippage
            2 => (95_000, 60),   // Two hop: higher gas, higher slippage
            _ => (150_000, 100), // Fallback
        };

        // Calculate liquidity utilization based on swap amount vs available liquidity
        let liquidity_utilization =
            Self::calculate_liquidity_utilization(&pool_liquidity, swap_amount);

        RouteAnalysis {
            route,
            estimated_gas,
            estimated_slippage,
            liquidity_utilization,
        }
    }

    /// Calculate liquidity utilization as a percentage
    /// Higher utilization = more price impact
    fn calculate_liquidity_utilization(pool_liquidity: &[u128], swap_amount: u64) -> u8 {
        if pool_liquidity.is_empty() {
            return 85; // Default fallback
        }

        // For multi-hop swaps, use the minimum liquidity (bottleneck)
        let min_liquidity = pool_liquidity.iter().min().copied().unwrap_or(0);

        if min_liquidity == 0 {
            return 100; // Max utilization if no liquidity
        }

        // Calculate utilization as swap_amount / liquidity
        // Assume 1:1 token value for simplicity (in practice would consider prices)
        let utilization = (swap_amount as u128)
            .saturating_mul(100)
            .saturating_div(min_liquidity);

        // Cap at 100%
        std::cmp::min(utilization, 100) as u8
    }
}

// ============================================================================
// Order Execution Logic
// ============================================================================

/// Order state for tracking computation across all order types
#[derive(Debug)]
pub struct OrderState {
    pub amount_remaining: u64,
    pub amount_calculated: u64,
    pub sqrt_rate: u128,
    pub tick: i32,
    pub fee_amount: u64,
    pub liquidity: u128,
}


/// Helper struct for order step calculations
#[derive(Debug)]
pub struct OrderStep {
    pub sqrt_rate_next: u128,
    pub sqrt_rate_target: u128,
    pub tick_next: i32,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
}

/// Order manager for executing concentrated liquidity orders with security features
pub struct OrderManager;

// ============================================================================
// Secure Order Manager Features (from order_safe.rs)
// ============================================================================

pub struct SecureOrderManager;

impl SecureOrderManager {
    /// Calculate swap fees using physics-based approach
    pub fn calculate_swap_fees_safe(
        fee_config: &FeeConfig,
        amount_in: u64,
        oracle: Option<&AccountLoader<TwapOracle>>,
        _oracle_data: Option<&AccountInfo>,
        field_commitment: Option<&FieldCommitment>,
    ) -> Result<FeeBreakdown> {
        // Calculate base fee using current base rate
        // Use base fee from hysteresis controller if available in field commitment
        let base_fee_bps = if let Some(commitment) = field_commitment {
            // Use hysteresis controller's dynamic base fee
            commitment.base_fee_bps
        } else {
            // Fallback to static fee config
            fee_config.base_fee_rate as u64
        };
        
        let base_fee = amount_in
            .checked_mul(base_fee_bps)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
        
        // Work-based surcharges/rebates are handled separately in instantaneous fee model
        Ok(FeeBreakdown::new(base_fee, 0, 0))
    }
    
    /// Validate oracle price against pool price
    pub fn validate_oracle_price(
        market: &MarketManager,
        oracle: &AccountLoader<TwapOracle>,
    ) -> Result<()> {
        // Get safe oracle price (TWAP)
        let oracle_data = oracle.load()?;
        let oracle_price = oracle_data.get_safe_price()?;
        
        // Compare with pool price
        let pool_price = market.current_sqrt_rate;
        
        // Calculate deviation
        let deviation = if oracle_price > pool_price {
            ((oracle_price - pool_price) * 10000) / pool_price
        } else {
            ((pool_price - oracle_price) * 10000) / pool_price
        };
        
        // Allow up to 5% deviation between oracle and pool
        const MAX_ORACLE_POOL_DEVIATION: u128 = 500; // 5%
        
        require!(
            deviation <= MAX_ORACLE_POOL_DEVIATION,
            FeelsProtocolError::OraclePriceDeviation
        );
        
        Ok(())
    }
    
    /// Get oracle TWAP for different time windows
    pub fn get_oracle_twap(
        oracle: &AccountLoader<TwapOracle>,
        window: OracleTwapWindow,
    ) -> Result<u128> {
        let oracle_data = oracle.load()?;
        match window {
            OracleTwapWindow::Min5 => Ok(oracle_data.twap_5min_a),
            OracleTwapWindow::Min30 => {
                // Approximate 30min TWAP from 5min
                Ok(oracle_data.twap_5min_a)
            },
            OracleTwapWindow::Hour1 => Ok(oracle_data.twap_1hr_a),
            OracleTwapWindow::Hour4 => {
                // Approximate 4hr TWAP from 1hr
                Ok(oracle_data.twap_1hr_a)
            },
            OracleTwapWindow::Hour24 => {
                // Approximate 24hr TWAP from 1hr
                Ok(oracle_data.twap_1hr_a)
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OracleTwapWindow {
    Min5,
    Min30,
    Hour1,
    Hour4,
    Hour24,
}

/// Helper to safely get oracle from remaining accounts
pub fn get_oracle_from_remaining<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    expected_oracle_key: &Pubkey,
) -> Option<AccountLoader<'info, TwapOracle>> {
    remaining_accounts
        .iter()
        .find(|acc| acc.key() == *expected_oracle_key)
        .and_then(|acc| AccountLoader::<TwapOracle>::try_from(acc).ok())
}

/// Helper to get oracle data account from remaining accounts
pub fn get_oracle_data_from_remaining<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    expected_data_key: &Pubkey,
) -> Option<&'info AccountInfo<'info>> {
    remaining_accounts
        .iter()
        .find(|acc| acc.key() == *expected_data_key)
}


// ============================================================================
// Order Manager Implementation
// ============================================================================

impl OrderManager {
    
    /// Cross a tick boundary during order execution
    pub fn cross_tick<'info>(
        market_manager: &mut MarketManager,
        order_state: &mut OrderState,
        tick_next: i32,
        zero_for_one: bool,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<()> {
        // Load the tick data
        let (liquidity_gross, liquidity_net, fee_growth_0, fee_growth_1) = 
            TickManager::get_tick_data(
                market_manager,
                tick_next,
                remaining_accounts,
                tick_array_router,
                program_id,
            )?;
        
        // Apply liquidity changes
        if liquidity_gross > 0 {
            let liquidity_delta = if zero_for_one {
                -liquidity_net
            } else {
                liquidity_net
            };
            
            // Update order state liquidity
            if liquidity_delta >= 0 {
                order_state.liquidity = order_state.liquidity
                    .checked_add(liquidity_delta as u128)
                    .ok_or(FeelsProtocolError::MathOverflow)?;
            } else {
                order_state.liquidity = order_state.liquidity
                    .checked_sub((-liquidity_delta) as u128)
                    .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
            }
            
            // Update tick data if needed
            if fee_growth_0 > 0 || fee_growth_1 > 0 {
                TickManager::update_tick_fee_growth(
                    market_manager,
                    tick_next,
                    market_manager.fee_growth_global_0,
                    market_manager.fee_growth_global_1,
                    remaining_accounts,
                    tick_array_router,
                    program_id,
                )?;
            }
        }
        
        // Update current tick
        order_state.tick = if zero_for_one {
            tick_next - 1
        } else {
            tick_next
        };
        
        // Sync market manager state
        market_manager.current_tick = order_state.tick;
        market_manager.liquidity = order_state.liquidity;
        
        Ok(())
    }
    
    /// Execute concentrated liquidity order
    pub fn execute_concentrated_liquidity_order<'info>(
        order_state: &mut OrderState,
        market_manager: &mut MarketManager,
        sqrt_rate_limit: u128,
        zero_for_one: bool,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<u64> {
        // Adjust rate limit to ensure it's within protocol bounds
        let sqrt_rate_limit_adjusted = Self::adjust_rate_limit(sqrt_rate_limit, zero_for_one);
        
        // Main order loop - iterate through price space
        while Self::should_continue_order(order_state, sqrt_rate_limit_adjusted) {
            // Execute one step of the order within current tick range
            let step = Self::compute_order_step(
                order_state.sqrt_rate,
                sqrt_rate_limit_adjusted,
                order_state.liquidity,
                order_state.amount_remaining,
                market_manager.fee_rate,
                zero_for_one,
            )?;
            
            // Apply step results to order state
            Self::apply_order_step(order_state, &step)?;
            
            // Update pool's fee growth tracking
            Self::update_fee_growth(market_manager, order_state.liquidity, step.fee_amount, zero_for_one)?;
            
            // Handle tick crossing if we hit a boundary
            Self::handle_tick_crossing(
                market_manager, 
                order_state, 
                &step, 
                zero_for_one, 
                remaining_accounts,
                tick_array_router,
                program_id,
            )?;
        }
        
        Ok(order_state.amount_calculated)
    }
    
    /// Check if order should continue based on remaining amount and rate limit
    fn should_continue_order(order_state: &OrderState, sqrt_rate_limit: u128) -> bool {
        order_state.amount_remaining > 0 && order_state.sqrt_rate != sqrt_rate_limit
    }
    
    /// Adjust rate limit to ensure it's within protocol bounds
    fn adjust_rate_limit(sqrt_rate_limit: u128, zero_for_one: bool) -> u128 {
        if zero_for_one {
            // For sells: rate decreases, so limit must be above minimum
            sqrt_rate_limit.max(1) // Minimum non-zero price
        } else {
            // For buys: rate increases, limit is already bounded by caller
            sqrt_rate_limit
        }
    }
    
    /// Apply the results of an order step to the order state
    fn apply_order_step(order_state: &mut OrderState, step: &OrderStep) -> Result<()> {
        order_state.sqrt_rate = step.sqrt_rate_next;
        order_state.amount_remaining = order_state
            .amount_remaining
            .checked_sub(step.amount_in)
            .ok_or(FeelsProtocolError::ArithmeticUnderflow)?;
        order_state.amount_calculated = order_state
            .amount_calculated
            .checked_add(step.amount_out)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        order_state.fee_amount = order_state
            .fee_amount
            .checked_add(step.fee_amount)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        Ok(())
    }
    
    /// Update global fee growth for liquidity providers
    fn update_fee_growth(
        market_manager: &mut MarketManager,
        _liquidity: u128,
        fee_amount: u64,
        zero_for_one: bool,
    ) -> Result<()> {
        // Update fee growth directly on pool
        if market_manager.liquidity > 0 {
            market_manager.accumulate_fee_growth(fee_amount, zero_for_one)?;
        }
        Ok(())
    }
    
    /// Handle tick crossing or update pool state if no crossing occurred
    fn handle_tick_crossing<'info>(
        market_manager: &mut MarketManager,
        order_state: &mut OrderState,
        step: &OrderStep,
        zero_for_one: bool,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<()> {
        if step.sqrt_rate_next == step.sqrt_rate_target {
            // We've hit a tick boundary - cross it
            Self::cross_tick(
                market_manager,
                order_state,
                step.tick_next,
                zero_for_one,
                remaining_accounts,
                tick_array_router,
                program_id,
            )?;
        } else {
            // No tick crossed - just sync pool state with order state
            order_state.tick = TickMath::get_tick_at_sqrt_ratio(order_state.sqrt_rate)?;
            market_manager.current_tick = order_state.tick;
            market_manager.current_sqrt_rate = order_state.sqrt_rate;
            market_manager.liquidity = order_state.liquidity;
        }
        Ok(())
    }
    
    /// Compute a single order step within the current tick range
    fn compute_order_step(
        sqrt_rate_current: u128,
        sqrt_rate_target: u128,
        liquidity: u128,
        amount_remaining: u64,
        fee_rate: u16,
        zero_for_one: bool,
    ) -> Result<OrderStep> {
        // We always use "exact in" mode - consuming a specific input amount
        let exact_in = amount_remaining > 0;
        
        // Calculate the furthest rate we can move given available liquidity
        let sqrt_rate_next = if exact_in {
            ConcentratedLiquidityMath::get_next_sqrt_rate_from_input(
                sqrt_rate_current,
                liquidity,
                amount_remaining,
                zero_for_one,
            )?
        } else {
            ConcentratedLiquidityMath::get_next_sqrt_rate_from_output(
                sqrt_rate_current,
                liquidity,
                amount_remaining,
                zero_for_one,
            )?
        };
        
        // Use the more restrictive of target rate or calculated rate
        let sqrt_rate_next_bounded = if zero_for_one {
            sqrt_rate_next.max(sqrt_rate_target)
        } else {
            sqrt_rate_next.min(sqrt_rate_target)
        };
        
        // Calculate amounts based on price movement
        let amount_in = if zero_for_one {
            get_amount_0_delta(sqrt_rate_next_bounded, sqrt_rate_current, liquidity, true)?
        } else {
            get_amount_1_delta(sqrt_rate_current, sqrt_rate_next_bounded, liquidity, true)?
        };
        
        let amount_out = if zero_for_one {
            get_amount_1_delta(sqrt_rate_next_bounded, sqrt_rate_current, liquidity, false)?
        } else {
            get_amount_0_delta(sqrt_rate_current, sqrt_rate_next_bounded, liquidity, false)?
        };
        
        // Calculate fee based on the actual amount consumed in this step
        let fee_amount = ((amount_in as u128 * fee_rate as u128) / 10000) as u64;
        
        // Find the next initialized tick
        let tick_next = if sqrt_rate_next_bounded == sqrt_rate_target {
            TickMath::get_tick_at_sqrt_ratio(sqrt_rate_target)?
        } else {
            0 // No tick crossed
        };
        
        Ok(OrderStep {
            sqrt_rate_next: sqrt_rate_next_bounded,
            sqrt_rate_target,
            tick_next,
            amount_in: amount_in as u64,
            amount_out: amount_out as u64,
            fee_amount,
        })
    }
    
    
    /// Get oracle volatility from remaining accounts
    #[allow(dead_code)]
    fn get_oracle_volatility(pool: &MarketManager, remaining_accounts: &[AccountInfo]) -> Result<u64> {
        // TODO: Phase 2 - implement get_phase2_extensions
        if pool.has_oracle() {
            let oracle_pubkey = pool.oracle;
            // Try to find oracle in remaining accounts
            if let Some(oracle_account) = remaining_accounts
                .iter()
                .find(|acc| acc.key() == oracle_pubkey)
            {
                if let Ok(oracle_data) = oracle_account.try_borrow_data() {
                    if oracle_data.len() >= 16 {
                        // Read volatility_basis_points at offset 8 after discriminator
                        return Ok(u64::from_le_bytes(
                            oracle_data[8..16].try_into().unwrap_or([0u8; 8])
                        ));
                    }
                }
            }
        }
        Ok(0)
    }
    
    /// Calculate the average leverage of active liquidity
    #[allow(dead_code)]
    fn calculate_average_leverage(pool: &MarketManager) -> Result<u64> {
        // Return the tracked average leverage from the pool
        if pool.avg_leverage_bps > 0 {
            // Convert from basis points to the scale expected by RiskProfile
            // avg_leverage_bps is in basis points (10000 = 1x)
            // LEVERAGE_SCALE is typically also in a similar scale
            Ok(pool.avg_leverage_bps)
        } else {
            // Default to 1x if no leverage tracking data
            Ok(crate::state::RiskProfile::LEVERAGE_SCALE) // Default 1x
        }
    }
    
    /// Update pool volume statistics
    pub fn update_pool_volumes(
        market_manager: &mut MarketManager,
        amount_in: u64,
        amount_out: u64,
        is_token_a_to_b: bool,
        _timestamp: i64,
    ) -> Result<()> {
        if is_token_a_to_b {
            market_manager.total_volume_a = market_manager.total_volume_a
                .checked_add(amount_in as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            market_manager.total_volume_b = market_manager.total_volume_b
                .checked_add(amount_out as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            market_manager.total_volume_b = market_manager.total_volume_b
                .checked_add(amount_in as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            market_manager.total_volume_a = market_manager.total_volume_a
                .checked_add(amount_out as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }
        
        // Update volume directly
        if is_token_a_to_b {
            market_manager.total_volume_a = market_manager.total_volume_a.saturating_add(amount_in as u128);
            market_manager.total_volume_b = market_manager.total_volume_b.saturating_add(amount_out as u128);
        } else {
            market_manager.total_volume_a = market_manager.total_volume_a.saturating_add(amount_out as u128);
            market_manager.total_volume_b = market_manager.total_volume_b.saturating_add(amount_in as u128);
        }
        
        Ok(())
    }
}

// ============================================================================
// 3D Order Management
// ============================================================================

/// Order management logic for the 3D unified order system.
/// Handles the business logic for executing orders across three dimensions:
/// rate (price), duration (time commitment), and leverage (risk).
pub struct OrderManager3D;

impl OrderManager3D {
    /// Calculate the 3D invariant for a position
    /// K = R^wr × D^wd × L^wl
    pub fn calculate_invariant(
        rate: u128,
        duration: Duration,
        leverage: u64,
        weights: &DimensionWeights,
    ) -> Result<u128> {
        // Convert duration to numeric value
        let duration_value = duration.to_slots().max(1) as u128;
        
        // Apply weights using logarithmic approximation
        // Q64 fixed-point math is available for higher precision if needed
        let rate_component = apply_weight(rate, weights.rate_weight)?;
        let duration_component = apply_weight(duration_value, weights.duration_weight)?;
        let leverage_component = apply_weight(leverage as u128, weights.leverage_weight)?;
        
        // Combine components
        rate_component
            .checked_mul(duration_component)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_mul(leverage_component)
            .ok_or(FeelsProtocolError::MathOverflow.into())
    }
    
    /// Find optimal tick placement in 3D space
    pub fn find_optimal_3d_tick(
        market: &MarketManager,
        target_rate: u128,
        duration: Duration,
        leverage: u64,
    ) -> Result<Tick3D> {
        // Rate tick from price
        let rate_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(target_rate)?;
        
        // Duration tick from enum
        let duration_tick = duration.to_tick();
        
        // Leverage tick from risk profile
        let risk_profile = crate::state::RiskProfile::from_leverage_with_market(leverage, market)?;
        let leverage_tick = risk_profile.to_tick();
        
        Ok(Tick3D {
            rate_tick,
            duration_tick,
            leverage_tick,
        })
    }
    
    /// Calculate liquidity distribution across dimensions
    pub fn distribute_liquidity_3d(
        total_liquidity: u128,
        _tick_3d: &Tick3D,
        dimension_weights: &DimensionWeights,
    ) -> Result<Liquidity3D> {
        let total_weight = dimension_weights.total_weight();
        
        // Distribute based on weights
        let rate_liquidity = total_liquidity
            .checked_mul(dimension_weights.rate_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(total_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
            
        let duration_liquidity = total_liquidity
            .checked_mul(dimension_weights.duration_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(total_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
            
        let leverage_liquidity = total_liquidity
            .checked_mul(dimension_weights.leverage_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(total_weight as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
        
        Ok(Liquidity3D {
            rate_liquidity,
            duration_liquidity,
            leverage_liquidity,
            total_effective: total_liquidity,
        })
    }
    
    /// Calculate price impact in 3D space
    pub fn calculate_3d_price_impact(
        market: &MarketManager,
        amount_in: u64,
        tick_3d: &Tick3D,
        _order_type: OrderType,
    ) -> Result<PriceImpact3D> {
        // Get current 3D position
        let current_tick_3d = get_current_3d_tick(market)?;
        
        // Calculate distance in 3D space
        let distance = tick_3d.distance(&current_tick_3d);
        
        // Base impact from amount and liquidity
        let liquidity_impact = (amount_in as u128)
            .checked_mul(10000)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(market.liquidity.max(1))
            .ok_or(FeelsProtocolError::MathOverflow)? as u64;
        
        // Adjust for 3D distance
        let dimensional_multiplier = calculate_dimensional_multiplier(distance);
        
        let total_impact = liquidity_impact
            .saturating_mul(dimensional_multiplier)
            .saturating_div(100);
        
        Ok(PriceImpact3D {
            rate_impact: total_impact / 3, // Simplified distribution
            duration_impact: total_impact / 3,
            leverage_impact: total_impact / 3,
            total_impact,
        })
    }
    
    /// Validate order parameters in 3D space
    pub fn validate_3d_order(
        market: &MarketManager,
        tick_3d: &Tick3D,
        amount: u64,
        leverage: u64,
        _duration: Duration,
    ) -> Result<()> {
        // Validate rate bounds
        require!(
            tick_3d.rate_tick >= crate::utils::MIN_TICK && 
            tick_3d.rate_tick <= crate::utils::MAX_TICK,
            FeelsProtocolError::InvalidTickRange
        );
        
        // Validate duration - all durations are allowed in the unified system
        // The market will determine pricing for different durations
        
        // Validate leverage
        let max_leverage = market.get_max_leverage().unwrap_or(10_000_000); // 10x default
        require!(
            leverage >= crate::state::RiskProfile::LEVERAGE_SCALE &&
            leverage <= max_leverage,
            FeelsProtocolError::InvalidParameter
        );
        
        // Validate amount
        require!(
            amount > 0,
            FeelsProtocolError::InvalidAmount
        );
        
        Ok(())
    }
}

// ============================================================================
// 3D Order Types and Helpers
// ============================================================================

/// Dimension weights for 3D invariant calculation
#[derive(Debug, Clone, Copy)]
pub struct DimensionWeights {
    pub rate_weight: u64,
    pub duration_weight: u64,
    pub leverage_weight: u64,
}

impl DimensionWeights {
    pub fn default() -> Self {
        Self {
            rate_weight: 50,      // 50% weight on rate
            duration_weight: 30,  // 30% weight on duration
            leverage_weight: 20,  // 20% weight on leverage
        }
    }
    
    pub fn total_weight(&self) -> u64 {
        self.rate_weight + self.duration_weight + self.leverage_weight
    }
}

/// Liquidity distribution across dimensions
#[derive(Debug)]
pub struct Liquidity3D {
    pub rate_liquidity: u128,
    pub duration_liquidity: u128,
    pub leverage_liquidity: u128,
    pub total_effective: u128,
}

/// Price impact breakdown by dimension
#[derive(Debug)]
pub struct PriceImpact3D {
    pub rate_impact: u64,      // basis points
    pub duration_impact: u64,  // basis points
    pub leverage_impact: u64,  // basis points
    pub total_impact: u64,     // basis points
}

/// Order types in 3D system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderType {
    Immediate,
    Liquidity,
    Limit,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Apply weight to a value using approximation
fn apply_weight(value: u128, weight: u64) -> Result<u128> {
    // For weight = 100, return value as-is
    // For weight > 100, multiply
    // For weight < 100, divide
    if weight == 100 {
        Ok(value)
    } else if weight > 100 {
        value.checked_mul(weight as u128 / 100)
            .ok_or(FeelsProtocolError::MathOverflow.into())
    } else {
        value.checked_mul(100)
            .and_then(|v| v.checked_div(weight as u128))
            .ok_or(FeelsProtocolError::MathOverflow.into())
    }
}

/// Get current 3D tick from pool state
fn get_current_3d_tick(pool: &MarketManager) -> Result<Tick3D> {
    // For Phase 1, use simplified mapping
    Ok(Tick3D {
        rate_tick: pool.current_tick,
        duration_tick: Duration::Swap.to_tick(), // Default to swap
        leverage_tick: 0, // Default to 1x leverage
    })
}

/// Calculate dimensional multiplier based on distance
fn calculate_dimensional_multiplier(distance: u64) -> u64 {
    // Simple linear scaling for Phase 1
    // Each unit of distance adds 0.1% impact
    100 + (distance / 10).min(1000) // Cap at 10x multiplier
}

// ============================================================================
// Shared Order Logic (from instructions/order.rs and order_modify.rs)
// ============================================================================

/// Calculate 3D fees using keeper-provided field commitments
pub fn calculate_3d_fees(
    amount: u64,
    duration: &Duration,
    leverage: u64,
    keeper_update: Option<&FieldCommitment>,
) -> Result<u64> {
    if let Some(keeper) = keeper_update {
        // Validate the commitment is fresh
        let clock = Clock::get()?;
        let age = clock.unix_timestamp.saturating_sub(keeper.snapshot_ts);
        
        if age <= 60 && keeper.is_fresh(clock.unix_timestamp) {
            // Use keeper-provided fee calculation
            // Get fee basis points from keeper data (hysteresis controller)
            let base_fee_bps = keeper.base_fee_bps;
            let base_fee = (amount as u128)
                .checked_mul(base_fee_bps as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(10000)
                .ok_or(FeelsProtocolError::MathOverflow)? as u64;
            
            // Apply duration and leverage multipliers (simplified for now)
            // TODO: Implement actual multiplier calculation based on keeper data
            let duration_multiplier = 10000u64; // 1.0x
            let leverage_multiplier = 10000u64; // 1.0x
            
            let adjusted_fee = (base_fee as u128)
                .checked_mul(duration_multiplier as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(100)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_mul(leverage_multiplier as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?
                .checked_div(100)
                .ok_or(FeelsProtocolError::MathOverflow)? as u64;
            
            return Ok(adjusted_fee);
        }
    }
    
    // Fallback to simple percentage-based fee if keeper data unavailable
    msg!("Using fallback fee calculation - keeper data unavailable or stale");
    calculate_simple_percentage_fee(amount, duration, leverage)
}

/// Simple percentage-based fee calculation (fallback)
fn calculate_simple_percentage_fee(
    amount: u64,
    duration: &Duration,
    leverage: u64,
) -> Result<u64> {
    // Base fee: 0.3% (30 basis points)
    let base_fee_bps = 30u64;
    
    // Duration multiplier
    let duration_multiplier = match duration {
        Duration::Flash => 150,    // 1.5x for flash
        Duration::Swap => 100,     // 1x for regular swaps
        Duration::Weekly => 90,    // 0.9x for weekly
        Duration::Monthly => 80,   // 0.8x for monthly
        Duration::Quarterly => 70, // 0.7x for quarterly
        Duration::Annual => 60,    // 0.6x for annual
    };
    
    // Leverage multiplier (1x = 100, 2x = 110, 5x = 150, 10x = 200)
    let leverage_scale = crate::state::RiskProfile::LEVERAGE_SCALE;
    let leverage_ratio = leverage.saturating_mul(100).saturating_div(leverage_scale);
    let leverage_multiplier = 100u64.saturating_add(
        leverage_ratio.saturating_sub(100).saturating_div(10)
    );
    
    // Calculate total fee
    let fee = (amount as u128)
        .checked_mul(base_fee_bps as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_mul(duration_multiplier as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_mul(leverage_multiplier as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(10000) // basis points
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(100)   // duration multiplier
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(100)   // leverage multiplier
        .ok_or(FeelsProtocolError::MathOverflow)? as u64;
    
    Ok(fee)
}

/// Calculate margin requirement for a position
pub fn calculate_margin_requirement(
    position_value: u64,
    risk_profile: &RiskProfile,
) -> u64 {
    (position_value as u128)
        .checked_mul(risk_profile.required_margin_ratio as u128)
        .and_then(|m| m.checked_div(10000))
        .unwrap_or(position_value as u128)
        .min(u64::MAX as u128) as u64
}

/// Validate leverage adjustment against oracle
pub fn validate_leverage_adjustment(
    _pool: &MarketManager,
    oracle: &AccountLoader<TwapOracle>,
    new_risk_profile: &RiskProfile,
) -> Result<()> {
    // Check if market conditions support higher leverage
    let oracle_data = oracle.load()?;
    let volatility = oracle_data.volatility_5min;
    let max_safe_leverage = if volatility > 1000 {
        // High volatility - limit leverage
        2_000_000 // 2x
    } else if volatility > 500 {
        3_000_000 // 3x
    } else {
        5_000_000 // 5x
    };
    
    require!(
        new_risk_profile.leverage <= max_safe_leverage,
        FeelsProtocolError::MarketConditionsPreventLeverage
    );
    
    Ok(())
}

/// Calculate fee delta when changing duration
pub fn calculate_duration_fee_delta(
    amount: u64,
    old_duration: &Duration,
    new_duration: &Duration,
    _manager: &MarketManager,
) -> Result<i64> {
    let old_multiplier = match old_duration {
        Duration::Flash => 15000,
        Duration::Swap => 10000,
        Duration::Weekly => 9000,
        Duration::Monthly => 8000,
        Duration::Quarterly => 7000,
        Duration::Annual => 6000,
    };
    
    let new_multiplier = match new_duration {
        Duration::Flash => 15000,
        Duration::Swap => 10000,
        Duration::Weekly => 9000,
        Duration::Monthly => 8000,
        Duration::Quarterly => 7000,
        Duration::Annual => 6000,
    };
    
    let base_fee = (amount as u128 * 30u128 / 10000) as i64; // 0.3% base fee
    let old_fee = base_fee as i64 * old_multiplier / 10000;
    let new_fee = base_fee as i64 * new_multiplier / 10000;
    
    Ok(new_fee - old_fee)
}

/// Build hook context for order events
pub fn build_order_hook_context(
    pool: Pubkey,
    user: Pubkey,
    order_type: &str,
    amount_in: u64,
    amount_out: u64,
) -> crate::logic::hook::HookContext {
    use crate::logic::hook::HookContextBuilder;
    
    let mut context = HookContextBuilder::base(pool, user);
    
    context.data.insert("order_type".to_string(), order_type.to_string());
    context.data.insert("amount_in".to_string(), amount_in.to_string());
    context.data.insert("amount_out".to_string(), amount_out.to_string());
    
    context
}

/// Build hook context for order modification events
pub fn build_modify_hook_context(
    pool: Pubkey,
    user: Pubkey,
    order_id: Pubkey,
    modification_type: &str,
    old_value: u64,
    new_value: u64,
) -> crate::logic::hook::HookContext {
    use crate::logic::hook::HookContextBuilder;
    
    let mut context = HookContextBuilder::base(pool, user);
    
    context.data.insert("order_id".to_string(), order_id.to_string());
    context.data.insert("modification_type".to_string(), modification_type.to_string());
    context.data.insert("old_value".to_string(), old_value.to_string());
    context.data.insert("new_value".to_string(), new_value.to_string());
    
    context
}


// ============================================================================
// Order Transfer Helpers
// ============================================================================

/// Execute transfers for order execution
pub fn execute_order_transfers<'info>(
    ctx: &Context<'_, '_, 'info, 'info, crate::Order<'info>>,
    amount_in: u64,
    amount_out: u64,
    zero_for_one: bool,
) -> Result<()> {
    use crate::utils::cpi_helpers::{transfer_from_user_to_pool, transfer_from_pool_to_user};
    
    // Transfer input tokens from user to pool
    if zero_for_one {
        transfer_from_user_to_pool(
            ctx.accounts.user_token_0.to_account_info(),
            ctx.accounts.market_token_0.to_account_info(),
            ctx.accounts.user.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            amount_in,
        )?;
    } else {
        transfer_from_user_to_pool(
            ctx.accounts.user_token_1.to_account_info(),
            ctx.accounts.market_token_1.to_account_info(),
            ctx.accounts.user.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            amount_in,
        )?;
    }
    
    // Transfer output tokens from pool to user
    let manager = ctx.accounts.market_manager.load()?;
    let (_, market_bump) = Pubkey::find_program_address(
        &[
            b"market",
            manager.token_0_mint.as_ref(),
            manager.token_1_mint.as_ref(),
        ],
        ctx.program_id,
    );
    
    if zero_for_one {
        transfer_from_pool_to_user(
            ctx.accounts.market_token_1.to_account_info(),
            ctx.accounts.user_token_1.to_account_info(),
            ctx.accounts.market_field.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            amount_out,
            &manager,
            market_bump,
        )?;
    } else {
        transfer_from_pool_to_user(
            ctx.accounts.market_token_0.to_account_info(),
            ctx.accounts.user_token_0.to_account_info(),
            ctx.accounts.market_field.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            amount_out,
            &manager,
            market_bump,
        )?;
    }
    
    Ok(())
}

/// Execute transfers for order modifications
pub fn execute_modification_transfers<'info>(
    ctx: &Context<'_, '_, 'info, 'info, crate::OrderModify<'info>>,
    margin_delta: Option<MarginDelta>,
) -> Result<()> {
    use crate::utils::cpi_helpers::{transfer_from_user_to_pool, transfer_from_pool_to_user};
    
    if let Some(delta) = margin_delta {
        match delta {
            MarginDelta::Required(amount) => {
                // Transfer additional margin from user to pool
                transfer_from_user_to_pool(
                    ctx.accounts.user_token_0.as_ref().unwrap().to_account_info(),
                    ctx.accounts.market_token_0.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    amount,
                )?;
            },
            MarginDelta::Releasable(amount) => {
                // Transfer released margin from market to user
                let manager = ctx.accounts.market_manager.load()?;
                let (_, market_bump) = Pubkey::find_program_address(
                    &[
                        b"market",
                        manager.token_0_mint.as_ref(),
                        manager.token_1_mint.as_ref(),
                    ],
                    ctx.program_id,
                );
                
                transfer_from_pool_to_user(
                    ctx.accounts.market_token_0.to_account_info(),
                    ctx.accounts.user_token_0.as_ref().unwrap().to_account_info(),
                    ctx.accounts.market_field.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    amount,
                    &manager,
                    market_bump,
                )?;
            },
        }
    }
    
    Ok(())
}

/// Margin delta types for modifications
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub enum MarginDelta {
    /// Additional margin required from user
    Required(u64),
    /// Margin that can be released to user
    Releasable(u64),
}

/// Validate order parameters (combined validation logic)
pub fn validate_order_parameters(
    amount: u64,
    sqrt_price_limit: u128,
    _duration: &Duration,
    leverage: u64,
    max_slippage_bps: u16,
) -> Result<()> {
    // Validate amount
    require!(amount > 0, FeelsProtocolError::InvalidAmount);
    
    // Validate price limit
    require!(
        sqrt_price_limit > 0,
        FeelsProtocolError::InvalidAmount
    );
    
    // Validate slippage
    require!(
        max_slippage_bps <= 10000,
        FeelsProtocolError::InvalidAmount
    );
    
    // Validate leverage bounds
    require!(
        leverage >= RiskProfile::LEVERAGE_SCALE && leverage <= RiskProfile::MAX_LEVERAGE_SCALE,
        FeelsProtocolError::InvalidPercentage
    );
    
    Ok(())
}

/// Calculate slippage adjusted amounts
pub fn calculate_slippage_adjusted_amounts(
    amount_calculated: u64,
    max_slippage_bps: u16,
    _zero_for_one: bool,
) -> Result<u64> {
    let slippage_factor = 10000u64.saturating_sub(max_slippage_bps as u64);
    
    let adjusted_amount = (amount_calculated as u128)
        .checked_mul(slippage_factor as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?
        .checked_div(10000)
        .ok_or(FeelsProtocolError::MathOverflow)? as u64;
    
    Ok(adjusted_amount)
}

/// Collect accumulated fees for a position
pub fn collect_position_fees(
    _manager: &MarketManager,
    liquidity_ratio: u128,
) -> Result<u64> {
    // Calculate fees based on position
    let estimated_fees = (0u128) // position locked_amount placeholder
        .checked_mul(30u128) // 0.3% fee rate
        .and_then(|f| f.checked_mul(liquidity_ratio))
        .and_then(|f| f.checked_div(u128::MAX))
        .and_then(|f| f.checked_div(10000))
        .ok_or(FeelsProtocolError::MathOverflow)?;
    
    Ok(estimated_fees as u64)
}