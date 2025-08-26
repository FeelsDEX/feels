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
}
