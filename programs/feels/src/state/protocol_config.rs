//! Protocol configuration state
//! 
//! Global protocol parameters that can be updated by governance

use anchor_lang::prelude::*;

/// Protocol configuration account
#[account]
pub struct ProtocolConfig {
    /// Authority that can update protocol parameters
    pub authority: Pubkey,
    
    /// Fee for minting a new token (in FeelsSOL lamports)
    pub mint_fee: u64,
    
    /// Treasury account to receive protocol fees
    pub treasury: Pubkey,
    
    /// Time window (in seconds) for deploying liquidity after token mint
    /// If liquidity isn't deployed within this window, token can be destroyed
    pub token_expiration_seconds: i64,
    
    /// Reserved for future protocol parameters
    pub _reserved: [u8; 24],
}

impl ProtocolConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        8 +  // mint_fee
        32 + // treasury
        8 +  // token_expiration_seconds
        24;  // _reserved
    
    /// Seed for deriving the protocol config PDA
    pub const SEED: &'static [u8] = b"protocol_config";
}