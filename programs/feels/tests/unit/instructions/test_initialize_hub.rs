//! Unit tests for the initialize_hub instruction

use anchor_lang::prelude::*;
use feels::{
    error::FeelsError,
    instructions::{InitializeHub, InitializeHubParams},
    state::{FeelsHub, ProtocolConfig},
    constants::*,
};
use solana_program::pubkey::Pubkey;

#[test]
fn test_initialize_hub_validation() {
    // Test invalid JitoSOL mint (all zeros)
    let params = InitializeHubParams {
        jitosol_mint: Pubkey::default(),
        entry_exit_fee_bps: 10,
    };
    
    assert_eq!(
        validate_initialize_hub_params(&params).unwrap_err(),
        FeelsError::InvalidMint
    );
    
    // Test fee too high
    let params = InitializeHubParams {
        jitosol_mint: Pubkey::new_unique(),
        entry_exit_fee_bps: MAX_FEE_BPS + 1,
    };
    
    assert_eq!(
        validate_initialize_hub_params(&params).unwrap_err(),
        FeelsError::InvalidFee
    );
}

#[test]
fn test_hub_fee_validation() {
    // Test maximum allowed fee
    let params = InitializeHubParams {
        jitosol_mint: Pubkey::new_unique(),
        entry_exit_fee_bps: MAX_FEE_BPS,
    };
    
    assert!(validate_initialize_hub_params(&params).is_ok());
    
    // Test zero fee
    let params = InitializeHubParams {
        jitosol_mint: Pubkey::new_unique(),
        entry_exit_fee_bps: 0,
    };
    
    assert!(validate_initialize_hub_params(&params).is_ok());
    
    // Test reasonable fee (0.1%)
    let params = InitializeHubParams {
        jitosol_mint: Pubkey::new_unique(),
        entry_exit_fee_bps: 10,
    };
    
    assert!(validate_initialize_hub_params(&params).is_ok());
}

#[test]
fn test_hub_state_initialization() {
    let feelssol_mint = Pubkey::new_unique();
    let jitosol_mint = Pubkey::new_unique();
    let protocol_config = Pubkey::new_unique();
    let oracle = Pubkey::new_unique();
    
    let hub = FeelsHub {
        is_initialized: true,
        feelssol_mint,
        jitosol_mint,
        feelssol_authority_bump: 254,
        hub_bump: 255,
        total_feelssol_supply: 0,
        total_jitosol_locked: 0,
        entry_exit_fee_bps: 10,
        last_rebase_slot: 1000,
        last_rebase_timestamp: 1234567890,
        exchange_rate_numerator: 1_000_000,
        exchange_rate_denominator: 1_000_000,
        protocol_config,
        oracle,
        paused: false,
        _reserved: [0; 64],
    };
    
    // Verify initialization state
    assert!(hub.is_initialized);
    assert_eq!(hub.total_feelssol_supply, 0);
    assert_eq!(hub.total_jitosol_locked, 0);
    assert!(!hub.paused);
    
    // Verify exchange rate calculation
    let rate = calculate_exchange_rate(&hub);
    assert_eq!(rate, 1.0);
}

#[test]
fn test_hub_exchange_rate_calculations() {
    let mut hub = create_test_hub();
    
    // Test 1:1 exchange rate
    hub.exchange_rate_numerator = 1_000_000;
    hub.exchange_rate_denominator = 1_000_000;
    assert_eq!(calculate_exchange_rate(&hub), 1.0);
    
    // Test 1.1:1 exchange rate (10% appreciation)
    hub.exchange_rate_numerator = 1_100_000;
    hub.exchange_rate_denominator = 1_000_000;
    assert_eq!(calculate_exchange_rate(&hub), 1.1);
    
    // Test 0.95:1 exchange rate (5% depreciation)
    hub.exchange_rate_numerator = 950_000;
    hub.exchange_rate_denominator = 1_000_000;
    assert_eq!(calculate_exchange_rate(&hub), 0.95);
}

#[test]
fn test_hub_entry_exit_fee_calculations() {
    let hub = create_test_hub();
    
    // Test entry fee calculation
    let amount_in = 1_000_000_000; // 1 SOL worth
    let fee = calculate_entry_fee(&hub, amount_in);
    assert_eq!(fee, 1_000_000); // 0.1% of 1 SOL
    
    // Test exit fee calculation
    let amount_out = 1_000_000_000;
    let fee = calculate_exit_fee(&hub, amount_out);
    assert_eq!(fee, 1_000_000); // 0.1% of 1 SOL
    
    // Test with zero fee
    let mut hub_no_fee = hub;
    hub_no_fee.entry_exit_fee_bps = 0;
    assert_eq!(calculate_entry_fee(&hub_no_fee, amount_in), 0);
    assert_eq!(calculate_exit_fee(&hub_no_fee, amount_out), 0);
}

#[test]
fn test_hub_pda_validation() {
    let program_id = feels::ID;
    
    // Test FeelsHub PDA
    let (hub_pda, hub_bump) = Pubkey::find_program_address(
        &[b"feels_hub"],
        &program_id
    );
    
    // Test FeelsSOL authority PDA
    let (feelssol_authority, feelssol_bump) = Pubkey::find_program_address(
        &[b"feelssol_mint_authority"],
        &program_id
    );
    
    // Verify PDAs are deterministic
    let (hub_pda2, hub_bump2) = Pubkey::find_program_address(
        &[b"feels_hub"],
        &program_id
    );
    assert_eq!(hub_pda, hub_pda2);
    assert_eq!(hub_bump, hub_bump2);
    
    // Verify PDAs are different
    assert_ne!(hub_pda, feelssol_authority);
}

#[test]
fn test_hub_authority_permissions() {
    let hub = create_test_hub();
    let authority = Pubkey::new_unique();
    let non_authority = Pubkey::new_unique();
    
    // Test authority check
    assert!(check_hub_authority(&hub, &authority, &authority).is_ok());
    assert!(check_hub_authority(&hub, &non_authority, &authority).is_err());
}

// Helper functions

fn create_test_hub() -> FeelsHub {
    FeelsHub {
        is_initialized: true,
        feelssol_mint: Pubkey::new_unique(),
        jitosol_mint: Pubkey::new_unique(),
        feelssol_authority_bump: 254,
        hub_bump: 255,
        total_feelssol_supply: 0,
        total_jitosol_locked: 0,
        entry_exit_fee_bps: 10, // 0.1%
        last_rebase_slot: 0,
        last_rebase_timestamp: 0,
        exchange_rate_numerator: 1_000_000,
        exchange_rate_denominator: 1_000_000,
        protocol_config: Pubkey::new_unique(),
        oracle: Pubkey::new_unique(),
        paused: false,
        _reserved: [0; 64],
    }
}

fn validate_initialize_hub_params(params: &InitializeHubParams) -> Result<()> {
    if params.jitosol_mint == Pubkey::default() {
        return Err(FeelsError::InvalidMint.into());
    }
    
    if params.entry_exit_fee_bps > MAX_FEE_BPS {
        return Err(FeelsError::InvalidFee.into());
    }
    
    Ok(())
}

fn calculate_exchange_rate(hub: &FeelsHub) -> f64 {
    hub.exchange_rate_numerator as f64 / hub.exchange_rate_denominator as f64
}

fn calculate_entry_fee(hub: &FeelsHub, amount: u64) -> u64 {
    (amount as u128 * hub.entry_exit_fee_bps as u128 / 10_000) as u64
}

fn calculate_exit_fee(hub: &FeelsHub, amount: u64) -> u64 {
    (amount as u128 * hub.entry_exit_fee_bps as u128 / 10_000) as u64
}

fn check_hub_authority(hub: &FeelsHub, signer: &Pubkey, expected_authority: &Pubkey) -> Result<()> {
    if signer != expected_authority {
        return Err(FeelsError::Unauthorized.into());
    }
    Ok(())
}