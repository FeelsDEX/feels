//! Unit tests for the initialize_hub instruction

use anchor_lang::prelude::*;
use feels::{constants::*, error::FeelsError, state::FeelsHub};
use solana_program::pubkey::Pubkey;

#[test]
fn test_hub_state_initialization() {
    let feelssol_mint = Pubkey::new_unique();

    let hub = FeelsHub {
        feelssol_mint,
        reentrancy_guard: false,
    };

    // Verify initialization state
    assert_eq!(hub.feelssol_mint, feelssol_mint);
    assert!(!hub.reentrancy_guard);
}

#[test]
fn test_hub_pda_derivation() {
    let program_id = feels::ID;
    let feelssol_mint = Pubkey::new_unique();

    // Test FeelsHub PDA
    let (hub_pda, hub_bump) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint.as_ref()], &program_id);
    assert!(hub_bump > 0);

    // Test JitoSOL vault PDA
    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[JITOSOL_VAULT_SEED, feelssol_mint.as_ref()], &program_id);
    assert!(vault_bump > 0);

    // Test vault authority PDA
    let (vault_authority, authority_bump) =
        Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED, feelssol_mint.as_ref()], &program_id);
    assert!(authority_bump > 0);

    // Verify PDAs are different
    assert_ne!(hub_pda, vault_pda);
    assert_ne!(hub_pda, vault_authority);
    assert_ne!(vault_pda, vault_authority);
}

#[test]
fn test_multiple_hubs_different_mints() {
    let program_id = feels::ID;
    let feelssol_mint1 = Pubkey::new_unique();
    let feelssol_mint2 = Pubkey::new_unique();

    // Different FeelsSOL mints should have different hub PDAs
    let (hub_pda1, _) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint1.as_ref()], &program_id);

    let (hub_pda2, _) =
        Pubkey::find_program_address(&[FEELS_HUB_SEED, feelssol_mint2.as_ref()], &program_id);

    assert_ne!(hub_pda1, hub_pda2);
}

#[test]
fn test_reentrancy_guard_initial_state() {
    let hub = FeelsHub {
        feelssol_mint: Pubkey::new_unique(),
        reentrancy_guard: false,
    };

    // Reentrancy guard should be false initially
    assert!(!hub.reentrancy_guard);

    // Test reentrancy check
    assert!(check_reentrancy(&hub).is_ok());

    // Test with guard set
    let mut hub_locked = hub;
    hub_locked.reentrancy_guard = true;
    assert_eq!(
        check_reentrancy(&hub_locked).unwrap_err(),
        FeelsError::ReentrancyDetected.into()
    );
}

#[test]
fn test_vault_authority_seeds() {
    let feelssol_mint = Pubkey::new_unique();

    // Verify vault authority can sign for vault
    let vault_authority_seeds = [VAULT_AUTHORITY_SEED, feelssol_mint.as_ref()];

    // Seeds should be consistent
    assert_eq!(vault_authority_seeds[0], VAULT_AUTHORITY_SEED);
    assert_eq!(vault_authority_seeds[1].len(), 32);
}

// Helper functions

fn check_reentrancy(hub: &FeelsHub) -> Result<()> {
    if hub.reentrancy_guard {
        return Err(FeelsError::ReentrancyDetected.into());
    }
    Ok(())
}
