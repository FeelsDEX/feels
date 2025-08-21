use crate::Initialize;
use anchor_lang::prelude::*;

pub fn handler(_ctx: Context<Initialize>) -> Result<()> {
    msg!("Feels Protocol initialized!");
    Ok(())
}
