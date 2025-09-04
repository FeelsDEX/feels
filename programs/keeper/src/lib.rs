#![allow(deprecated)]
#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod events;
pub mod instructions;
pub mod state;
#[cfg(test)]
mod tests;

use instructions::*;

declare_id!("E9X9EMSorquZNAtXG72K4ZqMSzX4Ag9drv78f2eJAvty");

#[program]
pub mod feels_keeper {
    use super::*;

    /// Initialize the Feels Keeper
    pub fn initialize(
        ctx: Context<Initialize>,
        feelssol_to_lst_rate_numerator: u64,
        feelssol_to_lst_rate_denominator: u64,
    ) -> Result<()> {
        instructions::initialize_feels_keeper(
            ctx,
            feelssol_to_lst_rate_numerator,
            feelssol_to_lst_rate_denominator,
        )
    }
}
