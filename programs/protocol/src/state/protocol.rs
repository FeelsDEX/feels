use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolState {
    pub authority: Pubkey, // 32 - Who can modify the protocol state and withdraw
    pub treasury: Pubkey,  // 32 - Reference to the treasury account
    pub token_factory: Pubkey, // 32 - Reference to the token factory account
    pub feelssol_controller: Pubkey, // 32 - Reference to the FeelsSOL controller account
    pub default_protocol_fee_rate: u16, // 2 - Default fee rate for the protocol
    pub max_pool_fee_rate: u16, // 2 - Maximum fee rate for any pool
    pub paused: bool,      // 1 - Is the protocol paused
    pub pool_creation_allowed: bool, // 1 - Can new pools be created
    pub initialized_at: i64, // 8 - Timestamp when the protocol was initialized
    pub last_updated: i64, // 8 - Timestamp when the protocol was last updated
    pub pending_authority: Option<Pubkey>, // 33 (1 + 32) - Authority pending approval for transfer
    pub authority_transfer_initiated_at: Option<i64>, // 9 (1 + 8) - Timestamp when authority transfer was initiated
    pub _reserved: [u8; 64],                          // 64 - Reserved for future use
}
