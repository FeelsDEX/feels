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
declare_id!("BjBR6qczDV4iimwRq6PTEF9sqjoju8GgAytqi5ATrSes");

#[program]
pub mod feelssol_controller {
    use super::*;

    /// Initialize the FeelsSOL token controller
    pub fn initialize(
        ctx: Context<Initialize>,
        underlying_mint: Pubkey,
        keeper: Pubkey,
        feels_protocol: Pubkey,
    ) -> Result<()> {
        instructions::initialize_feelssol(ctx, underlying_mint, keeper, feels_protocol)
    }

    /// Deposit underlying LST assets to get feelsSOL
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    /// Withdraw feelsSOL and receive underlying LST assets
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }
}
