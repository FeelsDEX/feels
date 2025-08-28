#![allow(deprecated)]
#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod error;
pub mod events;
pub mod instructions;
pub mod state;
#[cfg(test)]
mod tests;
pub mod token_validate;

use instructions::*;

// TODO: Update when we have the real ID
declare_id!("TokenFactory1111111111111111111111111111111");

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
        ticker: String,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<()> {
        instructions::create_token(ctx, ticker, name, symbol, decimals, initial_supply)
    }
}
