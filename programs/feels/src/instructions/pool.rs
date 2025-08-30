/// Pool initialization instructions including protocol setup, pool creation, and FeelsSOL wrapper.
/// These are one-time setup operations that establish the foundational infrastructure for the
/// Feels Protocol AMM system. All operations require appropriate authority and are typically
/// called during protocol deployment and pool creation phases.
use anchor_lang::prelude::*;
use crate::logic::event::FeelsSOLInitialized;
use crate::state::FeelsProtocolError;
use crate::utils::{MIN_SQRT_RATE_X96, MAX_FEE_RATE};

// ============================================================================
// Protocol Initialization
// ============================================================================

/// Initialize the Feels Protocol
pub fn initialize_protocol(ctx: Context<crate::InitializeFeels>) -> Result<()> {
    let protocol_state = &mut ctx.accounts.protocol_state;
    let clock = Clock::get()?;

    // Set protocol authority
    protocol_state.authority = ctx.accounts.authority.key();
    protocol_state.treasury = ctx.accounts.treasury.key();

    // Set default fee parameters
    protocol_state.default_protocol_fee_rate = 2000; // 20% of pool fees (2000/10000)
    protocol_state.max_pool_fee_rate = MAX_FEE_RATE; // 100% max (10000 basis points)

    // Enable protocol operations
    protocol_state.paused = false;
    protocol_state.pool_creation_allowed = true;

    // Initialize counters
    protocol_state.total_pools = 0;
    protocol_state.total_volume_usd = 0;
    protocol_state.total_fees_collected_usd = 0;
    protocol_state.total_liquidity_usd = 0;

    // Set creation timestamp
    protocol_state.initialized_at = clock.unix_timestamp;

    msg!("Feels Protocol initialized successfully");
    msg!("Authority: {}", ctx.accounts.authority.key());
    msg!("Treasury: {}", ctx.accounts.treasury.key());
    msg!("Default protocol fee rate: {} bps", protocol_state.default_protocol_fee_rate);

    Ok(())
}

// ============================================================================
// FeelsSOL Initialization  
// ============================================================================

/// Initialize the FeelsSOL wrapper token (universal base pair)
pub fn initialize_feelssol(ctx: Context<crate::InitializeFeelsSOL>, underlying_mint: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    
    // Get keys before mutable borrow
    let feelssol_key = ctx.accounts.feelssol.key();
    let feels_mint_key = ctx.accounts.feels_mint.key();
    let vault_key = ctx.accounts.vault.key();
    
    let feelssol = &mut ctx.accounts.feelssol;
    
    // Note: The vault initialization would need to be done separately
    // as we need the underlying mint account, not just its pubkey
    // For now, we'll assume the vault is pre-initialized

    // Validate underlying mint is not the same as FeelsSOL mint
    require!(
        underlying_mint != ctx.accounts.feels_mint.key(),
        FeelsProtocolError::InvalidMint
    );

    // Validate underlying mint is not a system account
    require!(
        underlying_mint != anchor_lang::solana_program::system_program::id(),
        FeelsProtocolError::InvalidMint
    );

    // Initialize FeelsSOL wrapper
    feelssol.underlying_mint = underlying_mint;
    feelssol.feels_mint = ctx.accounts.feels_mint.key();
    feelssol.vault = ctx.accounts.vault.key();
    feelssol.authority = ctx.accounts.authority.key();

    // Initialize rates and statistics
    feelssol.exchange_rate = 1_000_000_000; // 1:1 initial rate (9 decimals)
    feelssol.total_supply = 0;
    feelssol.total_underlying = 0;

    // Initialize yield tracking
    feelssol.last_yield_update = clock.unix_timestamp;
    feelssol.cumulative_yield = 0;
    feelssol.yield_rate_per_second = 0; // Will be set by oracle updates

    // Initialize flags
    feelssol.is_paused = false;
    feelssol.deposits_paused = false;
    feelssol.withdrawals_paused = false;

    // Set creation timestamp
    feelssol.created_at = clock.unix_timestamp;
    feelssol.last_updated_at = clock.unix_timestamp;

    // Get exchange rate before emitting event
    let exchange_rate = feelssol.exchange_rate;
    
    // Emit event
    emit!(FeelsSOLInitialized {
        feelssol: feelssol_key,
        underlying_mint,
        feels_mint: feels_mint_key,
        vault: vault_key,
        initial_exchange_rate: exchange_rate,
        timestamp: clock.unix_timestamp,
    });

    msg!("FeelsSOL wrapper initialized successfully");
    msg!("Underlying mint: {}", underlying_mint);
    msg!("FeelsSOL mint: {}", feels_mint_key);
    msg!("Initial exchange rate: {}", feelssol.exchange_rate);

    Ok(())
}

// ============================================================================
// Pool Initialization
// ============================================================================

/// Initialize a new concentrated liquidity pool  
/// This ensures only one pool can exist for any token pair regardless of input order
pub fn initialize_pool(
    ctx: Context<crate::InitializePool>,
    fee_rate: u16,
    initial_sqrt_rate: u128,
    base_rate: u16,
    protocol_share: u16,
) -> Result<()> {
    let clock = Clock::get()?;

    // Get token information for validation
    let _decimals_a = ctx.accounts.token_a_mint.decimals;
    let _decimals_b = ctx.accounts.token_b_mint.decimals;

    // Validate protocol state allows pool creation
    require!(
        !ctx.accounts.protocol_state.paused,
        FeelsProtocolError::PoolOperationsPaused
    );
    require!(
        ctx.accounts.protocol_state.pool_creation_allowed,
        FeelsProtocolError::InvalidOperation
    );

    // Validate authority can create pools
    require!(
        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
        FeelsProtocolError::InvalidAuthority
    );

    // Ensure different tokens
    require!(
        ctx.accounts.token_a_mint.key() != ctx.accounts.token_b_mint.key(),
        FeelsProtocolError::InvalidTokenPair
    );

    // Ensure one token is FeelsSOL
    let feelssol_mint = ctx.accounts.feelssol.feels_mint;
    require!(
        ctx.accounts.token_a_mint.key() == feelssol_mint || ctx.accounts.token_b_mint.key() == feelssol_mint,
        FeelsProtocolError::InvalidTokenPair
    );

    // Validate initial sqrt price is within valid range
    require!(
        initial_sqrt_rate >= MIN_SQRT_RATE_X96,
        FeelsProtocolError::PriceOutOfBounds
    );
    require!(
        initial_sqrt_rate <= crate::utils::MAX_SQRT_RATE_X96,
        FeelsProtocolError::PriceOutOfBounds
    );

    // Initialize fee configuration
    let fee_config = &mut ctx.accounts.fee_config;
    fee_config.pool = ctx.accounts.pool.key();
    fee_config.base_rate = base_rate;
    fee_config.protocol_share = protocol_share;
    fee_config._reserved = [0u8; 64];

    // Initialize pool with canonical ordering
    let pool = &mut ctx.accounts.pool.load_init()?;
    
    // Initialize discriminator (this is typically done by Anchor, but we do it explicitly for clarity)
    pool._discriminator = [0u8; 8]; // Anchor will set the proper discriminator

    // Use canonical token ordering (token_a < token_b)
    let (token_a, token_b, vault_a, vault_b) = if ctx.accounts.token_a_mint.key() < ctx.accounts.token_b_mint.key() {
        (
            ctx.accounts.token_a_mint.key(),
            ctx.accounts.token_b_mint.key(),
            ctx.accounts.token_a_vault.key(),
            ctx.accounts.token_b_vault.key()
        )
    } else {
        (
            ctx.accounts.token_b_mint.key(), 
            ctx.accounts.token_a_mint.key(),
            ctx.accounts.token_b_vault.key(),
            ctx.accounts.token_a_vault.key()
        )
    };

    pool.token_a_mint = token_a;
    pool.token_b_mint = token_b;
    pool.token_a_vault = vault_a;
    pool.token_b_vault = vault_b;

    // Set fee configuration reference
    pool.fee_config = ctx.accounts.fee_config.key();
    pool.fee_rate = fee_rate; // IMMUTABLE: Only for PDA derivation
    pool._fee_padding = [0u8; 6];
    
    // Initialize FeeConfig account
    let fee_config = &mut ctx.accounts.fee_config;
    fee_config.initialize(
        ctx.accounts.pool.key(),
        fee_rate,
        protocol_share,
        match fee_rate {
            0..=500 => 10,      // 0.05% fee -> 10 tick spacing
            501..=3000 => 60,   // 0.3% fee -> 60 tick spacing  
            3001..=10000 => 200, // 1% fee -> 200 tick spacing
            _ => return Err(FeelsProtocolError::InvalidFeeRate.into()),
        },
        ctx.accounts.authority.key(),
    )?;
    
    // Set tick spacing from FeeConfig
    pool.tick_spacing = fee_config.tick_spacing;

    // Set initial price and tick
    pool.current_sqrt_rate = initial_sqrt_rate;
    pool.current_tick = crate::utils::TickMath::get_tick_at_sqrt_ratio(initial_sqrt_rate)?;

    // Initialize liquidity
    pool.liquidity = 0;

    // Initialize fee growth tracking
    pool.fee_growth_global_a = [0u64; 4];
    pool.fee_growth_global_b = [0u64; 4];
    pool.protocol_fees_a = 0;
    pool.protocol_fees_b = 0;

    // Initialize volume tracking
    pool.total_volume_a = 0;
    pool.total_volume_b = 0;

    // Note: Token decimals are not stored in the pool anymore
    // They can be fetched from the token mints when needed

    // Initialize tick array bitmap for efficient tick searching
    pool.tick_array_bitmap = [0u64; 16];
    pool._tick_padding = [0u8; 6];

    // Initialize security features
    pool.reentrancy_status = crate::state::reentrancy::ReentrancyStatus::Unlocked as u8;
    pool._security_padding = [0u8; 7];
    
    // Initialize pool features
    pool.oracle = Pubkey::default();
    pool.position_vault = Pubkey::default();
    pool.leverage_params = crate::state::LeverageParameters {
        max_leverage: 10_000_000, // 10x max (6 decimals)
        current_ceiling: 10_000_000,
        protection_curve_type: crate::state::ProtectionCurveType::default(), // Linear
        protection_curve_data: crate::state::ProtectionCurveData::default(),
        last_ceiling_update: 0,
        _padding: [0; 8],
    };
    pool.leverage_stats = crate::state::LeverageStatistics::default();
    pool.volume_tracker = crate::state::VolumeTracker::default();
    pool.hook_registry = Pubkey::default();
    pool.valence_session = Pubkey::default();
    
    // Initialize redenomination
    pool.last_redenomination = 0;
    pool.redenomination_threshold = 5_000_000; // 5% default
    
    // Initialize reserved space
    pool._reserved = [0u8; 128];
    pool._reserved2 = [0u8; 64];
    pool._reserved3 = [0u8; 32];

    // Set creation metadata
    pool.creation_timestamp = clock.unix_timestamp;
    pool.last_update_slot = clock.slot;
    pool.authority = ctx.accounts.authority.key();

    // Update protocol statistics
    let protocol_state = &mut ctx.accounts.protocol_state;
    protocol_state.total_pools = protocol_state.total_pools.saturating_add(1);

    msg!("Pool initialized successfully");
    msg!("Token A: {}", token_a);
    msg!("Token B: {}", token_b);
    msg!("Fee rate: {} bps", fee_rate);
    msg!("Initial sqrt price: {}", initial_sqrt_rate);
    let current_tick = pool.current_tick;
    msg!("Initial tick: {}", current_tick);

    Ok(())
}