use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Treasury {
    pub protocol: Pubkey,     // 32 - Reference to protocol state
    pub authority: Pubkey,    // 32 - Who can withdraw (usually protocol authority)
    pub total_collected: u64, // 8 - Total fees collected
    pub total_withdrawn: u64, // 8 - Total fees withdrawn
    pub last_withdrawal: i64, // 8 - Last withdrawal timestamp
    pub _reserved: [u8; 64],  // 64 - Reserved for future use
}
