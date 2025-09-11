//! Time utilities for testing time-dependent features like TWAP

use super::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Time utilities for testing
pub struct TimeUtils;

impl TimeUtils {
    /// Get current Unix timestamp
    pub fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Convert seconds to slots (assuming ~400ms per slot)
    pub fn seconds_to_slots(seconds: i64) -> u64 {
        (seconds * 1000 / 400).max(1) as u64
    }

    /// Convert slots to approximate seconds
    pub fn slots_to_seconds(slots: u64) -> i64 {
        (slots * 400 / 1000) as i64
    }

    /// Create a duration for async sleep
    pub fn duration_seconds(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    /// Calculate time until next epoch (for testing epoch transitions)
    pub fn time_until_next_epoch(current_timestamp: i64, epoch_duration: i64) -> i64 {
        let time_in_epoch = current_timestamp % epoch_duration;
        epoch_duration - time_in_epoch
    }
}

/// Time-based test scenarios
pub struct TimeScenarios;

impl TimeScenarios {
    /// Simulate price movement over time with oracle observations
    pub async fn simulate_price_movement(
        ctx: &TestContext,
        market: &Pubkey,
        price_changes: Vec<(i64, u128)>, // (seconds_to_advance, new_price)
    ) -> TestResult<()> {
        for (seconds, _target_price) in price_changes {
            // Advance time
            ctx.advance_time(seconds).await?;
            
            // Observe oracle to update TWAP
            let market_helper = ctx.market_helper();
            market_helper.observe_oracle(market).await?;
            
            // In real scenario, we would manipulate price through swaps
            // For now, just observe at different times
        }
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    /// Test TWAP calculation over a period
    pub async fn test_twap_calculation(
        ctx: &TestContext,
        market: &Pubkey,
        observation_interval: i64,
        num_observations: usize,
    ) -> TestResult<Vec<i64>> {
        let mut timestamps = Vec::new();
        let market_helper = ctx.market_helper();
        
        for _ in 0..num_observations {
            // Observe oracle
            market_helper.observe_oracle(market).await?;
            
            // Get current timestamp (would come from Clock sysvar)
            let slot = ctx.get_slot().await?;
            let timestamp = TimeUtils::slots_to_seconds(slot);
            timestamps.push(timestamp);
            
            // Advance time for next observation
            ctx.advance_time(observation_interval).await?;
        }
        
        Ok(timestamps)
    }

    /// Simulate high-frequency trading scenario
    pub async fn simulate_hft_scenario(
        ctx: &TestContext,
        market: &Pubkey,
        trades_per_second: usize,
        duration_seconds: i64,
    ) -> TestResult<()> {
        let swap_helper = ctx.swap_helper();
        let market_state = ctx.get_account::<Market>(market).await?.unwrap();
        
        let total_trades = (trades_per_second as i64) * duration_seconds;
        let time_between_trades = 1_000 / trades_per_second as i64; // milliseconds
        
        for i in 0..total_trades {
            // Alternate buy/sell
            let (token_in, token_out) = if i % 2 == 0 {
                (&market_state.token_0, &market_state.token_1)
            } else {
                (&market_state.token_1, &market_state.token_0)
            };
            
            // Execute small swap
            swap_helper.swap(
                market,
                token_in,
                token_out,
                constants::SMALL_SWAP,
                &ctx.accounts.alice,
            ).await?;
            
            // Advance time slightly (simulating real-time passage)
            if time_between_trades >= 1000 {
                ctx.advance_time(time_between_trades / 1000).await?;
            }
        }
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }

    /// Test epoch transitions
    pub async fn test_epoch_transition(
        ctx: &TestContext,
        market: &Pubkey,
        epoch_duration: i64,
    ) -> TestResult<()> {
        let market_helper = ctx.market_helper();
        
        // Get initial state
        let initial_market = market_helper.get_market(market).await?.unwrap();
        let initial_epoch = initial_market.current_tick;
        
        // Advance to just before epoch boundary
        let time_to_boundary = TimeUtils::time_until_next_epoch(
            TimeUtils::now(),
            epoch_duration,
        );
        ctx.advance_time(time_to_boundary - 1).await?;
        
        // Trigger observation before epoch change
        market_helper.observe_oracle(market).await?;
        
        // Cross epoch boundary
        ctx.advance_time(2).await?;
        
        // Trigger observation after epoch change
        market_helper.observe_oracle(market).await?;
        
        // Verify epoch changed
        let final_market = market_helper.get_market(market).await?.unwrap();
        assert!(final_market.current_tick > initial_epoch);
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }
}

/// Helper trait for time-based assertions
pub trait TimeAssertions {
    /// Assert that operation completed within time window
    async fn assert_completed_within<F, T>(
        &self,
        max_duration: Duration,
        operation: F,
    ) -> TestResult<T>
    where
        F: std::future::Future<Output = TestResult<T>>;
        
    /// Assert timestamps are properly ordered
    fn assert_timestamps_ordered(&self, timestamps: &[i64]) -> TestResult<()>;
}

impl TimeAssertions for TestContext {
    async fn assert_completed_within<F, T>(
        &self,
        max_duration: Duration,
        operation: F,
    ) -> TestResult<T>
    where
        F: std::future::Future<Output = TestResult<T>>,
    {
        use tokio::time::timeout;
        
        match timeout(max_duration, operation).await {
            Ok(result) => result,
            Err(_) => Err("Operation timed out".into()),
        }
    }
    
    fn assert_timestamps_ordered(&self, timestamps: &[i64]) -> TestResult<()> {
        for i in 1..timestamps.len() {
            if timestamps[i] <= timestamps[i - 1] {
                return Err(format!(
                    "Timestamps not ordered: {} <= {}",
                    timestamps[i], timestamps[i - 1]
                ).into());
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }
}

/// Macro for time-based test setup
#[macro_export]
macro_rules! with_time_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            use $crate::common::time::*;
            
            // Setup test context
            let ctx = TestContext::new(TestEnvironment::in_memory()).await
                .expect("Failed to create test context");
            
            // Run test with time utilities available
            $body(&ctx).await.expect("Test failed");
        }
    };
}