use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct TokenFactory {
    pub total_tokens: u64,      // 8 - Total number of tokens created
    pub feels_protocol: Pubkey, // 32 - Feels protocol authority
    pub _reserved: [u8; 64],    // 64 - Reserved for future use
}
