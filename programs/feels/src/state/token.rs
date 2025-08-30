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

// ============================================================================
// FeelsSOL Wrapper
// ============================================================================

/// FeelsSOL wrapper account for the protocol's native token
#[account]
pub struct FeelsSOL {
    /// Underlying token mint that backs FeelsSOL
    pub underlying_mint: Pubkey,
    
    /// The Feels mint this wrapper controls
    pub feels_mint: Pubkey,
    
    /// Vault holding the underlying tokens
    pub vault: Pubkey,
    
    /// Authority that can update the wrapper
    pub authority: Pubkey,
    
    /// Exchange rate (9 decimals) - how many underlying tokens per FeelsSOL
    pub exchange_rate: u64,
    
    /// Total supply of wrapped FeelsSOL
    pub total_supply: u64,
    
    /// Total underlying tokens in the vault
    pub total_underlying: u64,
    
    /// Last time yield was updated
    pub last_yield_update: i64,
    
    /// Cumulative yield earned (9 decimals)
    pub cumulative_yield: u64,
    
    /// Yield rate per second (9 decimals)
    pub yield_rate_per_second: u64,
    
    /// Whether the wrapper is paused
    pub is_paused: bool,
    
    /// Whether deposits are paused
    pub deposits_paused: bool,
    
    /// Whether withdrawals are paused
    pub withdrawals_paused: bool,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub last_updated_at: i64,
    
    /// Reserved for future use
    pub _reserved: [u8; 64],
}

impl FeelsSOL {
    pub const SIZE: usize = 8 + // discriminator
        32 + // underlying_mint
        32 + // feels_mint
        32 + // vault
        32 + // authority
        8 + // exchange_rate
        8 + // total_supply
        8 + // total_underlying
        8 + // last_yield_update
        8 + // cumulative_yield
        8 + // yield_rate_per_second
        1 + // is_paused
        1 + // deposits_paused
        1 + // withdrawals_paused
        8 + // created_at
        8 + // last_updated_at
        64; // reserved
}
