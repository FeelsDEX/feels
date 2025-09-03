/// Advanced router functionality with quote calculation and route optimization
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;

use crate::errors::SdkError;
use crate::router::{Route, PoolInfo, HubRouter};

/// Quote result for a route
#[derive(Debug, Clone)]
pub struct RouteQuote {
    /// The route
    pub route: Route,
    /// Estimated output amount
    pub amount_out: u64,
    /// Total fees paid
    pub total_fees: u64,
    /// Price impact percentage (basis points)
    pub price_impact_bps: u16,
    /// Execution price (output/input)
    pub execution_price: f64,
}

/// Pool reserves for quote calculation
#[derive(Debug, Clone)]
pub struct PoolReserves {
    /// Pool address
    pub pool: Pubkey,
    /// Token A reserves
    pub reserve_a: u64,
    /// Token B reserves
    pub reserve_b: u64,
    /// Last update slot
    pub last_update_slot: u64,
}

/// Advanced router with quote functionality
pub struct AdvancedRouter {
    /// Base router
    base_router: HubRouter,
    /// Pool reserves cache
    reserves: HashMap<Pubkey, PoolReserves>,
    /// Slippage tolerance (basis points)
    default_slippage_bps: u16,
}

impl AdvancedRouter {
    /// Create new advanced router
    pub fn new(feelssol_mint: Pubkey, default_slippage_bps: u16) -> Self {
        Self {
            base_router: HubRouter::new(feelssol_mint),
            reserves: HashMap::new(),
            default_slippage_bps,
        }
    }

    /// Add pool with reserves
    pub fn add_pool_with_reserves(
        &mut self,
        pool: PoolInfo,
        reserves: PoolReserves,
    ) -> Result<(), SdkError> {
        self.base_router.add_pool(pool)?;
        self.reserves.insert(reserves.pool, reserves);
        Ok(())
    }

    /// Update pool reserves
    pub fn update_reserves(&mut self, reserves: PoolReserves) {
        self.reserves.insert(reserves.pool, reserves);
    }

    /// Get quote for a route
    pub fn get_quote(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<RouteQuote, SdkError> {
        // Find route
        let route = self.base_router.find_route(token_in, token_out)?;
        
        // Calculate quote based on hops
        if route.hops == 1 {
            self.calculate_single_hop_quote(&route, amount_in)
        } else {
            self.calculate_two_hop_quote(&route, amount_in)
        }
    }

    /// Calculate quote for single hop
    fn calculate_single_hop_quote(
        &self,
        route: &Route,
        amount_in: u64,
    ) -> Result<RouteQuote, SdkError> {
        let pool = &route.pools[0];
        let reserves = self.reserves.get(&pool.address)
            .ok_or_else(|| SdkError::InvalidPool("No reserves data".to_string()))?;

        // Determine which reserve corresponds to input token
        let (reserve_in, reserve_out) = if pool.token_a == route.token_in {
            (reserves.reserve_a, reserves.reserve_b)
        } else {
            (reserves.reserve_b, reserves.reserve_a)
        };

        // Calculate output using constant product formula
        let amount_out = self.calculate_swap_output(
            amount_in,
            reserve_in,
            reserve_out,
            pool.fee_rate,
        )?;

        // Calculate fees
        let fee_amount = (amount_in as u128 * pool.fee_rate as u128 / 10_000) as u64;

        // Calculate price impact
        let price_impact_bps = self.calculate_price_impact(
            amount_in,
            reserve_in,
            reserve_out,
        );

        Ok(RouteQuote {
            route: route.clone(),
            amount_out,
            total_fees: fee_amount,
            price_impact_bps,
            execution_price: amount_out as f64 / amount_in as f64,
        })
    }

    /// Calculate quote for two hop route
    fn calculate_two_hop_quote(
        &self,
        route: &Route,
        amount_in: u64,
    ) -> Result<RouteQuote, SdkError> {
        // First hop
        let pool1 = &route.pools[0];
        let reserves1 = self.reserves.get(&pool1.address)
            .ok_or_else(|| SdkError::InvalidPool("No reserves data for pool 1".to_string()))?;

        let (reserve_in1, reserve_out1) = if pool1.token_a == route.token_in {
            (reserves1.reserve_a, reserves1.reserve_b)
        } else {
            (reserves1.reserve_b, reserves1.reserve_a)
        };

        let intermediate_amount = self.calculate_swap_output(
            amount_in,
            reserve_in1,
            reserve_out1,
            pool1.fee_rate,
        )?;

        let fee1 = (amount_in as u128 * pool1.fee_rate as u128 / 10_000) as u64;

        // Second hop
        let pool2 = &route.pools[1];
        let reserves2 = self.reserves.get(&pool2.address)
            .ok_or_else(|| SdkError::InvalidPool("No reserves data for pool 2".to_string()))?;

        // Intermediate token should be FeelsSOL
        let (reserve_in2, reserve_out2) = if pool2.token_a == route.token_out {
            (reserves2.reserve_b, reserves2.reserve_a)
        } else {
            (reserves2.reserve_a, reserves2.reserve_b)
        };

        let amount_out = self.calculate_swap_output(
            intermediate_amount,
            reserve_in2,
            reserve_out2,
            pool2.fee_rate,
        )?;

        let fee2 = (intermediate_amount as u128 * pool2.fee_rate as u128 / 10_000) as u64;

        // Calculate total price impact
        let impact1 = self.calculate_price_impact(amount_in, reserve_in1, reserve_out1);
        let impact2 = self.calculate_price_impact(intermediate_amount, reserve_in2, reserve_out2);
        let total_impact = impact1 + impact2; // Simplified - should compound

        Ok(RouteQuote {
            route: route.clone(),
            amount_out,
            total_fees: fee1 + fee2,
            price_impact_bps: total_impact,
            execution_price: amount_out as f64 / amount_in as f64,
        })
    }

    /// Calculate swap output using constant product formula
    fn calculate_swap_output(
        &self,
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_rate: u16,
    ) -> Result<u64, SdkError> {
        // Apply fee to input
        let amount_in_with_fee = amount_in
            .checked_mul(10_000u64.saturating_sub(fee_rate as u64))
            .ok_or_else(|| SdkError::InvalidParameter("Overflow in fee calculation".to_string()))?
            / 10_000;

        // x * y = k formula
        let numerator = (amount_in_with_fee as u128)
            .checked_mul(reserve_out as u128)
            .ok_or_else(|| SdkError::InvalidParameter("Overflow in numerator".to_string()))?;

        let denominator = (reserve_in as u128)
            .checked_add(amount_in_with_fee as u128)
            .ok_or_else(|| SdkError::InvalidParameter("Overflow in denominator".to_string()))?;

        let amount_out = (numerator / denominator) as u64;

        Ok(amount_out)
    }

    /// Calculate price impact in basis points
    fn calculate_price_impact(
        &self,
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
    ) -> u16 {
        // Simplified impact calculation
        let impact_ratio = (amount_in as f64 / reserve_in as f64) * 10_000.0;
        impact_ratio.min(10_000.0) as u16
    }

    /// Get minimum amount out with slippage
    pub fn get_minimum_amount_out(&self, quote: &RouteQuote) -> u64 {
        let slippage_factor = 10_000u64.saturating_sub(self.default_slippage_bps as u64);
        quote.amount_out
            .saturating_mul(slippage_factor)
            .saturating_div(10_000)
    }

    /// Compare multiple routes and return the best
    pub fn get_best_quote(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
        amount_in: u64,
    ) -> Result<RouteQuote, SdkError> {
        // For hub-and-spoke, there's usually only one route
        // This is where you could compare multiple pools if they exist
        self.get_quote(token_in, token_out, amount_in)
    }
}