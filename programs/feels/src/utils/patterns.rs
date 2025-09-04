/// Essential patterns for Feels Protocol instructions
/// 
/// Provides streamlined patterns for account validation, instruction execution,
/// and common parameter checking used throughout the thermodynamic AMM.
use anchor_lang::prelude::*;
use crate::error::FeelsError;

// ============================================================================
// Essential Validation Helpers
// ============================================================================

/// Validate non-zero amount
pub fn validate_amount(amount: u64) -> Result<()> {
    require!(amount > 0, FeelsError::InvalidAmount);
    Ok(())
}

/// Validate basis points value (0-10000)
pub fn validate_bps(bps: u16) -> Result<()> {
    require!(bps <= 10_000, FeelsError::InvalidParameter);
    Ok(())
}


// ============================================================================
// Simple Instruction Phases
// ============================================================================

/// Standard instruction execution phases without complex macros
pub fn execute_with_phases<F1, F2, F3, R>(
    validate: F1,
    execute: F2,
    finalize: F3,
) -> Result<R>
where
    F1: FnOnce() -> Result<()>,
    F2: FnOnce() -> Result<R>,
    F3: FnOnce(&R) -> Result<()>,
{
    // Phase 1: Validation
    msg!("Phase 1: Validating inputs");
    validate()?;
    
    // Phase 2: Core execution
    msg!("Phase 2: Executing logic");
    let result = execute()?;
    
    // Phase 3: Finalization
    msg!("Phase 3: Finalizing");
    finalize(&result)?;
    
    Ok(result)
}

// ============================================================================
// Common Account Validation Patterns
// ============================================================================

/// Validate token account ownership
pub fn validate_token_account_owner(
    token_account: &anchor_spl::token_interface::TokenAccount,
    expected_owner: &Pubkey,
) -> Result<()> {
    require!(
        token_account.owner == *expected_owner,
        FeelsError::InvalidOwner
    );
    Ok(())
}

/// Validate token account mint
pub fn validate_token_account_mint(
    token_account: &anchor_spl::token_interface::TokenAccount,
    expected_mint: &Pubkey,
) -> Result<()> {
    require!(
        token_account.mint == *expected_mint,
        FeelsError::InvalidMint
    );
    Ok(())
}

// ============================================================================
// PDA Validation Helpers
// ============================================================================

/// Validate PDA derivation
pub fn validate_pda(
    account: &Pubkey,
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<()> {
    let (derived_pda, _bump) = Pubkey::find_program_address(seeds, program_id);
    require!(
        *account == derived_pda,
        FeelsError::InvalidPDA
    );
    Ok(())
}