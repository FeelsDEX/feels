use anchor_lang::prelude::*;

#[event]
pub struct TokenFactoryInitialized {
    pub feels_protocol: Pubkey,
}

#[event]
pub struct TokenCreated {
    pub mint: Pubkey,
    pub decimals: u8,
    pub initial_supply: u64,
}
