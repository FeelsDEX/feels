//! Pre-launch escrow state
//! 
//! Temporary escrow that holds newly minted tokens and mint fees
//! before market initialization

use anchor_lang::prelude::*;

/// Pre-launch escrow account for newly minted tokens
/// This temporary account holds tokens and mint fees until market goes live
#[account]
pub struct PreLaunchEscrow {
    /// Token mint this escrow is for
    pub token_mint: Pubkey,
    
    /// Creator who minted the token
    pub creator: Pubkey,
    
    /// FeelsSOL mint (for reference)
    pub feelssol_mint: Pubkey,
    
    /// Creation timestamp (used for expiration)
    pub created_at: i64,
    
    /// Associated market (set when market is initialized)
    pub market: Pubkey,
    
    /// Canonical bump for escrow authority PDA
    pub escrow_authority_bump: u8,
    
    /// Reserved space for future expansion
    pub _reserved: [u8; 128],
}

impl PreLaunchEscrow {
    pub const LEN: usize = 8 + // discriminator
        32 + // token_mint
        32 + // creator
        32 + // feelssol_mint
        8 +  // created_at
        32 + // market
        1 +  // escrow_authority_bump
        128; // _reserved
}