use anchor_lang::prelude::*;

#[account]
pub struct ProtocolState {
    pub authority: Pubkey,                            // 32 - Who can modify the protocol state and withdraw
    pub treasury: Pubkey,                             // 32 - Reference to the treasury account
    pub default_protocol_fee_rate: u16,               // 2 - Default fee rate for the protocol
    pub max_pool_fee_rate: u16,                       // 2 - Maximum fee rate for any pool
    pub paused: bool,                                 // 1 - Is the protocol paused
    pub pool_creation_allowed: bool,                  // 1 - Can new pools be created
    pub total_pools: u64,                             // 8 - Total number of pools
    pub total_fees_collected: u64,                    // 8 - Total fees collected by the protocol
    pub total_volume: u64,                            // 8 - Total trading volume across all pools
    pub initialized_at: i64,                          // 8 - Timestamp when the protocol was initialized
    pub last_updated: i64,                            // 8 - Timestamp when the protocol was last updated
    pub pending_authority: Option<Pubkey>,            // 33 (1 + 32) - Authority pending approval for transfer
    pub authority_transfer_initiated_at: Option<i64>, // 9 (1 + 8) - Timestamp when authority transfer was initiated
}

impl ProtocolState {
    pub const SIZE: usize = 8 + 32 + 32 + 2 + 2 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 33 + 9;
}
