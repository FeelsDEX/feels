/// Token creation instruction for creating new fungible tokens on the Feels platform.
/// Integrates with the validation system to prevent creation of tokens with restricted tickers.
/// This instruction creates a new Token-2022 mint with metadata and applies ticker validation.
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{mint_to, MintTo};
use crate::logic::event::TokenCreated;
use crate::state::FeelsProtocolError;
use crate::utils::token_validation::validate_ticker_format;

// ============================================================================
// Token Creation Handler
// ============================================================================

/// Create a new fungible token with ticker validation
pub fn create_token(
    ctx: Context<crate::CreateToken>,
    ticker: String,
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> Result<()> {
    // Validate ticker against restrictions and format requirements
    validate_ticker_format(&ticker)?;

    // Validate token decimals (must be <= 18 for compatibility)
    require!(decimals <= 18, FeelsProtocolError::DecimalsTooLarge);

    // Validate name and symbol lengths
    require!(
        name.len() >= 1 && name.len() <= 32,
        FeelsProtocolError::InvalidTokenName
    );
    require!(
        symbol.len() >= 1 && symbol.len() <= 10,
        FeelsProtocolError::InvalidTokenSymbol
    );

    // Validate ticker matches symbol (consistency check)
    require!(
        ticker == symbol,
        FeelsProtocolError::TickerSymbolMismatch
    );

    // Validate initial supply (reasonable limits)
    require!(
        initial_supply > 0 && initial_supply <= 1_000_000_000_000_000_000, // 1 quintillion max
        FeelsProtocolError::InvalidInitialSupply
    );

    // Initialize token metadata
    let token = &mut ctx.accounts.token;
    let clock = Clock::get()?;

    token.mint = ctx.accounts.mint.key();
    token.authority = ctx.accounts.authority.key();
    token.ticker = ticker.clone();
    token.name = name.clone();
    token.symbol = symbol.clone();
    token.decimals = decimals;
    token.total_supply = initial_supply;
    token.is_paused = false;
    token.freeze_authority = ctx.accounts.authority.key();
    token.created_at = clock.unix_timestamp;
    token.last_updated_at = clock.unix_timestamp;

    // Initialize metadata fields
    token.description = "".to_string(); // Can be updated later
    token.image_uri = "".to_string();   // Can be updated later
    token.website_uri = "".to_string(); // Can be updated later

    // Initialize supply tracking
    token.circulating_supply = initial_supply;
    token.burned_supply = 0;

    // Mint initial supply to authority
    if initial_supply > 0 {
        let mint_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        
        let mint_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
        );
        
        mint_to(mint_ctx, initial_supply)?;
    }

    // Emit token creation event
    emit!(TokenCreated {
        mint: ctx.accounts.mint.key(),
        authority: ctx.accounts.authority.key(),
        ticker: ticker.clone(),
        name: name.clone(),
        symbol: symbol.clone(),
        decimals,
        initial_supply,
        timestamp: clock.unix_timestamp,
    });

    msg!("Token created successfully");
    msg!("Mint: {}", ctx.accounts.mint.key());
    msg!("Ticker: {}", ticker);
    msg!("Name: {}", name);
    msg!("Symbol: {}", symbol);
    msg!("Decimals: {}", decimals);
    msg!("Initial supply: {}", initial_supply);

    Ok(())
}