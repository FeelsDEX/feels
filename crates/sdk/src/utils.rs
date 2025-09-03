use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;

/// Derive the protocol state PDA
pub fn derive_protocol_state(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"protocol"], program_id)
}

/// Derive the FeelsSOL state PDA
pub fn derive_feelssol_state(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"feelssol"], program_id)
}

/// Derive a pool PDA
pub fn derive_pool(
    token_0: &Pubkey,
    token_1: &Pubkey,
    fee_rate: u16,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let fee_bytes = fee_rate.to_le_bytes();
    Pubkey::find_program_address(
        &[
            b"pool",
            token_0.as_ref(),
            token_1.as_ref(),
            fee_bytes.as_ref(),
        ],
        program_id,
    )
}

/// Derive a vault PDA for a pool
pub fn derive_vault(pool: &Pubkey, token_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", pool.as_ref(), token_mint.as_ref()], program_id)
}

/// Derive a position PDA
pub fn derive_position(pool: &Pubkey, position_mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"position", pool.as_ref(), position_mint.as_ref()],
        program_id,
    )
}

/// Derive a tick array PDA
pub fn derive_tick_array(
    pool: &Pubkey,
    start_tick_index: i32,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let tick_bytes = start_tick_index.to_le_bytes();
    Pubkey::find_program_address(
        &[b"tick_array", pool.as_ref(), tick_bytes.as_ref()],
        program_id,
    )
}

/// Get the associated token account for Token-2022
pub fn get_token_2022_account(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(wallet, mint, &spl_token_2022::ID)
}

/// Calculate the start tick index for a tick array
pub fn get_tick_array_start_index(tick: i32, tick_spacing: i32) -> i32 {
    let tick_array_size = 88; // Standard tick array size
    let ticks_per_array = tick_array_size * tick_spacing;
    (tick / ticks_per_array) * ticks_per_array
}

/// Sort tokens to ensure consistent pool derivation
pub fn sort_tokens(token_0: Pubkey, token_1: Pubkey) -> (Pubkey, Pubkey) {
    if token_0 < token_1 {
        (token_0, token_1)
    } else {
        (token_1, token_0)
    }
}
