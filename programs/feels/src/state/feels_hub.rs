use anchor_lang::prelude::*;

#[account]
pub struct FeelsHub {
    /// FeelsSOL mint this hub controls
    pub feelssol_mint: Pubkey,
    /// Reentrancy guard for mint/redeem flows
    pub reentrancy_guard: bool,
}

impl FeelsHub {
    pub const SEED: &'static [u8] = b"feels_hub";
    pub const LEN: usize = 8 + // disc
        32 + // feelssol_mint
        1;   // reentrancy_guard
}

