/// Token creation instruction using standardized instruction patterns
/// This demonstrates how to use the instruction_handler! macro and patterns
use anchor_lang::prelude::*;
use anchor_spl::token_2022::{mint_to, MintTo};
use crate::logic::event::TokenCreated;
use crate::error::FeelsError;
use crate::utils::token_validation::validate_ticker_format;
use crate::utils::instruction_pattern::{ValidationUtils, EventBuilder};
use crate::{instruction_handler, validate, emit_event};

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CreateTokenParams {
    pub ticker: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_supply: u64,
}

// ============================================================================
// Token Creation Handler (Refactored with Pattern)
// ============================================================================

instruction_handler! {
    create_token_v2,
    crate::CreateToken<'info>,
    CreateTokenParams,
    (),
    {
        validate: {
            // Use standardized validation
            let validator = TokenValidator;
            validator.validate_token_params(&params)?;
        },
        prepare: {
            // No special preparation needed for token creation
        },
        execute: {
            // Initialize token metadata
            let token_metadata = &mut ctx.accounts.token_metadata;
            let clock = Clock::get()?;

            token_metadata.mint = ctx.accounts.token_mint.key();
            token_metadata.authority = ctx.accounts.authority.key();
            token_metadata.ticker = params.ticker.clone();
            token_metadata.name = params.name.clone();
            token_metadata.symbol = params.symbol.clone();
            token_metadata.created_at = clock.unix_timestamp;

            // Mint initial supply if requested
            if params.initial_supply > 0 {
                let mint_accounts = MintTo {
                    mint: ctx.accounts.token_mint.to_account_info(),
                    to: ctx.accounts.authority_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                };
                
                let mint_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    mint_accounts,
                );
                
                mint_to(mint_ctx, params.initial_supply)?;
            }
        },
        events: {
            // Use event builder pattern
            EventBuilder::new(TokenCreated {
                mint: ctx.accounts.token_mint.key(),
                authority: ctx.accounts.authority.key(),
                ticker: params.ticker.clone(),
                name: params.name.clone(),
                symbol: params.symbol.clone(),
                decimals: params.decimals,
                initial_supply: params.initial_supply,
            }).emit()?;
        },
        finalize: {
            msg!("Token created successfully");
            msg!("Mint: {}", ctx.accounts.token_mint.key());
            msg!("Ticker: {}", params.ticker);
        }
    }
}

// ============================================================================
// Validation Helper
// ============================================================================

struct TokenValidator;

impl ValidationUtils for TokenValidator {}

impl TokenValidator {
    fn validate_token_params(&self, params: &CreateTokenParams) -> Result<()> {
        // Validate ticker format
        validate_ticker_format(&params.ticker)?;
        
        // Validate decimals
        validate!(range: params.decimals, 0, 18, "decimals");
        
        // Validate name length
        require!(
            params.name.len() >= 1 && params.name.len() <= 32,
            FeelsError::ValidationError {
                field: "name".to_string(),
                reason: format!("Length must be 1-32 characters, got {}", params.name.len()),
            }
        );
        
        // Validate symbol length
        require!(
            params.symbol.len() >= 1 && params.symbol.len() <= 10,
            FeelsError::ValidationError {
                field: "symbol".to_string(),
                reason: format!("Length must be 1-10 characters, got {}", params.symbol.len()),
            }
        );
        
        // Validate ticker matches symbol
        require!(
            params.ticker == params.symbol,
            FeelsError::ValidationError {
                field: "ticker".to_string(),
                reason: "Ticker must match symbol".to_string(),
            }
        );
        
        // Validate initial supply
        const MAX_SUPPLY: u64 = 1_000_000_000_000_000_000; // 1 quintillion
        if params.initial_supply > 0 {
            validate!(range: params.initial_supply, 1, MAX_SUPPLY, "initial_supply");
        }
        
        Ok(())
    }
}

// ============================================================================
// Alternative: Manual Pattern Implementation
// ============================================================================

/// This shows how to implement the pattern manually without the macro
pub fn create_token_manual<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::CreateToken<'info>>,
    params: CreateTokenParams,
) -> Result<()> {
    // Phase 1: Validation
    msg!("Phase 1: Validating inputs");
    let validator = TokenValidator;
    validator.validate_token_params(&params)?;
    
    // Phase 2: State preparation
    msg!("Phase 2: Preparing state");
    let token_metadata = &mut ctx.accounts.token_metadata;
    let clock = Clock::get()?;
    
    // Phase 3: Core execution
    msg!("Phase 3: Executing logic");
    token_metadata.mint = ctx.accounts.token_mint.key();
    token_metadata.authority = ctx.accounts.authority.key();
    token_metadata.ticker = params.ticker.clone();
    token_metadata.name = params.name.clone();
    token_metadata.symbol = params.symbol.clone();
    token_metadata.created_at = clock.unix_timestamp;
    
    if params.initial_supply > 0 {
        let mint_accounts = MintTo {
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        
        let mint_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
        );
        
        mint_to(mint_ctx, params.initial_supply)?;
    }
    
    // Phase 4: Event emission
    msg!("Phase 4: Emitting events");
    emit_event!(TokenCreated {
        mint: ctx.accounts.token_mint.key(),
        authority: ctx.accounts.authority.key(),
        ticker: params.ticker.clone(),
        name: params.name.clone(),
        symbol: params.symbol.clone(),
        decimals: params.decimals,
        initial_supply: params.initial_supply,
    });
    
    // Phase 5: Finalization
    msg!("Phase 5: Finalizing");
    msg!("Token created successfully");
    
    Ok(())
}