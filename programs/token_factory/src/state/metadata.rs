use anchor_lang::prelude::*;

/// Metadata for tokens created on the Feels platform
#[account]
#[derive(InitSpace)]
pub struct TokenMetadata {
    /// Token ticker (e.g., "FEELS") - validated against restrictions
    #[max_len(12)]
    pub ticker: String,

    /// Full token name (e.g., "Feel Something")
    #[max_len(32)]
    pub name: String,

    /// Token symbol (usually same as ticker)
    #[max_len(12)]
    pub symbol: String,

    /// Token mint account
    pub mint: Pubkey,

    /// Create authority (original creator)
    pub authority: Pubkey,

    /// Create timestamp
    pub created_at: i64,

    /// Reserved space for future fields
    pub _reserved: [u8; 64],
}
