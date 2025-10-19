use anchor_lang::prelude::*;
use feels::state::ProtocolOracle;

#[test]
fn test_dex_oracle_staleness() {
    let current_ts = 1000000i64;
    let max_age_secs = 1800u32; // 30 minutes

    let mut oracle = ProtocolOracle {
        native_rate_q64: 1 << 64,
        dex_twap_rate_q64: (1 << 64) + 1000,
        dex_last_update_slot: 100,
        native_last_update_slot: 100,
        dex_last_update_ts: current_ts - 3600, // 1 hour ago (stale)
        native_last_update_ts: current_ts - 100, // recent
        dex_window_secs: 300,
        flags: 0,
    };

    // DEX oracle should be stale
    assert!(oracle.is_dex_oracle_stale(current_ts, max_age_secs));

    // Native oracle should be fresh
    assert!(!oracle.is_native_oracle_stale(current_ts, max_age_secs));

    // min_rate_q64_checked should return None because DEX is stale
    assert_eq!(oracle.min_rate_q64_checked(current_ts, max_age_secs), None);

    // Update DEX oracle to be fresh
    oracle.dex_last_update_ts = current_ts - 100;

    // Now both should be fresh
    assert!(!oracle.is_dex_oracle_stale(current_ts, max_age_secs));
    assert!(!oracle.is_native_oracle_stale(current_ts, max_age_secs));

    // min_rate_q64_checked should now return the minimum
    assert_eq!(
        oracle.min_rate_q64_checked(current_ts, max_age_secs),
        Some(oracle.native_rate_q64) // native is smaller
    );
}

#[test]
fn test_never_updated_oracle_is_stale() {
    let current_ts = 1000000i64;
    let max_age_secs = 1800u32;

    let oracle = ProtocolOracle {
        native_rate_q64: 1 << 64,
        dex_twap_rate_q64: 1 << 64,
        dex_last_update_slot: 0,
        native_last_update_slot: 0,
        dex_last_update_ts: 0,    // Never updated
        native_last_update_ts: 0, // Never updated
        dex_window_secs: 300,
        flags: 0,
    };

    // Both oracles should be considered stale
    assert!(oracle.is_dex_oracle_stale(current_ts, max_age_secs));
    assert!(oracle.is_native_oracle_stale(current_ts, max_age_secs));

    // min_rate_q64_checked should return None
    assert_eq!(oracle.min_rate_q64_checked(current_ts, max_age_secs), None);
}

#[test]
fn test_only_one_oracle_active() {
    let current_ts = 1000000i64;
    let max_age_secs = 1800u32;

    // Test when only DEX oracle is active (native rate is 0)
    let oracle_dex_only = ProtocolOracle {
        native_rate_q64: 0, // Not active
        dex_twap_rate_q64: 1 << 64,
        dex_last_update_slot: 100,
        native_last_update_slot: 0,
        dex_last_update_ts: current_ts - 100, // Fresh
        native_last_update_ts: 0,
        dex_window_secs: 300,
        flags: 0,
    };

    // Should use DEX rate when fresh
    assert_eq!(
        oracle_dex_only.min_rate_q64_checked(current_ts, max_age_secs),
        Some(1 << 64)
    );

    // Test when only native oracle is active (DEX rate is 0)
    let oracle_native_only = ProtocolOracle {
        native_rate_q64: 1 << 64,
        dex_twap_rate_q64: 0, // Not active
        dex_last_update_slot: 100,
        native_last_update_slot: 100,
        dex_last_update_ts: 0,
        native_last_update_ts: current_ts - 100, // Fresh
        dex_window_secs: 300,
        flags: 0,
    };

    // Should use native rate when fresh
    assert_eq!(
        oracle_native_only.min_rate_q64_checked(current_ts, max_age_secs),
        Some(1 << 64)
    );
}
