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
declare_id!("Fee1sProtoco11111111111111111111111111111111");

#[program]
pub mod feels_protocol {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        default_protocol_fee_rate: u16,
        max_pool_fee_rate: u16,
    ) -> Result<()> {
        instructions::initialize_protocol(ctx, default_protocol_fee_rate, max_pool_fee_rate)
    }

    pub fn update_protocol(
        ctx: Context<UpdateProtocol>,
        new_default_protocol_fee_rate: Option<u16>,
        new_max_pool_fee_rate: Option<u16>,
        new_paused: Option<bool>,
        new_pool_creation_allowed: Option<bool>,
    ) -> Result<()> {
        instructions::update_protocol(
            ctx,
            new_default_protocol_fee_rate,
            new_max_pool_fee_rate,
            new_paused,
            new_pool_creation_allowed,
        )
    }

    pub fn initiate_authority_transfer(ctx: Context<InitiateAuthorityTransfer>) -> Result<()> {
        instructions::initiate_authority_transfer(ctx)
    }

    pub fn cancel_authority_transfer(ctx: Context<CancelAuthorityTransfer>) -> Result<()> {
        instructions::cancel_authority_transfer(ctx)
    }

    pub fn accept_authority_transfer(ctx: Context<AcceptAuthorityTransfer>) -> Result<()> {
        instructions::accept_authority_transfer(ctx)
    }

    pub fn create_token(
        ctx: Context<CreateToken>,
        ticker: String,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<()> {
        instructions::create_token_via_factory(ctx, ticker, name, symbol, decimals, initial_supply)
    }
}
