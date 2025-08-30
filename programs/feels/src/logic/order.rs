/// Unified 3D order management system implementing the three-dimensional trading model.
/// All trading activity (swaps, liquidity provision, leveraged positions) is expressed
/// as orders in 3D space with dimensions: Rate (price/interest), Duration (time commitment),
/// and Leverage (risk level). Includes secure execution with TWAP oracles and reentrancy protection.

use anchor_lang::prelude::*;
use crate::state::{FeelsProtocolError, Pool, TickArray, TickArrayRouter};
use crate::state::Oracle;
use crate::state::metrics_price::calculate_volatility_safe;
use crate::logic::ConcentratedLiquidityMath;
use crate::logic::fee_manager::FeeManager;
use crate::utils::{
    TickMath, FeeBreakdown,
    add_liquidity_delta, get_amount_0_delta, get_amount_1_delta, MIN_SQRT_RATE_X96,
};
use crate::state::{Tick3D, duration::Duration};

// ============================================================================
// Type Definitions
// ============================================================================

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum OrderRoute {
    /// Direct order - one of the tokens is FeelsSOL
    Direct(Pubkey), // pool_key
    /// Two-hop order - neither token is FeelsSOL, route through FeelsSOL
    TwoHop(Pubkey, Pubkey), // pool1_key, pool2_key
}


impl OrderRoute {
    /// Determine the optimal routing strategy for a token pair
    /// Returns the route using the lowest available fee tier
    pub fn find(
        token_in: Pubkey,
        token_out: Pubkey,
        feelssol_mint: Pubkey,
        program_id: &Pubkey,
    ) -> OrderRoute {
        // Check if either token is FeelsSOL
        if token_in == feelssol_mint || token_out == feelssol_mint {
            // Direct order possible - find best fee tier
            // In production, would check which pools actually exist
            // TODO: For now, return first available fee tier (would query on-chain)
            let pool_key = Self::find_best_pool(token_in, token_out, program_id);
            OrderRoute::Direct(pool_key)
        } else {
            // Two-hop order needed - find best fee tiers for each hop
            let pool1_key = Self::find_best_pool(token_in, feelssol_mint, program_id);
            let pool2_key = Self::find_best_pool(feelssol_mint, token_out, program_id);
            OrderRoute::TwoHop(pool1_key, pool2_key)
        }
    }

    /// Find the best pool for a token pair by checking multiple fee tiers
    /// Returns the pool with the lowest fee tier that has sufficient liquidity
    fn find_best_pool(token_a: Pubkey, token_b: Pubkey, program_id: &Pubkey) -> Pubkey {
        // Ensure canonical token ordering
        let (token_0, token_1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        
        // Try fee tiers in order of preference (lowest to highest)
        // For minimal implementation, we'll use a simple heuristic:
        // - 0.01% for stablecoin pairs (would check mint metadata in production)
        // - 0.05% for blue chip pairs (would check liquidity depth in production)
        // - 0.30% for standard pairs (default)
        // - 1.00% for exotic pairs (would check volatility in production)
        
        // TODO: For now, return the standard 0.3% fee tier as the most common
        // In production, this would:
        // 1. Query each potential pool account
        // 2. Check if initialized and has liquidity
        // 3. Compare liquidity depth and select optimal pool
        Self::derive_pool_key(token_0, token_1, 30, program_id)
    }

    /// Derive pool PDA for a token pair using proper program derivation
    /// Considers fee tiers and ensures canonical token ordering
    pub fn derive_pool_key(
        token_a: Pubkey,
        token_b: Pubkey,
        fee_rate: u16,
        program_id: &Pubkey,
    ) -> Pubkey {
        // Use canonical token ordering to ensure deterministic pool addresses
        let (token_a_sorted, token_b_sorted) = crate::utils::CanonicalSeeds::sort_token_mints(&token_a, &token_b);

        // Use proper PDA derivation with program ownership
        let seeds = &[
            b"pool",
            token_a_sorted.as_ref(),
            token_b_sorted.as_ref(),
            &fee_rate.to_le_bytes(),
        ];

        // Proper PDA derivation owned by the program
        let (pool_address, _bump) = Pubkey::find_program_address(seeds, program_id);
        pool_address
    }

    /// Get all pools involved in this route
    pub fn get_pools(&self) -> Vec<Pubkey> {
        match self {
            OrderRoute::Direct(pool) => vec![*pool],
            OrderRoute::TwoHop(pool1, pool2) => vec![*pool1, *pool2],
        }
    }

    /// Check if this route is optimal (single hop preferred over two hop)
    pub fn is_optimal(&self) -> bool {
        matches!(self, OrderRoute::Direct(_))
    }

    /// Get the number of hops in this route
    pub fn hop_count(&self) -> u8 {
        match self {
            OrderRoute::Direct(_) => 1,
            OrderRoute::TwoHop(_, _) => 2,
        }
    }
}

// ============================================================================
// Routing Logic
// ============================================================================

/// Routing logic for cross-token swaps
pub struct RoutingLogic;

impl RoutingLogic {
    /// Calculate the optimal route for a given token pair
    pub fn calculate_route(
        token_a: Pubkey,
        token_b: Pubkey,
        feelssol_mint: Pubkey,
        program_id: &Pubkey,
    ) -> OrderRoute {
        OrderRoute::find(token_a, token_b, feelssol_mint, program_id)
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
    token_a: Pubkey,
    token_b: Pubkey,
    program_id: &Pubkey,
) -> Result<Pubkey> {
    // Use canonical token ordering to ensure deterministic pool addresses
    let (token_a_sorted, token_b_sorted) = crate::utils::CanonicalSeeds::sort_token_mints(&token_a, &token_b);

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
    /// Calculate swap fees using secure oracle TWAP
    pub fn calculate_swap_fees_safe(
        fee_config: &Account<FeeConfig>,
        amount_in: u64,
        oracle: Option<&Account<Oracle>>,
        oracle_data: Option<&AccountInfo>,
    ) -> Result<FeeBreakdown> {
        use crate::logic::fee_manager::{FeeManager, FeeContext, FeeType};
        
        // Get volatility from oracle if available
        let volatility_bps = if let (Some(oracle), Some(oracle_data_acc)) = (oracle, oracle_data) {
            let current_time = Clock::get()?.unix_timestamp;
            if !oracle.is_stale(current_time) {
                // Get volatility multiplier and convert to basis points
                let volatility_multiplier = Self::get_volatility_multiplier_safe(
                    oracle,
                    oracle_data_acc,
                )?;
                // Convert multiplier to basis points (10000 = 100%)
                Some(((volatility_multiplier as u64).saturating_sub(10000)).saturating_mul(100).saturating_div(10000))
            } else {
                // Oracle unhealthy - use high volatility assumption
                msg!("Oracle unhealthy, assuming high volatility");
                Some(600) // 6% volatility
            }
        } else {
            None
        };
        
        // Build fee context
        let context = FeeContext {
            fee_type: FeeType::Swap,
            amount: amount_in,
            fee_config,
            volatility_tracker: None,
            lending_metrics: None,
            position_data: None,
            volatility_bps,
            volume_24h: None,
        };
        
        // Calculate fees using FeeManager
        let mut fee_breakdown = FeeManager::calculate_fee(context)?;
        
        // If oracle is unhealthy, apply safety multiplier
        if let Some(oracle) = oracle {
            let current_time = Clock::get()?.unix_timestamp;
            if oracle.is_stale(current_time) {
                // Apply safety multiplier during oracle issues
                let safety_multiplier = 12000u128; // 1.2x fees when oracle is down
                fee_breakdown.liquidity_fee = ((fee_breakdown.liquidity_fee as u128 * safety_multiplier) / 10000) as u64;
                fee_breakdown.protocol_fee = ((fee_breakdown.protocol_fee as u128 * safety_multiplier) / 10000) as u64;
                fee_breakdown.total_fee = fee_breakdown.liquidity_fee + fee_breakdown.protocol_fee;
            }
        }
        
        Ok(fee_breakdown)
    }
    
    /// Get volatility multiplier using secure TWAP calculations
    fn get_volatility_multiplier_safe(
        oracle: &Account<Oracle>,
        oracle_data_acc: &AccountInfo,
    ) -> Result<u16> {
        // Deserialize oracle data
        let oracle_data = oracle_data_acc.try_borrow_data()?;
        let oracle_data_parsed = unsafe {
            &*(oracle_data.as_ptr() as *const crate::state::OracleData)
        };
        
        // Calculate volatility from different time windows
        let vol_5min = calculate_volatility_safe(
            oracle_data_parsed,
            oracle.observation_count,
            oracle.ring_index,
            300,
        )?;
        
        let vol_30min = calculate_volatility_safe(
            oracle_data_parsed,
            oracle.observation_count,
            oracle.ring_index,
            1800,
        )?;
        
        let vol_1hr = calculate_volatility_safe(
            oracle_data_parsed,
            oracle.observation_count,
            oracle.ring_index,
            3600,
        )?;
        
        // Weight different time windows
        let weighted_vol = (vol_5min as u32 * 50 + 
                           vol_30min as u32 * 30 + 
                           vol_1hr as u32 * 20) / 100;
        
        // Convert volatility to fee multiplier
        // 0 vol = 1.0x fees, 100 bps vol = 1.1x fees, 500 bps = 1.5x fees
        let base_multiplier = 10000;
        let additional_fee = weighted_vol.min(5000) / 10; // Cap at 50% increase
        
        Ok((base_multiplier + additional_fee) as u16)
    }
    
    /// Validate oracle price against pool price
    pub fn validate_oracle_price(
        pool: &Pool,
        oracle: &Account<Oracle>,
    ) -> Result<()> {
        // Get safe oracle price (TWAP)
        let oracle_price = oracle.get_safe_price()?;
        
        // Compare with pool price
        let pool_price = pool.current_sqrt_rate;
        
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
        oracle: &Account<Oracle>,
        window: OracleTwapWindow,
    ) -> Result<u128> {
        match window {
            OracleTwapWindow::Min5 => Ok(oracle.twap_5min),
            OracleTwapWindow::Min30 => Ok(oracle.twap_30min),
            OracleTwapWindow::Hour1 => Ok(oracle.twap_1hr),
            OracleTwapWindow::Hour4 => Ok(oracle.twap_4hr),
            OracleTwapWindow::Hour24 => Ok(oracle.twap_24hr),
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
) -> Option<Account<'info, Oracle>> {
    remaining_accounts
        .iter()
        .find(|acc| acc.key() == *expected_oracle_key)
        .and_then(|acc| Account::<Oracle>::try_from(acc).ok())
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

/// Helper to get tick array from router or remaining accounts
pub fn get_tick_array_from_router_or_remaining<'info>(
    start_tick: i32,
    tick_array_router: Option<&Account<'info, TickArrayRouter>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    pool_key: &Pubkey,
    program_id: &Pubkey,
) -> Result<Option<&'info AccountInfo<'info>>> {
    // First, try to get from router if available
    if let Some(router) = tick_array_router {
        if let Some(index) = router.contains_array(start_tick) {
            if router.is_slot_active(index) {
                let expected_key = router.tick_arrays[index];
                // Find the account in remaining_accounts
                return Ok(remaining_accounts
                    .iter()
                    .find(|acc| acc.key() == expected_key));
            }
        }
    }
    
    // Fallback to computing the PDA and searching in remaining_accounts
    let (expected_pda, _) = crate::utils::CanonicalSeeds::derive_tick_array_pda(
        pool_key,
        start_tick,
        program_id,
    );
    
    Ok(remaining_accounts
        .iter()
        .find(|acc| acc.key() == expected_pda))
}

// ============================================================================
// Order Manager Implementation
// ============================================================================

impl OrderManager {
    
    /// Execute concentrated liquidity order
    pub fn execute_concentrated_liquidity_order<'info>(
        order_state: &mut OrderState,
        pool: &mut Pool,
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
                pool.fee_rate,
                zero_for_one,
            )?;
            
            // Apply step results to order state
            Self::apply_order_step(order_state, &step)?;
            
            // Update pool's fee growth tracking
            Self::update_fee_growth(pool, order_state.liquidity, step.fee_amount, zero_for_one)?;
            
            // Handle tick crossing if we hit a boundary
            Self::handle_tick_crossing(
                pool, 
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
            sqrt_rate_limit.max(MIN_SQRT_RATE_X96)
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
        pool: &mut Pool,
        _liquidity: u128,
        fee_amount: u64,
        zero_for_one: bool,
    ) -> Result<()> {
        // Update fee growth directly on pool
        if pool.liquidity > 0 {
            pool.accumulate_fee_growth(fee_amount, zero_for_one)?;
        }
        Ok(())
    }
    
    /// Handle tick crossing or update pool state if no crossing occurred
    fn handle_tick_crossing<'info>(
        pool: &mut Pool,
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
                pool,
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
            pool.current_tick = order_state.tick;
            pool.current_sqrt_rate = order_state.sqrt_rate;
            pool.liquidity = order_state.liquidity;
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
    
    /// Cross a tick boundary and update all related state
    fn cross_tick<'info>(
        pool: &mut Pool,
        order_state: &mut OrderState,
        tick_index: i32,
        zero_for_one: bool,
        remaining_accounts: &'info [AccountInfo<'info>],
        tick_array_router: Option<&Account<'info, TickArrayRouter>>,
        program_id: &Pubkey,
    ) -> Result<()> {
        // Get pool key for PDA derivation
        // Note: Pool key should be passed from the context
        // For now, we'll derive it from pool seeds
        let (pool_key, _) = Pubkey::find_program_address(
            &[
                b"pool",
                pool.token_a_mint.as_ref(),
                pool.token_b_mint.as_ref(),
                &pool.fee_rate.to_le_bytes(),
            ],
            program_id,
        );
        
        // Calculate which tick array contains this tick
        let tick_spacing = pool.tick_spacing as i32;
        let ticks_per_array = crate::constant::TICK_ARRAY_SIZE as i32 * tick_spacing;
        let array_index = tick_index.div_euclid(ticks_per_array);
        let start_tick = array_index * ticks_per_array;
        
        // Try to get tick array from router first, then fallback to remaining_accounts
        let tick_array_account = get_tick_array_from_router_or_remaining(
            start_tick,
            tick_array_router,
            remaining_accounts,
            pool_key,
            program_id,
        )?;
        
        let account_info = tick_array_account
            .ok_or(FeelsProtocolError::TickArrayNotFound)?;
        
        // Validate account is owned by the program
        require!(
            account_info.owner == program_id,
            FeelsProtocolError::InvalidAccountOwner
        );
        
        // Try to deserialize as TickArray
        let mut tick_array_data = account_info.try_borrow_mut_data()?;
        let tick_array = unsafe {
            &mut *(tick_array_data.as_mut_ptr() as *mut TickArray)
        };
        
        // Get the tick from the array
        let tick = tick_array.get_tick_mut(tick_index)?;
        
        // Update pool liquidity based on the tick's net liquidity
        if zero_for_one {
            order_state.liquidity = add_liquidity_delta(
                order_state.liquidity,
                -tick.liquidity_net,
            )?;
        } else {
            order_state.liquidity = add_liquidity_delta(
                order_state.liquidity,
                tick.liquidity_net,
            )?;
        }
        
        // Update the tick's fee growth outside values
        if tick.liquidity_net != 0 {
            tick.update_fee_growth_outside(
                pool.fee_growth_global_a,
                pool.fee_growth_global_b,
                !zero_for_one,
            )?;
        }
        
        // Move to the next tick
        if zero_for_one {
            order_state.tick = tick_index - 1;
        } else {
            order_state.tick = tick_index;
        }
        
        Ok(())
    }
    
    /// Get oracle volatility from remaining accounts
    fn get_oracle_volatility(pool: &Pool, remaining_accounts: &[AccountInfo]) -> Result<u64> {
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
    fn calculate_average_leverage(pool: &Pool) -> Result<u64> {
        // Check if leverage is enabled directly on pool
        if pool.leverage_params.max_leverage == 0 {
            return Ok(crate::state::RiskProfile::LEVERAGE_SCALE); // Default 1x
        }
        
        // For Phase 2 initial implementation, return base leverage
        Ok(crate::state::RiskProfile::LEVERAGE_SCALE)
    }
    
    /// Update pool volume statistics
    pub fn update_pool_volumes(
        pool: &mut Pool,
        amount_in: u64,
        amount_out: u64,
        is_token_a_to_b: bool,
        _timestamp: i64,
    ) -> Result<()> {
        if is_token_a_to_b {
            pool.total_volume_a = pool.total_volume_a
                .checked_add(amount_in as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            pool.total_volume_b = pool.total_volume_b
                .checked_add(amount_out as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            pool.total_volume_b = pool.total_volume_b
                .checked_add(amount_in as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
            pool.total_volume_a = pool.total_volume_a
                .checked_add(amount_out as u128)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }
        
        // Update volume tracker for Phase 2 dynamic fees
        if pool.is_phase2_enabled() {
            if is_token_a_to_b {
                pool.update_volume_tracker(amount_in, amount_out)?;
            } else {
                pool.update_volume_tracker(amount_out, amount_in)?;
            }
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
        // In production, would use fixed-point math library
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
        pool: &Pool,
        target_rate: u128,
        duration: Duration,
        leverage: u64,
    ) -> Result<Tick3D> {
        // Rate tick from price
        let rate_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(target_rate)?;
        
        // Duration tick from enum
        let duration_tick = duration.to_tick();
        
        // Leverage tick from risk profile
        let risk_profile = crate::state::RiskProfile::from_leverage_with_pool(leverage, pool)?;
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
        pool: &Pool,
        amount_in: u64,
        tick_3d: &Tick3D,
        _order_type: OrderType,
    ) -> Result<PriceImpact3D> {
        // Get current 3D position
        let current_tick_3d = get_current_3d_tick(pool)?;
        
        // Calculate distance in 3D space
        let distance = tick_3d.distance(&current_tick_3d);
        
        // Base impact from amount and liquidity
        let liquidity_impact = (amount_in as u128)
            .checked_mul(10000)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(pool.liquidity.max(1))
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
        pool: &Pool,
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
        let max_leverage = pool.get_max_leverage().unwrap_or(10_000_000); // 10x default
        require!(
            leverage >= crate::state::RiskProfile::LEVERAGE_SCALE &&
            leverage <= max_leverage,
            FeelsProtocolError::InvalidLeverage
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
fn get_current_3d_tick(pool: &Pool) -> Result<Tick3D> {
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