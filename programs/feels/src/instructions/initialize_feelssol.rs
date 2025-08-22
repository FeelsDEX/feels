/// Creates and initializes the FeelsSOL token wrapper that serves as the universal base pair.
/// FeelsSOL wraps liquid staking tokens (e.g., JitoSOL) and acts as the hub in the hub-and-spoke
/// liquidity model where all tokens must trade against FeelsSOL. This enables automatic yield
/// generation and simplified routing between any two tokens through the FeelsSOL intermediary.

use anchor_lang::prelude::*;

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
    
    // V79 Fix: Validate underlying mint is not the same as FeelsSOL mint
    require!(
        underlying_mint != ctx.accounts.feels_mint.key(),
        crate::state::FeelsError::InvalidMint
    );
    
    // Validate underlying mint is not a system account
    require!(
        underlying_mint != anchor_lang::solana_program::system_program::id(),
        crate::state::FeelsError::InvalidMint
    );
    
    // Initialize FeelsSOL wrapper
    feelssol.underlying_mint = underlying_mint;
    feelssol.feels_mint = ctx.accounts.feels_mint.key();
    feelssol.total_wrapped = 0;
    feelssol.virtual_reserves = 0;
    feelssol.yield_accumulator = 0;
    feelssol.last_update_slot = clock.slot;
    feelssol.authority = ctx.accounts.authority.key();
    
    // V136 Fix: Validate mint authority was properly set
    // Although Anchor handles this with mint::authority constraint,
    // we explicitly validate for extra safety
    require!(
        ctx.accounts.feels_mint.mint_authority.unwrap() == feelssol.key(),
        crate::state::FeelsError::Unauthorized
    );
    require!(
        ctx.accounts.feels_mint.freeze_authority.unwrap() == feelssol.key(),
        crate::state::FeelsError::Unauthorized
    );
    
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