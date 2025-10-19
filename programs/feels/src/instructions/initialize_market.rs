//! Initialize market instruction
//!
//! Creates a new market with commitment to initial liquidity deployment.
//! The actual liquidity deployment happens in a separate instruction.

use crate::{
    constants::{
        BUFFER_AUTHORITY_SEED, BUFFER_SEED, MARKET_AUTHORITY_SEED, MARKET_SEED, MAX_TICK, MIN_TICK,
        VAULT_SEED,
    },
    error::FeelsError,
    events::MarketInitialized,
    state::{Buffer, Market, MarketPhase, OracleState, PolicyV1, ProtocolConfig},
    utils::tick_from_sqrt_price,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

/// Initialize market parameters
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitializeMarketParams {
    /// Base fee in basis points (e.g., 30 = 0.3%)
    pub base_fee_bps: u16,
    /// Tick spacing for the market
    pub tick_spacing: u16,
    /// Initial price (as sqrt_price Q64)
    pub initial_sqrt_price: u128,
    /// Optional initial buy amount in FeelsSOL (0 = no initial buy)
    pub initial_buy_feelssol_amount: u64,
}

/// Initialize market accounts - minimal version to reduce stack usage
#[derive(Accounts)]
#[instruction(params: InitializeMarketParams)]
pub struct InitializeMarket<'info> {
    /// Creator initializing the market
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Token 0 mint (lower pubkey)
    /// CHECK: Validated in handler
    pub token_0: AccountInfo<'info>,

    /// Token 1 mint (higher pubkey)
    /// CHECK: Validated in handler
    pub token_1: AccountInfo<'info>,

    /// Market account to initialize
    #[account(
        init,
        payer = creator,
        space = Market::LEN,
        seeds = [MARKET_SEED, token_0.key().as_ref(), token_1.key().as_ref()],
        bump,
    )]
    pub market: Box<Account<'info, Market>>,

    /// Buffer account to initialize
    #[account(
        init,
        payer = creator,
        space = Buffer::LEN,
        seeds = [BUFFER_SEED, market.key().as_ref()],
        bump,
    )]
    pub buffer: Box<Account<'info, Buffer>>,

    /// Oracle account to initialize
    #[account(
        init,
        payer = creator,
        space = OracleState::LEN,
        seeds = [b"oracle", market.key().as_ref()],
        bump,
    )]
    pub oracle: Box<Account<'info, OracleState>>,

    /// Vault 0 for token 0
    /// CHECK: Created manually in handler to reduce constraints
    #[account(mut)]
    pub vault_0: AccountInfo<'info>,

    /// Vault 1 for token 1
    /// CHECK: Created manually in handler to reduce constraints
    #[account(mut)]
    pub vault_1: AccountInfo<'info>,

    /// Market authority PDA
    /// CHECK: PDA that controls vaults
    pub market_authority: AccountInfo<'info>,

    /// FeelsSOL mint (hub token)
    /// CHECK: Validated as either token_0 or token_1
    pub feelssol_mint: AccountInfo<'info>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,

    /// Protocol config account (for validating admin-controlled parameters)
    pub protocol_config: Account<'info, ProtocolConfig>,
}

/// Initialize market handler - minimal version to reduce stack usage
pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    params: InitializeMarketParams,
) -> Result<()> {
    // Basic parameter validation (enforce admin-controlled defaults)
    crate::utils::validate_base_fee_bps(params.base_fee_bps)?;
    crate::utils::validate_tick_spacing_param(params.tick_spacing)?;
    crate::utils::validate_initial_sqrt_price(params.initial_sqrt_price)?;

    // Validate token ordering (FeelsSOL must be token_0)
    let token_0_is_feelssol = ctx.accounts.token_0.key() == ctx.accounts.feelssol_mint.key();
    require!(token_0_is_feelssol, FeelsError::InvalidTokenOrder);

    let clock = Clock::get()?;

    // Validate protocol config PDA
    let (expected_config, _) =
        Pubkey::find_program_address(&[ProtocolConfig::SEED], ctx.program_id);
    require!(
        expected_config == ctx.accounts.protocol_config.key(),
        FeelsError::InvalidProtocol
    );

    // Enforce protocol-config defaults for launch window
    let protocol_config = &ctx.accounts.protocol_config;
    require!(
        params.base_fee_bps == protocol_config.default_base_fee_bps,
        FeelsError::InvalidParameter
    );
    require!(
        params.tick_spacing == protocol_config.default_tick_spacing,
        FeelsError::InvalidParameter
    );
    require!(
        params.initial_sqrt_price == protocol_config.default_initial_sqrt_price,
        FeelsError::InvalidParameter
    );

    // Compute current tick from sqrt price
    let mut current_tick = tick_from_sqrt_price(params.initial_sqrt_price)?;
    let tick_spacing_i32 = params.tick_spacing as i32;

    // Align global tick bounds to spacing while clamping within protocol limits
    let global_lower_tick =
        ((MIN_TICK - (tick_spacing_i32 - 1)) / tick_spacing_i32) * tick_spacing_i32;
    let global_upper_tick =
        ((MAX_TICK + (tick_spacing_i32 - 1)) / tick_spacing_i32) * tick_spacing_i32;
    current_tick = current_tick.clamp(global_lower_tick, global_upper_tick);

    // Derive PDAs for vaults and market authority
    let market_key = ctx.accounts.market.key();
    let (market_authority_key, market_authority_bump) = Pubkey::find_program_address(
        &[MARKET_AUTHORITY_SEED, market_key.as_ref()],
        ctx.program_id,
    );
    require!(
        market_authority_key == ctx.accounts.market_authority.key(),
        FeelsError::InvalidAuthority
    );

    let (vault_0_key, vault_0_bump) = Pubkey::find_program_address(
        &[
            VAULT_SEED,
            ctx.accounts.token_0.key().as_ref(),
            ctx.accounts.token_1.key().as_ref(),
            b"0",
        ],
        ctx.program_id,
    );
    require!(
        vault_0_key == ctx.accounts.vault_0.key(),
        FeelsError::InvalidVault
    );

    let (vault_1_key, vault_1_bump) = Pubkey::find_program_address(
        &[
            VAULT_SEED,
            ctx.accounts.token_0.key().as_ref(),
            ctx.accounts.token_1.key().as_ref(),
            b"1",
        ],
        ctx.program_id,
    );
    require!(
        vault_1_key == ctx.accounts.vault_1.key(),
        FeelsError::InvalidVault
    );

    // Validate vault token accounts
    let mut vault_0_data: &[u8] = &ctx.accounts.vault_0.try_borrow_data()?;
    let vault_0_account = TokenAccount::try_deserialize(&mut vault_0_data)?;
    require!(
        vault_0_account.mint == ctx.accounts.token_0.key(),
        FeelsError::InvalidVaultMint
    );
    require!(
        vault_0_account.owner == market_authority_key,
        FeelsError::InvalidAuthority
    );

    let mut vault_1_data: &[u8] = &ctx.accounts.vault_1.try_borrow_data()?;
    let vault_1_account = TokenAccount::try_deserialize(&mut vault_1_data)?;
    require!(
        vault_1_account.mint == ctx.accounts.token_1.key(),
        FeelsError::InvalidVaultMint
    );
    require!(
        vault_1_account.owner == market_authority_key,
        FeelsError::InvalidAuthority
    );

    // Initialize market account with full defaults
    let market = &mut ctx.accounts.market;
    market.version = 1;
    market.is_initialized = true;
    market.is_paused = false;
    market.token_0 = ctx.accounts.token_0.key();
    market.token_1 = ctx.accounts.token_1.key();
    market.feelssol_mint = ctx.accounts.feelssol_mint.key();
    market.base_fee_bps = protocol_config.default_base_fee_bps;
    market.policy = PolicyV1 {
        base_fee_bps: protocol_config.default_base_fee_bps,
        ..PolicyV1::default()
    };
    market.tick_spacing = params.tick_spacing;
    market.sqrt_price = params.initial_sqrt_price;
    market.current_tick = current_tick;
    market.liquidity = 0;
    market.global_lower_tick = global_lower_tick;
    market.global_upper_tick = global_upper_tick;
    market.floor_liquidity = 0;
    market.floor_tick = current_tick;
    market.floor_buffer_ticks = tick_spacing_i32;
    market.last_floor_ratchet_ts = clock.unix_timestamp;
    market.floor_cooldown_secs = 300;
    market.buffer = ctx.accounts.buffer.key();
    market.oracle = ctx.accounts.oracle.key();
    market.authority = ctx.accounts.creator.key();
    market.phase = MarketPhase::Created as u8;
    market.market_authority_bump = market_authority_bump;
    market.vault_0 = vault_0_key;
    market.vault_1 = vault_1_key;
    market.vault_0_bump = vault_0_bump;
    market.vault_1_bump = vault_1_bump;
    market.oracle_bump = ctx.bumps.oracle;
    market.reentrancy_guard = false;
    market.initial_liquidity_deployed = false;
    market.jit_enabled = false;
    market.jit_base_cap_bps = 0;
    market.jit_per_slot_cap_bps = 0;
    market.jit_concentration_width = tick_spacing_i32 as u32;
    market.jit_max_multiplier = 0;
    market.jit_drain_protection_bps = 0;
    market.jit_circuit_breaker_bps = 0;
    market.hub_protocol = Some(ctx.accounts.protocol_config.key());
    market.last_epoch_update = clock.unix_timestamp;
    market.epoch_number = 0;
    market.tick_snapshot_1hr = current_tick;
    market.last_snapshot_timestamp = clock.unix_timestamp;
    market.total_volume_token_0 = 0;
    market.total_volume_token_1 = 0;
    market.rolling_buy_volume = 0;
    market.rolling_sell_volume = 0;
    market.rolling_total_volume = 0;
    market.rolling_window_start_slot = 0;

    // Initialize buffer account
    let buffer = &mut ctx.accounts.buffer;
    buffer.market = ctx.accounts.market.key();
    buffer.authority = ctx.accounts.creator.key();
    buffer.feelssol_mint = ctx.accounts.feelssol_mint.key();
    buffer.fees_token_0 = 0;
    buffer.fees_token_1 = 0;
    buffer.tau_spot = 0;
    buffer.tau_time = 0;
    buffer.tau_leverage = 0;
    buffer.floor_tick_spacing = tick_spacing_i32;
    buffer.floor_placement_threshold = crate::constants::MIN_FLOOR_PLACEMENT_THRESHOLD;
    buffer.last_floor_placement = clock.unix_timestamp;
    buffer.last_rebase = clock.unix_timestamp;
    buffer.total_distributed = 0;
    let (buffer_authority, buffer_authority_bump) = Pubkey::find_program_address(
        &[BUFFER_AUTHORITY_SEED, market_key.as_ref()],
        ctx.program_id,
    );
    buffer.buffer_authority_bump = buffer_authority_bump;
    buffer.jit_last_slot = 0;
    buffer.jit_slot_used_q = 0;
    buffer.jit_rolling_consumption = 0;
    buffer.jit_rolling_window_start = 0;
    buffer.jit_last_heavy_usage_slot = 0;
    buffer.jit_total_consumed_epoch = 0;
    buffer.initial_tau_spot = 0;
    buffer.protocol_owned_override = 0;
    buffer.pomm_position_count = 0;
    buffer._padding = [0; 7];
    // Silence unused warning for derived buffer authority
    let _ = buffer_authority;

    // Initialize oracle account with basic setup
    let oracle = &mut ctx.accounts.oracle;
    oracle.pool_id = ctx.accounts.market.key();
    oracle.observation_index = 0;
    oracle.observation_cardinality = 1;
    oracle.observation_cardinality_next = 1;
    oracle.oracle_bump = ctx.bumps.oracle;

    // Emit event
    emit!(MarketInitialized {
        market: ctx.accounts.market.key(),
        token_0: ctx.accounts.token_0.key(),
        token_1: ctx.accounts.token_1.key(),
        feelssol_mint: ctx.accounts.feelssol_mint.key(),
        buffer: ctx.accounts.buffer.key(),
        base_fee_bps: params.base_fee_bps,
        tick_spacing: params.tick_spacing,
        initial_sqrt_price: params.initial_sqrt_price,
        timestamp: clock.unix_timestamp,
        version: 1,
    });

    Ok(())
}
