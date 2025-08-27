use anchor_lang::prelude::*;

#[event]
pub struct FeelsSolInitiated {
    pub underlying_mint: Pubkey,
    pub feels_protocol: Pubkey,
}
