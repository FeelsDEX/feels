//! Tests for stale oracle protection mechanisms

#[cfg(test)]
mod test_stale_oracle_protection {
    use anchor_lang::prelude::*;
    use feels::state::{ProtocolConfig, ProtocolOracle, SafetyController};

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
