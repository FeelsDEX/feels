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
declare_id!("Fee1sSo1Contro11er11111111111111111111111111");

#[program]
pub mod feelssol_controller {
    use super::*;

    /// Initialize the FeelsSOL token controller
    pub fn initialize(
        ctx: Context<Initialize>,
        underlying_mint: Pubkey,
        underlying_stake_pool: Pubkey,
        feels_protocol: Pubkey,
    ) -> Result<()> {
        instructions::initialize_feelssol(
            ctx,
            underlying_mint,
            underlying_stake_pool,
            feels_protocol,
        )
    }
}
