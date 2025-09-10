/// Feels Protocol Jupiter Adapter
/// 
/// This crate provides the Jupiter AMM interface implementation for Feels Protocol

use anchor_lang::prelude::*;

declare_id!("EbBFsqA3E4KNReSq9TZs5CGv36BiGDk24BePsUdhbvBu");

pub mod amm;
pub use amm::FeelsAmm;

#[cfg(test)]
mod tests;

/// Re-export the main Feels program for CPI
pub use feels;

/// Jupiter adapter program (kept for backwards compatibility)
#[program]
pub mod feels_jupiter_adapter {
    use super::*;
    
    /// Placeholder instruction for backwards compatibility
    pub fn noop(_ctx: Context<Noop>) -> Result<()> {
        msg!("This program now provides the Jupiter AMM interface implementation");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Noop {}