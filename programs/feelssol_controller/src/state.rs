use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct FeelsSolController {
    pub underlying_mint: Pubkey,
    pub underlying_stake_pool: Pubkey,
    pub feels_mint: Pubkey,
    pub total_wrapped: u64,
    pub feels_protocol: Pubkey,
    pub _reserved: [u8; 64],
}
