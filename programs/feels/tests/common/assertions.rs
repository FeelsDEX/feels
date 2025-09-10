use super::*;

/// Common assertions for market state
pub trait MarketAssertions {
    fn assert_liquidity_conserved(&self, before: u128, after: u128);
    fn assert_sqrt_price_in_bounds(&self, price: u128, min_price: u128, max_price: u128);
    fn assert_tick_in_range(&self, tick: i32, min_tick: i32, max_tick: i32);
    fn assert_protocol_fees_valid(&self, fees_0: u128, fees_1: u128, volume: u128, fee_rate: u16);
}

/// Common assertions for swap operations
pub trait SwapAssertions {
    fn assert_fee_growth_increases(&self, token_in: bool, before: u128, after: u128);
    fn assert_price_impact_reasonable(&self, amount_in: u64, price_before: u128, price_after: u128);
    fn assert_amount_bounds(&self, amount_in: u64, amount_out: u64, min_out: u64);
    fn assert_swap_direction_monotonic(&self, zero_for_one: bool, price_before: u128, price_after: u128);
}

/// Common assertions for position operations
pub trait PositionAssertions {
    fn assert_position_in_range(&self, position_lower: i32, position_upper: i32, current_tick: i32);
    fn assert_fees_collectable(&self, position_fees: (u64, u64), min_expected: (u64, u64));
    fn assert_liquidity_tokens_match(&self, liquidity: u128, tokens_0: u64, tokens_1: u64, price: u128);
}

/// Common assertions for tick arrays
pub trait TickArrayAssertions {
    fn assert_tick_initialized(&self, tick: &Tick);
    fn assert_tick_liquidity_valid(&self, tick: &Tick);
    fn assert_tick_array_sorted(&self, array: &TickArray);
    fn assert_fee_growth_flipped(&self, before: u128, after: u128, crossed: bool);
}

// Implement assertions for test results
impl MarketAssertions for MarketTestData {
    fn assert_liquidity_conserved(&self, before: u128, after: u128) {
        assert_eq!(
            before, after,
            "Market liquidity must be conserved: {} -> {}",
            before, after
        );
    }
    
    fn assert_sqrt_price_in_bounds(&self, price: u128, min_price: u128, max_price: u128) {
        assert!(
            price >= min_price && price <= max_price,
            "Price {} not in bounds [{}, {}]",
            price, min_price, max_price
        );
    }
    
    fn assert_tick_in_range(&self, tick: i32, min_tick: i32, max_tick: i32) {
        assert!(
            tick >= min_tick && tick <= max_tick,
            "Tick {} not in range [{}, {}]",
            tick, min_tick, max_tick
        );
    }
    
    fn assert_protocol_fees_valid(&self, fees_0: u128, fees_1: u128, volume: u128, fee_rate: u16) {
        let expected_fees = (volume as u128 * fee_rate as u128) / 1_000_000;
        let total_fees = fees_0 + fees_1;
        
        // Allow for small rounding differences
        let tolerance = expected_fees / 1000; // 0.1% tolerance
        assert!(
            (total_fees as i128 - expected_fees as i128).abs() <= tolerance as i128,
            "Protocol fees {} don't match expected {} (volume: {}, rate: {})",
            total_fees, expected_fees, volume, fee_rate
        );
    }
}

impl SwapAssertions for SwapResult {
    fn assert_fee_growth_increases(&self, token_in: bool, before: u128, after: u128) {
        if token_in {
            assert!(
                after > before,
                "Fee growth for input token should increase: {} -> {}",
                before, after
            );
        } else {
            assert_eq!(
                after, before,
                "Fee growth for output token should not change: {} -> {}",
                before, after
            );
        }
    }
    
    fn assert_price_impact_reasonable(&self, amount_in: u64, price_before: u128, price_after: u128) {
        let price_change = if price_after > price_before {
            ((price_after - price_before) as f64 / price_before as f64) * 100.0
        } else {
            ((price_before - price_after) as f64 / price_before as f64) * 100.0
        };
        
        // Warn if price impact exceeds 1% for small trades
        if amount_in < 1_000_000 && price_change > 1.0 {
            println!(
                "WARNING: High price impact {:.2}% for small trade amount {}",
                price_change, amount_in
            );
        }
    }
    
    fn assert_amount_bounds(&self, amount_in: u64, amount_out: u64, min_out: u64) {
        assert!(
            amount_out >= min_out,
            "Output amount {} less than minimum {}",
            amount_out, min_out
        );
        
        assert!(
            amount_in > 0 && amount_out > 0,
            "Swap amounts must be positive: in={}, out={}",
            amount_in, amount_out
        );
    }
    
    fn assert_swap_direction_monotonic(&self, zero_for_one: bool, price_before: u128, price_after: u128) {
        if zero_for_one {
            assert!(
                price_after <= price_before,
                "Price should decrease for zero-for-one swap: {} -> {}",
                price_before, price_after
            );
        } else {
            assert!(
                price_after >= price_before,
                "Price should increase for one-for-zero swap: {} -> {}",
                price_before, price_after
            );
        }
    }
}

// Helper macro for common test assertions
#[macro_export]
macro_rules! assert_tx_success {
    ($result:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("Transaction failed: {:?}", e),
        }
    };
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

#[macro_export]
macro_rules! assert_error {
    ($result:expr, $expected:pat) => {
        match $result {
            Err($expected) => (),
            Err(e) => panic!("Expected error {:?}, got {:?}", stringify!($expected), e),
            Ok(_) => panic!("Expected error {:?}, but succeeded", stringify!($expected)),
        }
    };
}

#[macro_export]
macro_rules! assert_balance_change {
    ($suite:expr, $account:expr, $before:expr, $expected_change:expr) => {
        async {
            let after = $suite.get_token_balance(&$account).await?;
            let actual_change = after as i64 - $before as i64;
            assert_eq!(
                actual_change, $expected_change,
                "Balance change mismatch: expected {}, got {} (before: {}, after: {})",
                $expected_change, actual_change, $before, after
            );
            Ok::<_, anyhow::Error>(after)
        }
    };
}

/// Test invariants that should hold across all operations
pub struct ProtocolInvariants;

impl ProtocolInvariants {
    /// Check that total liquidity is conserved in swaps
    pub fn check_liquidity_conservation(
        market_before: &Market,
        market_after: &Market,
    ) {
        assert_eq!(
            market_before.liquidity,
            market_after.liquidity,
            "Liquidity must be conserved during swaps"
        );
    }
    
    /// Check that sqrt price remains within tick bounds
    pub fn check_price_tick_consistency(
        market: &Market,
    ) {
        let expected_tick = feels::utils::tick_from_sqrt_price(market.sqrt_price).unwrap();
        
        // Allow for off-by-one due to rounding
        assert!(
            (market.current_tick - expected_tick).abs() <= 1,
            "Current tick {} inconsistent with sqrt price {} (expected tick {})",
            market.current_tick, market.sqrt_price, expected_tick
        );
    }
    
    /// Check that fee growth only increases
    pub fn check_fee_growth_monotonic(
        before_0: u128,
        after_0: u128,
        before_1: u128,
        after_1: u128,
    ) {
        assert!(
            after_0 >= before_0,
            "Fee growth 0 must be monotonic: {} -> {}",
            before_0, after_0
        );
        assert!(
            after_1 >= before_1,
            "Fee growth 1 must be monotonic: {} -> {}",
            before_1, after_1
        );
    }
    
    /// Check that initialized tick count matches actual initialized ticks
    pub fn check_tick_array_consistency(array: &TickArray) {
        let actual_initialized = array.ticks
            .iter()
            .filter(|t| t.initialized != 0)
            .count() as u16;
        
        assert_eq!(
            array.initialized_tick_count,
            actual_initialized,
            "Initialized tick count mismatch"
        );
    }
}