use anchor_lang::prelude::*;

// State account
#[account]
pub struct ProtocolState {
    pub authority: Pubkey,              // 32
    pub treasury: Pubkey,               // 32
    pub default_protocol_fee_rate: u16, // 2
    pub max_pool_fee_rate: u16,         // 2
    pub paused: bool,                   // 1
    pub pool_creation_allowed: bool,    // 1
    pub total_pools: u64,               // 8
    pub total_fees_collected: u64,      // 8
    pub total_volume: u64,              // 8
    pub initialized_at: i64,            // 8
    pub last_updated: i64,              // 8
}

impl ProtocolState {
    pub const SIZE: usize = 8 + 32 + 32 + 2 + 2 + 1 + 1 + 8 + 8 + 8 + 8 + 8;
}
