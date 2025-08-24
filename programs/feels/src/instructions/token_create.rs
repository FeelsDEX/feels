/// Token create instruction for creating new fungible tokens on the Feels platform.
/// Integrates with the validation system to prevent creation of tokens with restricted tickers.
/// This instruction creates a new Token-2022 mint with metadata and applies ticker validation.

use anchor_lang::prelude::*;
use anchor_spl::token_2022::{MintTo, mint_to};
use crate::utils::token_validate::validate_ticker_format;
use crate::logic::event::TokenCreated;

// ============================================================================
// Handler Function
// ============================================================================

/// Create a new fungible token with ticker validation
pub fn handler(
    ctx: Context<crate::CreateToken>, 
    ticker: String,
    name: String, 
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    // Validate ticker against restrictions and format requirements
    validate_ticker_format(&ticker)?;
    
    // Validate token decimals (must be <= 18)
    require!(
        decimals <= 18,
        crate::state::PoolError::DecimalsTooLarge
    );
    
    // Validate name and symbol lengths
    require!(
        name.len() <= 32 && !name.is_empty(),
        crate::state::PoolError::InvalidAmount
    );
    
    require!(
        symbol.len() <= 12 && !symbol.is_empty(),
        crate::state::PoolError::InvalidAmount
    );
    
    // Initialize token metadata
    let token_metadata = &mut ctx.accounts.token_metadata;
    token_metadata.ticker = ticker.clone();
    token_metadata.name = name.clone();
    token_metadata.symbol = symbol.clone();
    token_metadata.mint = ctx.accounts.token_mint.key();
    token_metadata.authority = ctx.accounts.authority.key();
    token_metadata.created_at = Clock::get()?.unix_timestamp;
    
    // Mint initial supply to authority if requested
    if initial_supply > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        mint_to(cpi_ctx, initial_supply)?;
    }
    
    // Emit create event
    emit!(TokenCreated {
        mint: ctx.accounts.token_mint.key(),
        ticker,
        name,
        symbol,
        decimals,
        authority: ctx.accounts.authority.key(),
        initial_supply,
    });
    
    Ok(())
}

// Import token account for compilation
