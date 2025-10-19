//! Token metadata tracking for protocol-minted tokens
//!
//! Tracks token type + origin for future Token-2022 support

use anchor_lang::prelude::*;

/// Token type enum for tracking SPL vs Token-2022
#[derive(Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize, Default)]
pub enum TokenType {
    /// Standard SPL token
    #[default]
    Spl = 0,
    /// Token-2022 (not supported in MVP)
    Token2022 = 1,
}

/// Origin of a token
#[derive(Clone, Copy, Debug, PartialEq, AnchorSerialize, AnchorDeserialize, Default)]
pub enum TokenOrigin {
    /// Minted by the protocol using mint_token instruction
    ProtocolMinted = 0,
    /// External token (not allowed to create markets in MVP)
    #[default]
    External = 1,
    /// FeelsSOL hub token
    FeelsSOL = 2,
}

/// Registry entry for protocol-minted tokens
#[account]
pub struct ProtocolToken {
    /// Token mint address
    pub mint: Pubkey,

    /// Creator who minted the token
    pub creator: Pubkey,

    /// Token type (SPL or Token-2022)
    pub token_type: TokenType,

    /// Creation timestamp
    pub created_at: i64,

    /// Whether this token can create markets (for future use)
    pub can_create_markets: bool,

    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl ProtocolToken {
    pub const LEN: usize = 8 + // discriminator
        32 + // mint
        32 + // creator
        1 + // token_type
        8 + // created_at
        1 + // can_create_markets
        32 + // _reserved
        6; // padding added by Rust compiler for alignment
}
