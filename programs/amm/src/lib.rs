#![allow(deprecated)]
#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

#[cfg(test)]
mod tests;

declare_id!("BjBR6qczDV4iimwRq6PTEF9sqjoju8GgAytqi5ATrSes");

#[program]
pub mod feels_amm {
    use super::*;

    pub fn create_pool(
        ctx: Context<CreatePool>,
        fee_bps: u16,
        protocol_fee_bps: u16,
        tick_spacing: i32,
        initial_sqrt_price: u128,
    ) -> Result<()> {
        instructions::create_pool(
            ctx,
            fee_bps,
            protocol_fee_bps,
            tick_spacing,
            initial_sqrt_price,
        )
    }
}
