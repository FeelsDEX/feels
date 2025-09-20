use anchor_lang::prelude::*;

/// Program ID for Feels Protocol
pub const PROGRAM_ID: &str = "Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N";

/// Get the program ID as a Pubkey
pub fn program_id() -> Pubkey {
    PROGRAM_ID.parse().unwrap()
}

/// Seeds for common PDAs
pub mod seeds {
    pub const MARKET: &[u8] = b"market";
    pub const BUFFER: &[u8] = b"buffer";
    pub const VAULT_AUTHORITY: &[u8] = b"vault_authority";
    pub const PROTOCOL_CONFIG: &[u8] = b"protocol_config";
    pub const PROTOCOL_ORACLE: &[u8] = b"protocol_oracle";
    pub const FEELS_HUB: &[u8] = b"feels_hub";
    pub const FEELS_MINT: &[u8] = b"feels_mint";
    pub const ORACLE: &[u8] = b"oracle";
    pub const TICK_ARRAY: &[u8] = b"tick_array";
    pub const POSITION: &[u8] = b"position";
    pub const POSITION_METADATA: &[u8] = b"position_metadata";
}

/// Protocol constants
pub const TICK_ARRAY_SIZE: i32 = 64;
pub const MAX_TICK: i32 = 309120;
pub const MIN_TICK: i32 = -309120;
pub const MAX_SQRT_PRICE: u128 = 184467440737095516;
pub const MIN_SQRT_PRICE: u128 = 1844674407370;