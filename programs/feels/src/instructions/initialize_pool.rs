/// Creates a new concentrated liquidity pool pairing any token with FeelsSOL.
/// Implements Uniswap V3-style concentrated liquidity with canonical token ordering
/// to ensure deterministic pool addresses. All pools must have FeelsSOL as one token,
/// enforcing the hub-and-spoke model where cross-token swaps route through FeelsSOL.

use crate::state::PoolError;
use crate::utils::{CanonicalSeeds, TickMath};
use anchor_lang::prelude::*;

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize a new concentrated liquidity pool with canonical token ordering
/// This ensures only one pool can exist for any token pair regardless of input order
pub fn handler(
    ctx: Context<crate::InitializePool>,
    fee_rate: u16,
    initial_sqrt_price: u128,
) -> Result<()> {
    // Validate protocol state allows pool creation
    let protocol_state = &ctx.accounts.protocol_state;
    require!(
        !protocol_state.paused,
        PoolError::PoolOperationsPaused
    );
    require!(
        protocol_state.pool_creation_allowed,
        PoolError::InvalidOperation
    );
    
    // Validate authority can create pools
    require!(
        ctx.accounts.authority.key() == protocol_state.authority,
        PoolError::InvalidAuthority
    );
    
    // Validate fee rate
    require!(
        fee_rate <= crate::utils::constant::MAX_FEE_RATE,
        PoolError::InvalidFeeRate
    );
    
    // Validate initial price bounds
    require!(
        initial_sqrt_price >= crate::utils::MIN_SQRT_PRICE_X64,
        PoolError::PriceOutOfBounds
    );
    
    let mut pool = ctx.accounts.pool.load_init()?;
    let clock = Clock::get()?;
    
    // Get token mints from context
    let mint_a = ctx.accounts.token_a_mint.key();
    let mint_b = ctx.accounts.token_b_mint.key();
    
    // V124 Fix: Validate token decimals compatibility
    let decimals_a = ctx.accounts.token_a_mint.decimals;
    let decimals_b = ctx.accounts.token_b_mint.decimals;
    
    // Both tokens must have the same decimals for proper price calculations
    require!(
        decimals_a == decimals_b,
        PoolError::IncompatibleDecimals
    );
    
    // Limit decimals to prevent overflow in price calculations
    require!(
        decimals_a <= 18 && decimals_b <= 18,
        PoolError::DecimalsTooLarge
    );
    
    // Sort tokens into canonical order
    let (token_0, token_1) = CanonicalSeeds::sort_token_mints(&mint_a, &mint_b);
    
    // Validate that FeelsSOL is one of the tokens
    let feelssol = &ctx.accounts.feelssol;
    require!(
        feelssol.feels_mint == token_0 || feelssol.feels_mint == token_1,
        PoolError::NotFeelsSOLPair
    );
    
    // Initialize pool with canonical token order
    pool.version = 1;
    pool.token_a_mint = token_0;
    pool.token_b_mint = token_1;
    
    // Map vaults to canonical order
    if token_0 == mint_a {
        pool.token_a_vault = ctx.accounts.token_a_vault.key();
        pool.token_b_vault = ctx.accounts.token_b_vault.key();
    } else {
        pool.token_a_vault = ctx.accounts.token_b_vault.key();
        pool.token_b_vault = ctx.accounts.token_a_vault.key();
    }
    
    // Initialize fee configuration
    pool.fee_rate = fee_rate;
    pool.protocol_fee_rate = 2000; // 20% of swap fees go to protocol
    pool.tick_spacing = match fee_rate {
        1 => 1,      // 0.01% fee = 1 tick spacing
        5 => 10,     // 0.05% fee = 10 tick spacing  
        30 => 60,    // 0.3% fee = 60 tick spacing
        100 => 200,  // 1% fee = 200 tick spacing
        _ => return Err(PoolError::InvalidFeeRate.into()),
    };
    
    // Initialize price and liquidity state
    pool.current_tick = TickMath::get_tick_at_sqrt_ratio(initial_sqrt_price)?;
    pool.current_sqrt_price = initial_sqrt_price;
    pool.liquidity = 0;
    
    // Initialize empty state
    pool.tick_array_bitmap = [0u64; 16];
    pool.fee_growth_global_0 = [0u64; 4];
    pool.fee_growth_global_1 = [0u64; 4];
    pool.protocol_fees_0 = 0;
    pool.protocol_fees_1 = 0;
    
    // Set metadata
    pool.authority = ctx.accounts.authority.key();
    pool.creation_timestamp = clock.unix_timestamp;
    pool.last_update_slot = clock.slot;
    
    // Initialize statistics
    pool.total_volume_0 = 0;
    pool.total_volume_1 = 0;
    
    // Reserved for future use
    pool._reserved = [0u8; 512];
    
    emit!(PoolInitialized {
        pool: ctx.accounts.pool.key(),
        token_0: pool.token_a_mint,
        token_1: pool.token_b_mint,
        fee_rate,
        tick_spacing: pool.tick_spacing,
        initial_sqrt_price,
        initial_tick: pool.current_tick,
        authority: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Pool initialized with canonical token order");
    msg!("Token 0: {}", token_0);
    msg!("Token 1: {}", token_1);
    let current_tick = pool.current_tick; // Copy to avoid packed field reference
    msg!("Initial price: {} (tick {})", initial_sqrt_price, current_tick);
    
    Ok(())
}

#[event]
pub struct PoolInitialized {
    #[index]
    pub pool: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee_rate: u16,
    pub tick_spacing: i16,
    pub initial_sqrt_price: u128,
    pub initial_tick: i32,
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Validate that a pool can be initialized with the given parameters
pub fn validate_pool_initialization(
    token_a: &Pubkey,
    token_b: &Pubkey,
    feelssol_mint: &Pubkey,
    _fee_rate: u16,
) -> Result<()> {
    // Ensure different tokens
    require!(
        token_a != token_b,
        PoolError::InvalidTokenPair
    );
    
    // Ensure one token is FeelsSOL
    require!(
        token_a == feelssol_mint || token_b == feelssol_mint,
        PoolError::NotFeelsSOLPair
    );
    
    // Validate fee rate
    crate::logic::fee::FeeMath::validate_fee_rate(_fee_rate)?;
    
    Ok(())
}