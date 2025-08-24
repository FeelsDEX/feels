/// Token metadata and state structures for the Feels Protocol token creation system.
/// Defines TokenMetadata account structure for storing ticker, name, symbol, and creation info
/// for tokens created through the platform's token factory with ticker validation.

use anchor_lang::prelude::*;

// ============================================================================
// Token Metadata
// ============================================================================

/// Metadata for tokens created on the Feels platform
#[account]
pub struct TokenMetadata {
    /// Token ticker (e.g., "FEELS") - validated against restrictions
    pub ticker: String,
    
    /// Full token name (e.g., "Feel Something")
    pub name: String,
    
    /// Token symbol (usually same as ticker)
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

impl TokenMetadata {
    pub const SIZE: usize = 8 + // discriminator
        4 + 32 + // ticker (max 12 chars + length)
        4 + 128 + // name (max 32 chars + length) 
        4 + 32 + // symbol (max 12 chars + length)
        32 + // mint
        32 + // authority
        8 + // created_at
        64; // reserved
}