#![allow(deprecated)]
#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod error;
pub mod events;
pub mod instructions;
pub mod state;
#[cfg(test)]
mod tests;

use instructions::*;

// TODO: Update when we have the real ID
declare_id!("49nnQKfdGZoksCFg3ZTdStvyaMgptmEkDY77oaMpG2Hd");

#[program]
pub mod feels_token_factory {
    use super::*;

    /// Initialize the token factory
    pub fn initialize(ctx: Context<Initialize>, feels_protocol: Pubkey) -> Result<()> {
        instructions::initialize_token_factory(ctx, feels_protocol)
    }

    /// Create a token
    pub fn create_token(
        ctx: Context<CreateToken>,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<()> {
        instructions::create_token(ctx, decimals, initial_supply)
    }
}
