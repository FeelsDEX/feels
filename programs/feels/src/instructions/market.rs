//! # Unified Market Instructions
//! 
//! Instructions for managing the consolidated Market account

use anchor_lang::prelude::*;
use crate::error::FeelsError;
use crate::state::unified_market::{Market, DomainWeights};
use crate::state::{BufferAccount, ProtocolState};
use crate::logic::event::{MarketEvent, MarketEventType};
use feels_core::constants::Q64;

// ============================================================================
// Initialize Market Instruction
// ============================================================================

/// Initialize a new unified market
pub fn initialize_unified_market(
    ctx: Context<InitializeUnifiedMarket>,
    params: InitializeUnifiedMarketParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let buffer = &mut ctx.accounts.buffer_account;
    let clock = Clock::get()?;
    
    // Validate parameters
    params.validate()?;
    
    // Initialize market account
    market.initialize(
        ctx.accounts.pool.key(),
        ctx.accounts.token_0_mint.key(),
        ctx.accounts.token_1_mint.key(),
        ctx.accounts.vault_0.key(),
        ctx.accounts.vault_1.key(),
        params.initial_sqrt_price,
        params.domain_weights,
        ctx.accounts.authority.key(),
    )?;
    
    // Set oracle buffer account
    market.oracle_buffer = ctx.accounts.oracle_buffer.key();
    
    // Initialize buffer account
    buffer.pool = ctx.accounts.pool.key();
    buffer.accumulated_fees_0 = 0;
    buffer.accumulated_fees_1 = 0;
    buffer.rebates_paid_0 = 0;
    buffer.rebates_paid_1 = 0;
    buffer.last_update = clock.unix_timestamp;
    buffer.participation_coefficients = [3333, 3333, 3334]; // Equal distribution
    buffer.fee_share_coefficients = [3333, 3333, 3334]; // Equal distribution
    buffer.rebate_cap_per_tx = 300; // 3% max rebate per transaction
    buffer.rebate_cap_per_epoch = 1000; // 10% max rebate per epoch
    
    // Emit initialization event
    emit!(MarketEvent {
        event_type: MarketEventType::MarketInitialized,
        market: market.pool,
        timestamp: clock.unix_timestamp,
        sqrt_price: market.sqrt_price,
        tick: market.current_tick,
        liquidity: 0,
        volume_token_0: 0,
        volume_token_1: 0,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
    });
    
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeUnifiedMarketParams {
    /// Initial sqrt price (Q64 format)
    pub initial_sqrt_price: u128,
    /// Domain weights configuration
    pub domain_weights: DomainWeights,
}

impl InitializeUnifiedMarketParams {
    pub fn validate(&self) -> Result<()> {
        // Validate sqrt price
        require!(
            self.initial_sqrt_price > 0 && self.initial_sqrt_price < u128::MAX / 2,
            FeelsError::InvalidPrice
        );
        
        // Validate domain weights
        self.domain_weights.validate()?;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeUnifiedMarket<'info> {
    /// The unified market account to initialize
    #[account(
        init,
        payer = authority,
        space = Market::LEN,
        seeds = [b"market", pool.key().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,
    
    /// Buffer account for fee accumulation
    #[account(
        init,
        payer = authority,
        space = BufferAccount::LEN,
        seeds = [b"buffer", pool.key().as_ref()],
        bump,
    )]
    pub buffer_account: Account<'info, BufferAccount>,
    
    /// Oracle buffer account (for TWAP data)
    /// CHECK: Will be initialized separately
    #[account(
        seeds = [b"oracle_buffer", pool.key().as_ref()],
        bump,
    )]
    pub oracle_buffer: UncheckedAccount<'info>,
    
    /// Pool identifier
    /// CHECK: Used as seed for PDA derivation
    pub pool: UncheckedAccount<'info>,
    
    /// Token 0 mint
    pub token_0_mint: Account<'info, anchor_spl::token::Mint>,
    
    /// Token 1 mint
    pub token_1_mint: Account<'info, anchor_spl::token::Mint>,
    
    /// Token 0 vault (must be pre-created)
    pub vault_0: Account<'info, anchor_spl::token::TokenAccount>,
    
    /// Token 1 vault (must be pre-created)
    pub vault_1: Account<'info, anchor_spl::token::TokenAccount>,
    
    /// Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// Authority initializing the market
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

// ============================================================================
// Update Market Instruction
// ============================================================================

/// Update market parameters
pub fn update_unified_market(
    ctx: Context<UpdateUnifiedMarket>,
    params: UpdateUnifiedMarketParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == market.authority,
        FeelsError::Unauthorized
    );
    
    // Update domain weights if provided
    if let Some(weights) = params.domain_weights {
        weights.validate()?;
        market.w_s = weights.w_s;
        market.w_t = weights.w_t;
        market.w_l = weights.w_l;
        market.w_tau = weights.w_tau;
    }
    
    // Update volatility parameters if provided
    if let Some((sigma_price, sigma_rate, sigma_leverage)) = params.volatility_params {
        market.sigma_price = sigma_price;
        market.sigma_rate = sigma_rate;
        market.sigma_leverage = sigma_leverage;
    }
    
    // Update fee configuration if provided
    if let Some((base_fee, max_fee)) = params.fee_config {
        require!(
            base_fee <= max_fee && max_fee <= 10000,
            FeelsError::InvalidAmount
        );
        market.base_fee_bps = base_fee;
        market.max_fee_bps = max_fee;
    }
    
    // Update spot token weights if provided
    if let Some((omega_0, omega_1)) = params.spot_weights {
        require!(
            omega_0 + omega_1 == 10000,
            FeelsError::InvalidWeights
        );
        market.omega_0 = omega_0;
        market.omega_1 = omega_1;
    }
    
    // Emit update event
    emit!(MarketEvent {
        event_type: MarketEventType::MarketUpdated,
        market: market.pool,
        timestamp: clock.unix_timestamp,
        sqrt_price: market.sqrt_price,
        tick: market.current_tick,
        liquidity: market.liquidity,
        volume_token_0: market.total_volume_0,
        volume_token_1: market.total_volume_1,
        fee_growth_global_0: market.fee_growth_global_0[0],
        fee_growth_global_1: market.fee_growth_global_1[0],
    });
    
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateUnifiedMarketParams {
    /// Update domain weights
    pub domain_weights: Option<DomainWeights>,
    /// Update volatility parameters (price, rate, leverage)
    pub volatility_params: Option<(u64, u64, u64)>,
    /// Update fee configuration (base, max)
    pub fee_config: Option<(u16, u16)>,
    /// Update spot token weights (omega_0, omega_1)
    pub spot_weights: Option<(u32, u32)>,
}

#[derive(Accounts)]
pub struct UpdateUnifiedMarket<'info> {
    /// The market account to update
    #[account(
        mut,
        seeds = [b"market", market.pool.as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,
    
    /// Authority updating the market
    pub authority: Signer<'info>,
}

// ============================================================================
// Pause/Unpause Market Instructions
// ============================================================================

/// Pause market operations
pub fn pause_market(ctx: Context<PauseMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == market.authority ||
        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
        FeelsError::Unauthorized
    );
    
    market.is_paused = true;
    
    emit!(MarketEvent {
        event_type: MarketEventType::MarketPaused,
        market: market.pool,
        timestamp: Clock::get()?.unix_timestamp,
        sqrt_price: market.sqrt_price,
        tick: market.current_tick,
        liquidity: market.liquidity,
        volume_token_0: 0,
        volume_token_1: 0,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
    });
    
    Ok(())
}

/// Unpause market operations
pub fn unpause_market(ctx: Context<UnpauseMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    
    // Only protocol authority can unpause
    require!(
        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
        FeelsError::Unauthorized
    );
    
    market.is_paused = false;
    
    emit!(MarketEvent {
        event_type: MarketEventType::MarketUnpaused,
        market: market.pool,
        timestamp: Clock::get()?.unix_timestamp,
        sqrt_price: market.sqrt_price,
        tick: market.current_tick,
        liquidity: market.liquidity,
        volume_token_0: 0,
        volume_token_1: 0,
        fee_growth_global_0: 0,
        fee_growth_global_1: 0,
    });
    
    Ok(())
}

#[derive(Accounts)]
pub struct PauseMarket<'info> {
    /// Market to pause
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    /// Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// Authority
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnpauseMarket<'info> {
    /// Market to unpause
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    /// Protocol state
    #[account(
        seeds = [b"protocol"],
        bump,
    )]
    pub protocol_state: Account<'info, ProtocolState>,
    
    /// Authority (must be protocol authority)
    pub authority: Signer<'info>,
}

// ============================================================================
// Market Event Types
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum MarketEventType {
    MarketInitialized,
    MarketUpdated,
    MarketPaused,
    MarketUnpaused,
}