use anchor_lang::prelude::*;

#[account]
pub struct Treasury {
    pub protocol: Pubkey,             // 32 - Reference to protocol state
    pub authority: Pubkey,            // 32 - Who can withdraw (usually protocol authority)
    pub total_collected: u64,         // 8 - Total fees collected
    pub total_withdrawn: u64,         // 8 - Total fees withdrawn
    pub last_withdrawal: i64,         // 8 - Last withdrawal timestamp
    pub current_epoch_withdrawn: u64, // 8 - Amount withdrawn this epoch
}

impl Treasury {
    pub const SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 32;
}
