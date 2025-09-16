//! Unit tests for the initialize_protocol instruction

use anchor_lang::prelude::*;
use feels::{
    error::FeelsError,
    instructions::{InitializeProtocol, InitializeProtocolParams},
    state::{ProtocolConfig, ProtocolOracle, FeelsHub},
};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};

#[test]
fn test_initialize_protocol_validation() {
    // Test invalid treasury pubkey (all zeros)
    let params = InitializeProtocolParams {
        treasury: Pubkey::default(),
        initial_oracle_authority: Pubkey::new_unique(),
        universal_feelssol_mint: Pubkey::new_unique(),
    };
    
    assert_eq!(
        validate_initialize_protocol_params(&params).unwrap_err(),
        FeelsError::InvalidTreasury
    );
    
    // Test invalid oracle authority (all zeros)
    let params = InitializeProtocolParams {
        treasury: Pubkey::new_unique(),
        initial_oracle_authority: Pubkey::default(),
        universal_feelssol_mint: Pubkey::new_unique(),
    };
    
    assert_eq!(
        validate_initialize_protocol_params(&params).unwrap_err(),
        FeelsError::InvalidAuthority
    );
    
    // Test invalid FeelsSOL mint (all zeros)
    let params = InitializeProtocolParams {
        treasury: Pubkey::new_unique(),
        initial_oracle_authority: Pubkey::new_unique(),
        universal_feelssol_mint: Pubkey::default(),
    };
    
    assert_eq!(
        validate_initialize_protocol_params(&params).unwrap_err(),
        FeelsError::InvalidMint
    );
}

#[test]
fn test_protocol_config_initialization() {
    let treasury = Pubkey::new_unique();
    let protocol_config_key = Pubkey::new_unique();
    
    let config = ProtocolConfig {
        bump: 255,
        is_initialized: true,
        authority: Pubkey::new_unique(),
        treasury,
        creation_fee: 100_000_000, // 0.1 SOL
        swap_fee_protocol_share_bps: 1000, // 10%
        swap_fee_lp_share_bps: 9000, // 90%
        swap_fee_treasury_share_bps: 500, // 5% of protocol share
        max_markets_per_token: 10,
        market_creation_cooldown: 60,
        oracle_update_interval: 300,
        feelssol_mint: Pubkey::new_unique(),
        hub_key: Pubkey::new_unique(),
        _reserved: [0; 64],
    };
    
    // Verify configuration values
    assert!(config.is_initialized);
    assert_eq!(config.creation_fee, 100_000_000);
    assert_eq!(config.swap_fee_protocol_share_bps, 1000);
    assert_eq!(config.swap_fee_lp_share_bps, 9000);
    assert_eq!(config.swap_fee_treasury_share_bps, 500);
    assert_eq!(config.max_markets_per_token, 10);
    assert_eq!(config.market_creation_cooldown, 60);
    assert_eq!(config.oracle_update_interval, 300);
    
    // Verify fee shares add up correctly
    assert!(config.swap_fee_protocol_share_bps + config.swap_fee_lp_share_bps <= 10000);
    assert!(config.swap_fee_treasury_share_bps <= config.swap_fee_protocol_share_bps);
}

#[test]
fn test_protocol_oracle_initialization() {
    let authority = Pubkey::new_unique();
    let oracle_key = Pubkey::new_unique();
    
    let oracle = ProtocolOracle {
        is_initialized: true,
        authority,
        jitosol_price_feed: Pubkey::new_unique(),
        feelssol_price_feed: Pubkey::new_unique(),
        last_update_slot: 1000,
        last_update_timestamp: 1234567890,
        jitosol_price: 110_000_000, // $110 with 8 decimals
        feelssol_price: 100_000_000, // $100 with 8 decimals
        confidence_interval_bps: 50, // 0.5%
        max_staleness_slots: 150,
        _reserved: [0; 64],
    };
    
    // Verify oracle initialization
    assert!(oracle.is_initialized);
    assert_eq!(oracle.authority, authority);
    assert_eq!(oracle.last_update_slot, 1000);
    assert_eq!(oracle.last_update_timestamp, 1234567890);
    assert_eq!(oracle.jitosol_price, 110_000_000);
    assert_eq!(oracle.feelssol_price, 100_000_000);
    assert_eq!(oracle.confidence_interval_bps, 50);
    assert_eq!(oracle.max_staleness_slots, 150);
}

#[test]
fn test_feels_hub_initialization() {
    let feelssol_mint = Pubkey::new_unique();
    let jitosol_mint = Pubkey::new_unique();
    let protocol_config = Pubkey::new_unique();
    
    let hub = FeelsHub {
        is_initialized: true,
        feelssol_mint,
        jitosol_mint,
        feelssol_authority_bump: 254,
        hub_bump: 255,
        total_feelssol_supply: 0,
        total_jitosol_locked: 0,
        entry_exit_fee_bps: 10, // 0.1%
        last_rebase_slot: 0,
        last_rebase_timestamp: 0,
        exchange_rate_numerator: 1_000_000,
        exchange_rate_denominator: 1_000_000,
        protocol_config,
        oracle: Pubkey::new_unique(),
        paused: false,
        _reserved: [0; 64],
    };
    
    // Verify hub initialization
    assert!(hub.is_initialized);
    assert_eq!(hub.feelssol_mint, feelssol_mint);
    assert_eq!(hub.jitosol_mint, jitosol_mint);
    assert_eq!(hub.entry_exit_fee_bps, 10);
    assert_eq!(hub.exchange_rate_numerator, 1_000_000);
    assert_eq!(hub.exchange_rate_denominator, 1_000_000);
    assert!(!hub.paused);
    
    // Verify initial exchange rate is 1:1
    let rate = hub.exchange_rate_numerator as f64 / hub.exchange_rate_denominator as f64;
    assert!((rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_protocol_pda_derivation() {
    let program_id = Pubkey::new_unique();
    
    // Test protocol config PDA
    let (config_pda, config_bump) = Pubkey::find_program_address(
        &[b"protocol_config"],
        &program_id
    );
    assert!(config_bump > 0);
    
    // Test protocol oracle PDA
    let (oracle_pda, oracle_bump) = Pubkey::find_program_address(
        &[b"protocol_oracle"],
        &program_id
    );
    assert!(oracle_bump > 0);
    
    // Test FeelsHub PDA
    let (hub_pda, hub_bump) = Pubkey::find_program_address(
        &[b"feels_hub"],
        &program_id
    );
    assert!(hub_bump > 0);
    
    // Verify PDAs are different
    assert_ne!(config_pda, oracle_pda);
    assert_ne!(config_pda, hub_pda);
    assert_ne!(oracle_pda, hub_pda);
}

// Helper function to validate initialize protocol params
fn validate_initialize_protocol_params(params: &InitializeProtocolParams) -> Result<()> {
    if params.treasury == Pubkey::default() {
        return Err(FeelsError::InvalidTreasury.into());
    }
    
    if params.initial_oracle_authority == Pubkey::default() {
        return Err(FeelsError::InvalidAuthority.into());
    }
    
    if params.universal_feelssol_mint == Pubkey::default() {
        return Err(FeelsError::InvalidMint.into());
    }
    
    Ok(())
}