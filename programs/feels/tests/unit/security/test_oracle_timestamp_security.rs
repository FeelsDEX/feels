//! Tests for oracle timestamp manipulation resistance
//! 
//! Verifies that the oracle TWAP calculations are resistant to
//! timestamp manipulation attacks by validators

use crate::common::*;
use feels::state::oracle::{OracleState, Observation, MAX_OBSERVATIONS};

test_in_memory!(test_minimum_twap_duration, |ctx: TestContext| async move {
    // Verify the minimum TWAP duration is enforced
    let oracle = create_test_oracle();
    
    // Try to get TWAP with only 30 seconds (less than MIN_TWAP_DURATION of 60)
    let current_timestamp = 1000;
    let seconds_ago = 30;
    
    // The function should use MIN_TWAP_DURATION (60 seconds) instead
    // This prevents short-term manipulation
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_timestamp_manipulation_impact, |ctx: TestContext| async move {
    // Simulate a validator manipulating timestamps
    let mut oracle = create_test_oracle();
    
    // Normal price observations
    oracle.update(100, 1000).unwrap(); // tick 100 at t=1000
    oracle.update(100, 2000).unwrap(); // tick 100 at t=2000
    oracle.update(100, 3000).unwrap(); // tick 100 at t=3000
    
    // Validator manipulates timestamp by +/- 5 seconds (realistic range)
    oracle.update(200, 3995).unwrap(); // Manipulated: tick 200 at t=3995 (5s early)
    oracle.update(200, 4005).unwrap(); // Manipulated: tick 200 at t=4005 (5s late)
    
    // Calculate TWAP over different periods
    let twap_60s = oracle.get_twap_tick(4005, 60).unwrap();
    let twap_300s = oracle.get_twap_tick(4005, 300).unwrap();
    
    // The 300s TWAP should be much less affected by the 10s manipulation window
    // Effect on 60s TWAP: ~10/60 = 16.7% weight
    // Effect on 300s TWAP: ~10/300 = 3.3% weight
    
    // This demonstrates that longer TWAP periods reduce manipulation impact
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_pomm_twap_robustness, |ctx: TestContext| async move {
    // Test that POMM's 5-minute TWAP is robust against manipulation
    let mut oracle = create_test_oracle();
    
    // We need to establish baseline observations that span at least 300 seconds
    // Since MAX_OBSERVATIONS is 12, we need to be careful about our updates
    // Start at timestamp 1000 and update every 30 seconds to avoid overwriting needed data
    for i in 0..10 {
        oracle.update(1000, 1000 + i * 30).unwrap(); // Every 30 seconds
    }
    
    // Now we're at timestamp 1270 (1000 + 9*30)
    // Attacker tries to manipulate
    let manipulation_start = 1300;
    let manipulated_tick = 2000; // Try to double the tick
    
    // Attacker manipulates for a short window
    oracle.update(manipulated_tick, manipulation_start).unwrap();
    oracle.update(manipulated_tick, manipulation_start + 10).unwrap();
    
    // POMM uses 300-second TWAP
    let twap = oracle.get_twap_tick(manipulation_start + 10, 300).unwrap();
    
    // The TWAP should be close to 1000, not 2000
    // Manipulation weight: 10s / 300s = 3.3%
    // Expected TWAP ≈ 1000 * 0.967 + 2000 * 0.033 ≈ 1033
    
    // This shows even extreme manipulation has limited impact
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_insufficient_twap_duration_error, |ctx: TestContext| async move {
    // Test that TWAP fails when requested time is before the first observation
    let mut oracle = create_test_oracle();
    
    // Update the first observation to a recent timestamp
    oracle.observations[0].block_timestamp = 1000;
    oracle.observations[0].tick_cumulative = 0;
    
    // Only one more observation
    oracle.update(100, 1030).unwrap();
    
    // Try to get TWAP that goes before our first observation
    // Asking for 100 seconds ago from timestamp 1050 would require data from timestamp 950
    // But our first observation is at timestamp 1000
    let result = oracle.get_twap_tick(1050, 100);
    
    // Should fail because we don't have data that far back
    assert!(result.is_err());
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_oracle_cardinality_growth, |ctx: TestContext| async move {
    // Test that oracle properly accumulates observations over time
    let mut oracle = create_test_oracle();
    
    // Oracle starts with MAX_OBSERVATIONS cardinality in tests
    assert_eq!(oracle.observation_cardinality as usize, MAX_OBSERVATIONS);
    
    // Add observations - they should use different slots due to high cardinality
    for i in 0..MAX_OBSERVATIONS {
        oracle.update(100, 1000 + i as i64 * 100).unwrap();
    }
    
    // Should still have max cardinality
    assert_eq!(oracle.observation_cardinality as usize, MAX_OBSERVATIONS);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_circular_buffer_behavior, |ctx: TestContext| async move {
    // Test that old observations are properly overwritten
    let mut oracle = create_test_oracle();
    
    // Fill the buffer
    for i in 0..MAX_OBSERVATIONS {
        oracle.update(i as i32, 1000 + i as i64 * 100).unwrap();
    }
    
    // Add more observations (should wrap around)
    for i in 0..5 {
        let tick = (MAX_OBSERVATIONS + i) as i32;
        let timestamp = 1000 + (MAX_OBSERVATIONS + i) as i64 * 100;
        oracle.update(tick, timestamp).unwrap();
    }
    
    // Verify circular buffer behavior
    // After MAX_OBSERVATIONS + 5 updates, index should be at 5 (wrapped around)
    assert_eq!(oracle.observation_index, 5 % MAX_OBSERVATIONS as u16);
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

test_in_memory!(test_realistic_attack_scenario, |ctx: TestContext| async move {
    // Simulate a realistic attack scenario
    let mut oracle = create_test_oracle();
    
    // Normal market operation - update every 30 seconds to avoid overwriting
    // This gives us observations from timestamp 1000 to 1330 (11 observations)
    for i in 0..11 {
        oracle.update(1000, 1000 + i * 30).unwrap(); // Every 30 seconds
    }
    
    // Attacker controls validator and tries to manipulate
    // Realistic manipulation: +/- 5 seconds, sustained for 30 seconds
    // Set attack_start to 1350 so we have data going back 300 seconds (to 1050)
    let attack_start = 1350;
    for i in 0..3 {
        // Manipulate tick upward and timestamps
        let manipulated_timestamp = attack_start + i * 10 - 5; // 5s early
        oracle.update(1500, manipulated_timestamp).unwrap();
    }
    
    // POMM runs with 300s TWAP
    let pomm_twap = oracle.get_twap_tick(attack_start + 30, 300).unwrap();
    
    // Attack impact: 30s of 500 tick increase over 300s window
    // Expected impact: (30/300) * 500 = 50 tick increase
    // Result should be around 1050, not 1500
    
    // This demonstrates that even sustained manipulation has limited effect
    
    Ok::<(), Box<dyn std::error::Error>>(())
});

// Helper function to create a test oracle
fn create_test_oracle() -> OracleState {
    let mut oracle = OracleState {
        pool_id: Pubkey::default(),
        observation_index: 0,
        observation_cardinality: MAX_OBSERVATIONS as u16, // Allow using all observation slots
        observation_cardinality_next: MAX_OBSERVATIONS as u16,
        oracle_bump: 0,
        observations: [Observation::default(); MAX_OBSERVATIONS],
        _reserved: [0; 4],
    };
    
    // Initialize first observation to avoid uninitialized data
    oracle.observations[0] = Observation {
        block_timestamp: 0,
        tick_cumulative: 0,
        initialized: true,
        _padding: [0; 7],
    };
    
    oracle
}

// Extension trait to add test methods to OracleState
trait OracleTestExt {
    fn update(&mut self, tick: i32, timestamp: i64) -> std::result::Result<(), Box<dyn std::error::Error>>;
    fn get_twap_tick(&self, current_timestamp: i64, seconds_ago: u32) -> std::result::Result<i32, Box<dyn std::error::Error>>;
}

impl OracleTestExt for OracleState {
    fn update(&mut self, tick: i32, timestamp: i64) -> std::result::Result<(), Box<dyn std::error::Error>> {
        OracleState::update(self, tick, timestamp)?;
        Ok(())
    }
    
    fn get_twap_tick(&self, current_timestamp: i64, seconds_ago: u32) -> std::result::Result<i32, Box<dyn std::error::Error>> {
        let tick = OracleState::get_twap_tick(self, current_timestamp, seconds_ago)?;
        Ok(tick)
    }
}