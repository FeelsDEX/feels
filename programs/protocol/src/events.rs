use anchor_lang::prelude::*;

#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub default_protocol_fee_rate: u16,
    pub max_pool_fee_rate: u16,
}

#[event]
pub struct ProtocolUpdated {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub default_protocol_fee_rate: u16,
    pub max_pool_fee_rate: u16,
    pub paused: bool,
    pub pool_creation_allowed: bool,
}

#[event]
pub struct AuthorityTransferInitiated {
    pub current_authority: Pubkey,
    pub new_authority: Pubkey,
    pub initiated_at: i64,
    pub can_be_accepted_at: i64,
}

#[event]
pub struct AuthorityTransferCancelled {
    pub current_authority: Pubkey,
    pub cancelled_authority: Pubkey,
    pub cancelled_at: i64,
}

#[event]
pub struct AuthorityTransferAccepted {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
    pub accepted_at: i64,
}
