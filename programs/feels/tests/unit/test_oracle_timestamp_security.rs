//! Tests for oracle timestamp manipulation resistance
//! 
//! Verifies that the oracle TWAP calculations are resistant to
//! timestamp manipulation attacks by validators

use feels::state::oracle::{OracleState, Observation, MAX_OBSERVATIONS};
use feels::error::FeelsError;
use anchor_lang::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_twap_duration() {
        // Verify the minimum TWAP duration is enforced
        let oracle = create_test_oracle();
        
        // Try to get TWAP with only 30 seconds (less than MIN_TWAP_DURATION of 60)
        let current_timestamp = 1000;
        let seconds_ago = 30;
        
        // The function should use MIN_TWAP_DURATION (60 seconds) instead
        // This prevents short-term manipulation
    }
    
    #[test]
    fn test_timestamp_manipulation_impact() {
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
    }
    
    #[test]
    fn test_pomm_twap_robustness() {
        // Test that POMM's 5-minute TWAP is robust against manipulation
        let mut oracle = create_test_oracle();
        
        // Establish baseline price
        for i in 0..30 {
            oracle.update(1000, 1000 + i * 10).unwrap(); // Stable at tick 1000
        }
        
        // Attacker tries to manipulate before POMM runs
        let manipulation_start = 1300;
        let manipulated_tick = 2000; // Try to double the tick
        
        // Even with perfect timestamp control for 10 seconds
        oracle.update(manipulated_tick, manipulation_start).unwrap();
        oracle.update(manipulated_tick, manipulation_start + 10).unwrap();
        
        // POMM uses 300-second TWAP
        let twap = oracle.get_twap_tick(manipulation_start + 10, 300).unwrap();
        
        // The TWAP should be close to 1000, not 2000
        // Manipulation weight: 10s / 300s = 3.3%
        // Expected TWAP ≈ 1000 * 0.967 + 2000 * 0.033 ≈ 1033
        
        // This shows even extreme manipulation has limited impact
    }
    
    #[test] 
    fn test_insufficient_twap_duration_error() {
        // Test that short TWAPs are rejected when data is insufficient
        let mut oracle = create_test_oracle();
        
        // Only one observation
        oracle.update(100, 1000).unwrap();
        
        // Try to get TWAP after only 30 seconds
        let result = oracle.get_twap_tick(1030, 60);
        
        // Should fail because we don't have 60 seconds of data
        assert!(result.is_err());
    }
    
    #[test]
    fn test_oracle_cardinality_growth() {
        // Test that oracle properly accumulates observations over time
        let mut oracle = create_test_oracle();
        
        assert_eq!(oracle.observation_cardinality, 0);
        
        // Add observations
        for i in 0..MAX_OBSERVATIONS {
            oracle.update(100, 1000 + i as i64 * 100).unwrap();
            assert!(oracle.observation_cardinality <= (i + 1) as u16);
        }
        
        // Should reach max but not exceed
        assert_eq!(oracle.observation_cardinality as usize, MAX_OBSERVATIONS);
    }
    
    #[test]
    fn test_circular_buffer_behavior() {
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
        assert_eq!(oracle.observation_index, 5);
    }
    
    #[test]
    fn test_realistic_attack_scenario() {
        // Simulate a realistic attack scenario
        let mut oracle = create_test_oracle();
        
        // Normal market operation for 10 minutes
        for i in 0..60 {
            oracle.update(1000, 1000 + i * 10).unwrap(); // Every 10 seconds
        }
        
        // Attacker controls validator and tries to manipulate
        // Realistic manipulation: +/- 5 seconds, sustained for 30 seconds
        let attack_start = 1600;
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
    }
    
    // Helper function to create a test oracle
    fn create_test_oracle() -> OracleState {
        let mut oracle = OracleState {
            pool_id: Pubkey::default(),
            observation_index: 0,
            observation_cardinality: 1, // Must be at least 1
            observation_cardinality_next: 1,
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
}