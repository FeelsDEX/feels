use anchor_lang::prelude::*;

#[event]
pub struct TokenWrapperInitialized {
    pub underlying_mint: Pubkey,
    pub feels_protocol: Pubkey,
}
