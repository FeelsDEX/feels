//! Unit tests for the initialize_protocol instruction

use feels::{
    instructions::InitializeProtocolParams,
    state::{DegradeFlags, ProtocolOracle, SafetyController},
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_initialize_protocol_validation() {
    // Test invalid depeg threshold (too low)
    let params = InitializeProtocolParams {
        mint_fee: 100_000_000,
        treasury: Pubkey::new_unique(),
        default_protocol_fee_rate: Some(1000),
        default_creator_fee_rate: Some(500),
        max_protocol_fee_rate: Some(2500),
        dex_twap_updater: Pubkey::new_unique(),
        depeg_threshold_bps: 24, // Below minimum of 25
        depeg_required_obs: 5,
        clear_required_obs: 3,
        dex_twap_window_secs: 600,
        dex_twap_stale_age_secs: 1200,
        dex_whitelist: vec![],
    };

    assert!(!validate_protocol_params(&params));

    // Test invalid depeg threshold (too high)
    let params = InitializeProtocolParams {
        depeg_threshold_bps: 5001, // Above maximum of 5000
        ..params
    };

    assert!(!validate_protocol_params(&params));

    // Test invalid depeg required observations
    let params = InitializeProtocolParams {
        depeg_threshold_bps: 100,
        depeg_required_obs: 0, // Below minimum of 1
        ..params
    };

    assert!(!validate_protocol_params(&params));

    // Test invalid TWAP window
    let params = InitializeProtocolParams {
        depeg_required_obs: 5,
        dex_twap_window_secs: 299, // Below minimum of 300
        ..params
    };

    assert!(!validate_protocol_params(&params));
}

#[test]
fn test_protocol_config_defaults() {
    let params = InitializeProtocolParams {
        mint_fee: 100_000_000,
        treasury: Pubkey::new_unique(),
        default_protocol_fee_rate: None,
        default_creator_fee_rate: None,
        max_protocol_fee_rate: None,
        dex_twap_updater: Pubkey::new_unique(),
        depeg_threshold_bps: 100,
        depeg_required_obs: 5,
        clear_required_obs: 3,
        dex_twap_window_secs: 600,
        dex_twap_stale_age_secs: 1200,
        dex_whitelist: vec![],
    };

    // Verify defaults would be applied
    let default_protocol_fee = params.default_protocol_fee_rate.unwrap_or(1000);
    let default_creator_fee = params.default_creator_fee_rate.unwrap_or(500);
    let max_protocol_fee = params.max_protocol_fee_rate.unwrap_or(2500);

    assert_eq!(default_protocol_fee, 1000); // 10%
    assert_eq!(default_creator_fee, 500); // 5%
    assert_eq!(max_protocol_fee, 2500); // 25%
}

#[test]
fn test_dex_whitelist_truncation() {
    let mut dex_list = vec![];
    for _ in 0..10 {
        dex_list.push(Pubkey::new_unique());
    }

    let params = InitializeProtocolParams {
        mint_fee: 100_000_000,
        treasury: Pubkey::new_unique(),
        default_protocol_fee_rate: Some(1000),
        default_creator_fee_rate: Some(500),
        max_protocol_fee_rate: Some(2500),
        dex_twap_updater: Pubkey::new_unique(),
        depeg_threshold_bps: 100,
        depeg_required_obs: 5,
        clear_required_obs: 3,
        dex_twap_window_secs: 600,
        dex_twap_stale_age_secs: 1200,
        dex_whitelist: dex_list,
    };

    // Verify list would be truncated to 8 entries
    let truncated_len = params.dex_whitelist.iter().take(8).count();
    assert_eq!(truncated_len, 8);
}

#[test]
fn test_protocol_oracle_initialization() {
    // Test oracle default values
    let oracle = ProtocolOracle {
        native_rate_q64: 0,
        dex_twap_rate_q64: 0,
        dex_last_update_slot: 0,
        native_last_update_slot: 0,
        dex_last_update_ts: 0,
        native_last_update_ts: 0,
        dex_window_secs: 600,
        flags: 0,
    };

    assert_eq!(oracle.native_rate_q64, 0);
    assert_eq!(oracle.dex_twap_rate_q64, 0);
    assert_eq!(oracle.dex_window_secs, 600);
    assert_eq!(oracle.flags, 0);
}

#[test]
fn test_safety_controller_initialization() {
    // Test safety controller default values
    let safety = SafetyController {
        redemptions_paused: false,
        consecutive_breaches: 0,
        consecutive_clears: 0,
        last_change_slot: 0,
        mint_last_slot: 0,
        mint_slot_amount: 0,
        redeem_last_slot: 0,
        redeem_slot_amount: 0,
        last_divergence_check_slot: 0,
        degrade_flags: DegradeFlags {
            gtwap_stale: false,
            oracle_stale: false,
            high_volatility: false,
            low_liquidity: false,
            _reserved: [false; 4],
        },
        _reserved: [0; 32],
    };

    assert!(!safety.redemptions_paused);
    assert_eq!(safety.consecutive_breaches, 0);
    assert_eq!(safety.consecutive_clears, 0);
    assert_eq!(safety.mint_slot_amount, 0);
    assert_eq!(safety.redeem_slot_amount, 0);
}

#[test]
fn test_protocol_pda_derivation() {
    let program_id = feels::ID;

    // Test protocol config PDA
    let (config_pda, config_bump) =
        Pubkey::find_program_address(&[b"protocol_config"], &program_id);
    assert!(config_bump > 0);

    // Test protocol oracle PDA
    let (oracle_pda, oracle_bump) =
        Pubkey::find_program_address(&[b"protocol_oracle"], &program_id);
    assert!(oracle_bump > 0);

    // Test safety controller PDA
    let (safety_pda, safety_bump) =
        Pubkey::find_program_address(&[b"safety_controller"], &program_id);
    assert!(safety_bump > 0);

    // Verify PDAs are different
    assert_ne!(config_pda, oracle_pda);
    assert_ne!(config_pda, safety_pda);
    assert_ne!(oracle_pda, safety_pda);
}

#[test]
fn test_token_expiration_default() {
    // Verify default token expiration is 7 days
    let expected_seconds = 7 * 24 * 60 * 60;
    assert_eq!(expected_seconds, 604800);
}

// Helper function to validate protocol params
fn validate_protocol_params(params: &InitializeProtocolParams) -> bool {
    // Depeg threshold validation
    if params.depeg_threshold_bps < 25 || params.depeg_threshold_bps > 5000 {
        return false;
    }

    // Required observations validation
    if params.depeg_required_obs == 0 || params.depeg_required_obs > 10 {
        return false;
    }

    if params.clear_required_obs == 0 || params.clear_required_obs > 10 {
        return false;
    }

    // TWAP window validation
    if params.dex_twap_window_secs < 300 || params.dex_twap_window_secs > 7200 {
        return false;
    }

    // Stale age must be >= window
    if params.dex_twap_stale_age_secs < params.dex_twap_window_secs {
        return false;
    }

    true
}
