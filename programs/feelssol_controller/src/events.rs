use anchor_lang::prelude::*;

#[event]
pub struct FeelsSolInitialized {
    pub underlying_mint: Pubkey,
    pub feels_protocol: Pubkey,
}
