use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct FeelsSolController {
    pub underlying_mint: Pubkey, // 32 - LST being wrapped (e.g. JitoSOL)
    pub total_wrapped: u128,     // 16 - Total LST wrapped
    pub virtual_reserves: u128,  // 16 - Virtual balance for AMM
    pub yield_accumulator: u128, // 16 - Accumulated staking yield
    pub last_update_slot: u64,   // 8 - Last yield update
    pub feels_protocol: Pubkey,  // 32 - Feels protocol authority
    pub _reserved: [u8; 64],     // 64 - Reserved for future use
}
