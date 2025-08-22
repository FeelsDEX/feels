/// Creates and initializes the FeelsSOL token wrapper that serves as the universal base pair.
/// FeelsSOL wraps liquid staking tokens (e.g., JitoSOL) and acts as the hub in the hub-and-spoke
/// liquidity model where all tokens must trade against FeelsSOL. This enables automatic yield
/// generation and simplified routing between any two tokens through the FeelsSOL intermediary.

use crate::state::FeelsSOL;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::Mint;
use anchor_lang::accounts::interface_account::InterfaceAccount;

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize the FeelsSOL wrapper token (universal base pair)
pub fn handler(
    ctx: Context<crate::InitializeFeelsSOL>,
    underlying_mint: Pubkey,
) -> Result<()> {
    let feelssol = &mut ctx.accounts.feelssol;
    let clock = Clock::get()?;
    
    // Initialize FeelsSOL wrapper
    feelssol.underlying_mint = underlying_mint;
    feelssol.feels_mint = ctx.accounts.feels_mint.key();
    feelssol.total_wrapped = 0;
    feelssol.virtual_reserves = 0;
    feelssol.yield_accumulator = 0;
    feelssol.last_update_slot = clock.slot;
    feelssol.authority = ctx.accounts.authority.key();
    
    emit!(FeelsSOLInitialized {
        feels_mint: feelssol.feels_mint,
        underlying_mint: feelssol.underlying_mint,
        authority: feelssol.authority,
    });
    
    Ok(())
}

#[event]
pub struct FeelsSOLInitialized {
    pub feels_mint: Pubkey,
    pub underlying_mint: Pubkey,
    pub authority: Pubkey,
}