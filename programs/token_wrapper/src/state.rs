use anchor_lang::prelude::*;

#[account]
pub struct FeelsSOLWrapper {
    pub underlying_mint: Pubkey, // 32 - LST being wrapped (e.g. JitoSOL)
    pub total_wrapped: u128,     // 16 - Total LST wrapped
    pub virtual_reserves: u128,  // 16 - Virtual balance for AMM
    pub yield_accumulator: u128, // 16 - Accumulated staking yield
    pub last_update_slot: u64,   // 8 - Last yield update
    pub feels_protocol: Pubkey,  // 32 - Feels protocol authority
}

impl FeelsSOLWrapper {
    pub const SIZE: usize = 8 + 32 + 16 + 16 + 16 + 8 + 32;
}
