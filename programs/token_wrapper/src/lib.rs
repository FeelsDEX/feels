#![allow(deprecated)]
#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod events;
pub mod instructions;
pub mod state;
#[cfg(test)]
mod tests;

use instructions::*;

// TODO: Update when we have the real ID
declare_id!("Fee1sTokenWrapper111111111111111111111111111");

#[program]
pub mod feels_token_wrapper {
    use super::*;

    /// Initialize the FeelsSOL token wrapper
    pub fn initialize(
        ctx: Context<Initialize>,
        underlying_mint: Pubkey,
        feels_protocol: Pubkey,
    ) -> Result<()> {
        instructions::initialize_feelssol(ctx, underlying_mint, feels_protocol)
    }
}
