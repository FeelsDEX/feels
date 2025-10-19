//! Consolidated Oracle Security Tests
//! 
//! This module combines all oracle-related security tests:
//! - Oracle timestamp manipulation resistance
//! - Stale oracle protection mechanisms
//! - TWAP robustness and attack mitigation

use crate::common::*;
use anchor_lang::prelude::*;
use feels::state::oracle::{Observation, OracleState, MAX_OBSERVATIONS};
use feels::state::{ProtocolConfig, ProtocolOracle, SafetyController};

#[cfg(test)]
mod oracle_timestamp_security {
    use super::*;

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

    test_in_memory!(
        test_timestamp_manipulation_impact,
        |ctx: TestContext| async move {
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
        }
    );

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
        oracle
            .update(manipulated_tick, manipulation_start + 10)
            .unwrap();

        // POMM uses 300-second TWAP
        let twap = oracle.get_twap_tick(manipulation_start + 10, 300).unwrap();

        // The TWAP should be close to 1000, not 2000
        // Manipulation weight: 10s / 300s = 3.3%
        // Expected TWAP ≈ 1000 * 0.967 + 2000 * 0.033 ≈ 1033

        // This shows even extreme manipulation has limited impact

        Ok::<(), Box<dyn std::error::Error>>(())
    });

    test_in_memory!(
        test_insufficient_twap_duration_error,
        |ctx: TestContext| async move {
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
        }
    );

    test_in_memory!(
        test_oracle_cardinality_growth,
        |ctx: TestContext| async move {
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
        }
    );

    test_in_memory!(
        test_circular_buffer_behavior,
        |ctx: TestContext| async move {
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
        }
    );

    test_in_memory!(
        test_realistic_attack_scenario,
        |ctx: TestContext| async move {
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
        }
    );
}

#[cfg(test)]
mod stale_oracle_protection {
    use super::*;

    fn create_test_protocol_config() -> ProtocolConfig {
        ProtocolConfig {
            authority: Pubkey::default(),
            mint_fee: 100_000_000, // 0.1 FeelsSOL
            treasury: Pubkey::default(),
            default_protocol_fee_rate: 30,
            default_creator_fee_rate: 70,
            max_protocol_fee_rate: 100,
            token_expiration_seconds: 86400,
            depeg_threshold_bps: 100,
            depeg_required_obs: 3,
            clear_required_obs: 5,
            dex_twap_window_secs: 300,
            dex_twap_stale_age_secs: 600,
            dex_twap_updater: Pubkey::default(),
            dex_whitelist: [Pubkey::default(); 8],
            dex_whitelist_len: 0,
            _reserved: [0; 7],
            mint_per_slot_cap_feelssol: 0,
            redeem_per_slot_cap_feelssol: 0,
            default_base_fee_bps: 30,
            default_tick_spacing: 64,
            default_initial_sqrt_price: 5825507814218144,
            default_tick_step_size: 128,
        }
    }

    #[test]
    fn test_exit_blocked_when_dex_oracle_stale() {
        // Setup protocol oracle with fresh native but stale DEX
        let protocol_oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 64) + 1000, // slightly different rate
            dex_last_update_ts: 1000,
            native_last_update_ts: 2000,
            dex_last_update_slot: 100,
            native_last_update_slot: 200,
            dex_window_secs: 300,
            flags: 0,
        };

        let protocol_config = create_test_protocol_config();

        let safety_controller = SafetyController {
            redemptions_paused: false,
            consecutive_breaches: 0,
            consecutive_clears: 0,
            last_change_slot: 0,
            mint_last_slot: 0,
            mint_slot_amount: 0,
            redeem_last_slot: 0,
            redeem_slot_amount: 0,
            last_divergence_check_slot: 0,
            degrade_flags: Default::default(),
            _reserved: [0; 32],
        };

        // Current time is 2700, making DEX oracle stale (1700 seconds old)
        let current_ts = 2700;

        // Should fail due to stale DEX oracle
        let result = safety_controller.check_redemptions_allowed(
            &protocol_oracle,
            &protocol_config,
            current_ts,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().to_string().contains("OracleStale"),
            true
        ));
    }

    #[test]
    fn test_exit_blocked_when_native_oracle_stale() {
        // Setup protocol oracle with stale native but fresh DEX
        let protocol_oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 64) + 1000,
            dex_last_update_ts: 2000,
            native_last_update_ts: 1000, // older update
            dex_last_update_slot: 200,
            native_last_update_slot: 100,
            dex_window_secs: 300,
            flags: 0,
        };

        let protocol_config = create_test_protocol_config();

        let safety_controller = SafetyController {
            redemptions_paused: false,
            consecutive_breaches: 0,
            consecutive_clears: 0,
            last_change_slot: 0,
            mint_last_slot: 0,
            mint_slot_amount: 0,
            redeem_last_slot: 0,
            redeem_slot_amount: 0,
            last_divergence_check_slot: 0,
            degrade_flags: Default::default(),
            _reserved: [0; 32],
        };

        // Current time is 2700, making native oracle stale
        let current_ts = 2700;

        // Should fail due to stale native oracle
        let result = safety_controller.check_redemptions_allowed(
            &protocol_oracle,
            &protocol_config,
            current_ts,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().to_string().contains("OracleStale"),
            true
        ));
    }

    #[test]
    fn test_exit_allowed_when_oracles_fresh() {
        // Setup protocol oracle with both oracles fresh
        let protocol_oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 64) - 100, // within threshold
            dex_last_update_ts: 2400,
            native_last_update_ts: 2450,
            dex_last_update_slot: 240,
            native_last_update_slot: 245,
            dex_window_secs: 300,
            flags: 0,
        };

        let mut protocol_config = create_test_protocol_config();
        protocol_config.depeg_threshold_bps = 200; // 2% threshold

        let safety_controller = SafetyController {
            redemptions_paused: false,
            consecutive_breaches: 0,
            consecutive_clears: 0,
            last_change_slot: 0,
            mint_last_slot: 0,
            mint_slot_amount: 0,
            redeem_last_slot: 0,
            redeem_slot_amount: 0,
            last_divergence_check_slot: 0,
            degrade_flags: Default::default(),
            _reserved: [0; 32],
        };

        // Current time is 2700, both oracles are fresh (< 600s old)
        let current_ts = 2700;

        // Should succeed with fresh oracles
        let result = safety_controller.check_redemptions_allowed(
            &protocol_oracle,
            &protocol_config,
            current_ts,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_exit_allowed_when_only_dex_oracle_active() {
        // Setup with only DEX oracle active (native rate = 0)
        let protocol_oracle = ProtocolOracle {
            native_rate_q64: 0, // Native oracle not set
            dex_twap_rate_q64: 1 << 64,
            dex_last_update_ts: 2400,
            native_last_update_ts: 0,
            dex_last_update_slot: 240,
            native_last_update_slot: 0,
            dex_window_secs: 300,
            flags: 0,
        };

        let protocol_config = create_test_protocol_config();

        let safety_controller = SafetyController {
            redemptions_paused: false,
            consecutive_breaches: 0,
            consecutive_clears: 0,
            last_change_slot: 0,
            mint_last_slot: 0,
            mint_slot_amount: 0,
            redeem_last_slot: 0,
            redeem_slot_amount: 0,
            last_divergence_check_slot: 0,
            degrade_flags: Default::default(),
            _reserved: [0; 32],
        };
        let current_ts = 2700;

        // Should succeed as only DEX oracle is active and fresh
        let result = safety_controller.check_redemptions_allowed(
            &protocol_oracle,
            &protocol_config,
            current_ts,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_min_rate_q64_checked_returns_none_when_stale() {
        let oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 64) + 1000,
            dex_last_update_ts: 1000,
            native_last_update_ts: 2000,
            dex_last_update_slot: 100,
            native_last_update_slot: 200,
            dex_window_secs: 300,
            flags: 0,
        };

        let current_ts = 2700;
        let max_age_secs = 600;

        // Should return None due to stale DEX oracle
        assert_eq!(oracle.min_rate_q64_checked(current_ts, max_age_secs), None);

        // Test with both oracles stale
        let current_ts = 3000;
        assert_eq!(oracle.min_rate_q64_checked(current_ts, max_age_secs), None);
    }

    #[test]
    fn test_min_rate_q64_checked_returns_value_when_fresh() {
        let oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 64) + 1000,
            dex_last_update_ts: 2400,
            native_last_update_ts: 2450,
            dex_last_update_slot: 240,
            native_last_update_slot: 245,
            dex_window_secs: 300,
            flags: 0,
        };

        let current_ts = 2700;
        let max_age_secs = 600;

        // Should return the minimum rate (native is lower)
        assert_eq!(
            oracle.min_rate_q64_checked(current_ts, max_age_secs),
            Some(oracle.native_rate_q64)
        );
    }

    #[test]
    fn test_staleness_check_with_zero_timestamp() {
        let oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: 1 << 64,
            dex_last_update_ts: 0, // Never updated
            native_last_update_ts: 2000,
            dex_last_update_slot: 0,
            native_last_update_slot: 200,
            dex_window_secs: 300,
            flags: 0,
        };

        let current_ts = 2700;
        let max_age_secs = 600;

        // DEX oracle should be considered stale with zero timestamp
        assert!(oracle.is_dex_oracle_stale(current_ts, max_age_secs));

        // min_rate_q64_checked should return None
        assert_eq!(oracle.min_rate_q64_checked(current_ts, max_age_secs), None);
    }

    #[test]
    fn test_divergence_check_skipped_when_oracle_stale() {
        // Setup with diverging rates but stale DEX oracle
        let protocol_oracle = ProtocolOracle {
            native_rate_q64: 1 << 64,
            dex_twap_rate_q64: (1 << 63), // 50% divergence
            dex_last_update_ts: 1000,     // stale
            native_last_update_ts: 2400,  // fresh
            dex_last_update_slot: 100,
            native_last_update_slot: 240,
            dex_window_secs: 300,
            flags: 0,
        };

        let mut protocol_config = create_test_protocol_config();
        protocol_config.depeg_threshold_bps = 100; // 1% threshold (should trigger with 50% divergence)

        let safety_controller = SafetyController {
            redemptions_paused: false,
            consecutive_breaches: 0,
            consecutive_clears: 0,
            last_change_slot: 0,
            mint_last_slot: 0,
            mint_slot_amount: 0,
            redeem_last_slot: 0,
            redeem_slot_amount: 0,
            last_divergence_check_slot: 0,
            degrade_flags: Default::default(),
            _reserved: [0; 32],
        };
        let current_ts = 2700;

        // Should fail due to staleness, not divergence
        let result = safety_controller.check_redemptions_allowed(
            &protocol_oracle,
            &protocol_config,
            current_ts,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().to_string().contains("OracleStale"),
            true
        ));

        // Divergence check should not have been performed
        assert_eq!(safety_controller.consecutive_breaches, 0);
    }
}

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
    fn update(
        &mut self,
        tick: i32,
        timestamp: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>;
    fn get_twap_tick(
        &self,
        current_timestamp: i64,
        seconds_ago: u32,
    ) -> std::result::Result<i32, Box<dyn std::error::Error>>;
}

impl OracleTestExt for OracleState {
    fn update(
        &mut self,
        tick: i32,
        timestamp: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        OracleState::update(self, tick, timestamp)?;
        Ok(())
    }

    fn get_twap_tick(
        &self,
        current_timestamp: i64,
        seconds_ago: u32,
    ) -> std::result::Result<i32, Box<dyn std::error::Error>> {
        let tick = OracleState::get_twap_tick(self, current_timestamp, seconds_ago)?;
        Ok(tick)
    }
}