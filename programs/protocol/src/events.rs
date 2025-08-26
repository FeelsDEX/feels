use anchor_lang::prelude::*;

#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub default_protocol_fee_rate: u16,
    pub max_pool_fee_rate: u16,
}
