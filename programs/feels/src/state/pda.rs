/// Unified PDA (Program Derived Address) derivation module.
/// Single source of truth for all PDA seeds and derivations in the protocol.
use anchor_lang::prelude::*;

// ============================================================================
// PDA Seed Constants
// ============================================================================

pub const PROTOCOL_SEED: &[u8] = b"protocol";
pub const MARKET_FIELD_SEED: &[u8] = b"market_field";
pub const MARKET_MANAGER_SEED: &[u8] = b"manager";
pub const BUFFER_SEED: &[u8] = b"buffer";
pub const ROUTER_SEED: &[u8] = b"router";
pub const TWAP_ORACLE_SEED: &[u8] = b"twap";
pub const DATA_SOURCE_SEED: &[u8] = b"data_source";
pub const VAULT_SEED: &[u8] = b"vault";
pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
pub const KEEPER_REGISTRY_SEED: &[u8] = b"keeper_registry";
pub const FIELD_COMMITMENT_SEED: &[u8] = b"field_commitment";
pub const NUMERAIRE_SEED: &[u8] = b"numeraire";
pub const FEELSSOL_SEED: &[u8] = b"feelssol";
pub const FEELSSOL_VAULT_SEED: &[u8] = b"feelssol_vault";
pub const TOKEN_PRICE_ORACLE_SEED: &[u8] = b"token_price_oracle";
pub const VOLATILITY_ORACLE_SEED: &[u8] = b"volatility_oracle";
pub const VOLUME_TRACKER_SEED: &[u8] = b"volume_tracker";
pub const EMERGENCY_FLAGS_SEED: &[u8] = b"emergency_flags";
pub const POOL_STATUS_SEED: &[u8] = b"pool_status";
pub const FEES_POLICY_SEED: &[u8] = b"fees_policy";
pub const REBASE_ACCUMULATOR_SEED: &[u8] = b"rebase_accumulator";

// ============================================================================
// PDA Derivation Functions
// ============================================================================

/// Derive protocol state PDA
pub fn derive_protocol_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_SEED], program_id)
}

/// Derive market field PDA
pub fn derive_market_field_pda(pool: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MARKET_FIELD_SEED, pool.as_ref()], program_id)
}

/// Derive market manager PDA
pub fn derive_market_manager_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MARKET_MANAGER_SEED, market_field.as_ref()], program_id)
}

/// Derive buffer PDA
pub fn derive_buffer_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BUFFER_SEED, market_field.as_ref()], program_id)
}

/// Derive router PDA
pub fn derive_router_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROUTER_SEED, market_field.as_ref()], program_id)
}

/// Derive TWAP oracle PDA
pub fn derive_twap_oracle_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TWAP_ORACLE_SEED, market_field.as_ref()], program_id)
}

/// Derive market data source PDA
pub fn derive_data_source_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DATA_SOURCE_SEED, market_field.as_ref()], program_id)
}

/// Derive vault PDA
pub fn derive_vault_pda(market_field: &Pubkey, token_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, market_field.as_ref(), token_mint.as_ref()], program_id)
}

// ============================================================================
// Canonical Token Ordering for Pool PDAs
// ============================================================================

/// Sort two token mints into canonical order for consistent PDA derivation
/// Returns (token_0, token_1) where token_0 < token_1 by byte comparison
pub fn sort_token_mints(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, Pubkey) {
    use std::cmp::Ordering;
    match mint_a.as_ref().cmp(mint_b.as_ref()) {
        Ordering::Less => (*mint_a, *mint_b),
        Ordering::Greater => (*mint_b, *mint_a),
        Ordering::Equal => (*mint_a, *mint_b), // Same token, though this shouldn't happen
    }
}

/// Derive pool PDA with canonical seed ordering
/// This ensures only one pool can exist for any token pair regardless of input order
pub fn derive_pool_pda(
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    fee_rate: u16,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let (token_a_sorted, token_b_sorted) = sort_token_mints(mint_a, mint_b);

    Pubkey::find_program_address(
        &[
            b"pool",
            token_a_sorted.as_ref(),
            token_b_sorted.as_ref(),
            &fee_rate.to_le_bytes(),
        ],
        program_id,
    )
}

/// Get pool seeds for PDA signing
/// Returns the seeds in the correct format for CPI calls
pub fn get_pool_seeds(
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    fee_rate: u16,
    bump: u8,
) -> Vec<Vec<u8>> {
    let (token_a_sorted, token_b_sorted) = sort_token_mints(mint_a, mint_b);
    
    vec![
        b"pool".to_vec(),
        token_a_sorted.as_ref().to_vec(),
        token_b_sorted.as_ref().to_vec(),
        fee_rate.to_le_bytes().to_vec(),
        vec![bump],
    ]
}

/// Derive tick array PDA
pub fn derive_tick_array_pda(market_field: &Pubkey, start_tick: i32, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TICK_ARRAY_SEED, market_field.as_ref(), &start_tick.to_le_bytes()], 
        program_id
    )
}

/// Derive keeper registry PDA
pub fn derive_keeper_registry_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[KEEPER_REGISTRY_SEED], program_id)
}

/// Derive field commitment PDA
pub fn derive_field_commitment_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FIELD_COMMITMENT_SEED, market_field.as_ref()], program_id)
}

/// Derive numeraire PDA
pub fn derive_numeraire_pda(authority: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[NUMERAIRE_SEED, authority.as_ref()], program_id)
}

/// Derive FeelsSOL mint PDA
pub fn derive_feelssol_mint_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FEELSSOL_SEED], program_id)
}

/// Derive FeelsSOL vault PDA
pub fn derive_feelssol_vault_pda(underlying_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FEELSSOL_VAULT_SEED, underlying_mint.as_ref()], program_id)
}

/// Derive token price oracle PDA
pub fn derive_token_price_oracle_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_PRICE_ORACLE_SEED, market_field.as_ref()], program_id)
}

/// Derive volatility oracle PDA
pub fn derive_volatility_oracle_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VOLATILITY_ORACLE_SEED, market_field.as_ref()], program_id)
}

/// Derive volume tracker PDA
pub fn derive_volume_tracker_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VOLUME_TRACKER_SEED, market_field.as_ref()], program_id)
}

/// Derive emergency flags PDA
pub fn derive_emergency_flags_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EMERGENCY_FLAGS_SEED, market_field.as_ref()], program_id)
}

/// Derive pool status PDA
pub fn derive_pool_status_pda(market_field: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_STATUS_SEED, market_field.as_ref()], program_id)
}

/// Derive fees policy PDA
pub fn derive_fees_policy_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FEES_POLICY_SEED], program_id)
}

/// Derive rebase accumulator PDA
pub fn derive_rebase_accumulator_pda(pool: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REBASE_ACCUMULATOR_SEED, pool.as_ref()], program_id)
}