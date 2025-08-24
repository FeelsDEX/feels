/// Initializes a new liquidity pool with the given token pair and fee rate.
/// Validates token ordering, sets initial price, and configures fee parameters.
/// Each pool is uniquely identified by its token pair and fee tier. Only
/// authorized accounts can create new pools when the protocol allows it.

use anchor_lang::prelude::*;
use crate::state::PoolError;
use crate::utils::MIN_SQRT_PRICE_X96;

// ============================================================================
// Handler Functions
// ============================================================================

/// Initialize a new concentrated liquidity pool  
/// This ensures only one pool can exist for any token pair regardless of input order
pub fn handler(
    ctx: Context<crate::InitializePool>,
    fee_rate: u16,
    initial_sqrt_price: u128,
) -> Result<()> {
    let clock = Clock::get()?;
    
    // Get token information for validation
    let decimals_a = ctx.accounts.token_a_mint.decimals;
    let decimals_b = ctx.accounts.token_b_mint.decimals;
    
    // Validate protocol state allows pool creation
    require!(
        !ctx.accounts.protocol_state.paused,
        PoolError::PoolOperationsPaused
    );
    require!(
        ctx.accounts.protocol_state.pool_creation_allowed,
        PoolError::InvalidOperation
    );
    
    // Validate authority can create pools
    require!(
        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
        PoolError::InvalidAuthority
    );
    
    // Ensure different tokens
    require!(
        ctx.accounts.token_a_mint.key() != ctx.accounts.token_b_mint.key(),
        PoolError::InvalidTokenPair
    );
    
    // Ensure one token is FeelsSOL
    require!(
        ctx.accounts.token_a_mint.key() == ctx.accounts.feelssol.feels_mint || 
        ctx.accounts.token_b_mint.key() == ctx.accounts.feelssol.feels_mint,
        PoolError::NotFeelsSOLPair
    );
    
    // Validate fee rate
    require!(
        fee_rate <= crate::utils::MAX_FEE_RATE,
        PoolError::InvalidFeeRate
    );
    crate::utils::FeeMath::validate_fee_rate(fee_rate)?;
    
    // Validate initial price bounds
    require!(
        initial_sqrt_price >= MIN_SQRT_PRICE_X96,
        PoolError::PriceOutOfBounds
    );
    
    // Validate token decimals compatibility
    require!(
        decimals_a == decimals_b,
        PoolError::IncompatibleDecimals
    );
    
    // Limit decimals to prevent overflow in price calculations
    require!(
        decimals_a <= 18 && decimals_b <= 18,
        PoolError::DecimalsTooLarge
    );

    let mut pool = ctx.accounts.pool.load_init()?;
    
    // Sort tokens into canonical order
    let (token_0_mint, token_1_mint) = if ctx.accounts.token_a_mint.key() < ctx.accounts.token_b_mint.key() {
        (ctx.accounts.token_a_mint.key(), ctx.accounts.token_b_mint.key())
    } else {
        (ctx.accounts.token_b_mint.key(), ctx.accounts.token_a_mint.key())
    };
    
    // Set pool tokens with canonical ordering
    pool.token_a_mint = token_0_mint;
    pool.token_b_mint = token_1_mint;
    
    // Ensure feelssol is one of the tokens
    let feelssol_mint = ctx.accounts.feelssol.feels_mint;
    require!(
        pool.token_a_mint == feelssol_mint || pool.token_b_mint == feelssol_mint,
        PoolError::NotFeelsSOLPair
    );
    
    // Set initial liquidity state
    pool.liquidity = 0;
    pool.current_sqrt_price = initial_sqrt_price;
    
    // Calculate initial tick from sqrt price
    pool.current_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(initial_sqrt_price)?;
    
    // Initialize fee parameters
    pool.fee_rate = fee_rate;
    pool.protocol_fee_rate = ctx.accounts.protocol_state.default_protocol_fee_rate;
    pool.fee_growth_global_0 = [0u64; 4];
    pool.fee_growth_global_1 = [0u64; 4];
    
    // Calculate tick spacing based on fee rate
    pool.tick_spacing = match fee_rate {
        1 => 1,      // 0.01%
        5 => 10,     // 0.05%
        30 => 60,    // 0.30%
        100 => 200,  // 1.00%
        _ => return Err(PoolError::InvalidFeeRate.into()),
    };
    
    // Initialize bitmap (empty - no tick arrays initialized)
    pool.tick_array_bitmap = [0u64; 16];
    
    // Initialize authority and metadata
    pool.authority = ctx.accounts.authority.key();
    pool.creation_timestamp = clock.unix_timestamp;
    
    // Initialize timestamps
    // Initialize volumes
    pool.total_volume_0 = 0;
    pool.total_volume_1 = 0;
    pool.last_update_slot = clock.slot;
    
    // Initialize protocol fees
    pool.protocol_fees_0 = 0;
    pool.protocol_fees_1 = 0;
    
    // Initialize padding and reserved space
    pool._padding2 = [0u8; 6];
    pool._reserved = [0u8; 512];
    
    // Log pool initialization (copy values from packed struct)
    let pool_token_a = pool.token_a_mint;
    let pool_token_b = pool.token_b_mint;
    let tick_spacing = pool.tick_spacing;
    let current_tick = pool.current_tick;
    
    msg!("Pool initialized: {} <-> {}", pool_token_a, pool_token_b);
    msg!("Fee rate: {} bps, tick spacing: {}", fee_rate, tick_spacing);
    msg!("Initial sqrt price: {}, tick: {}", initial_sqrt_price, current_tick);
    
    Ok(())
}