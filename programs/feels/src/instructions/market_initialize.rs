/// Market initialization instructions for the physics-based AMM.
/// Creates MarketField and BufferAccount to establish a new trading market.
use anchor_lang::prelude::*;
use crate::error::FeelsProtocolError;

// ============================================================================
// Parameters
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeMarketParams {
    /// Initial spot price (Q64 format)
    pub initial_spot: u128,
    /// Domain weights [w_s, w_t, w_l, w_tau] - must sum to 10000 (100%)
    pub initial_weights: [u32; 4],
}

// ============================================================================
// Result Type
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Default)]
pub struct InitializeMarketResult {
    /// The initialized market field address
    pub market_field: Pubkey,
    /// The initialized buffer account address
    pub buffer_account: Pubkey,
    /// Computed invariant value
    pub initial_invariant: u128,
}

// ============================================================================
// State Container
// ============================================================================


// ============================================================================
// Validation Utils
// ============================================================================


// ============================================================================
// Handler Function Using Standard Pattern
// ============================================================================

pub fn initialize_market<'info>(
    ctx: Context<'_, '_, 'info, 'info, crate::InitializeMarket<'info>>,
    params: InitializeMarketParams,
) -> Result<InitializeMarketResult> {
    // Phase 1: Validation
    msg!("Phase 1: Validating inputs");
    
    // Basic market parameter validation
    require!(params.initial_spot > 0, FeelsProtocolError::InvalidAmount);
    
    // Validate weights sum to 10000 (100%)
    let weight_sum: u32 = params.initial_weights.iter().sum();
    require!(weight_sum == 10000, FeelsProtocolError::InvalidWeights);
    
    // Ensure protocol state is initialized
    require!(
        ctx.accounts.protocol_state.is_initialized,
        FeelsProtocolError::NotInitialized
    );
    
    // Validate authority
    require!(
        ctx.accounts.authority.key() == ctx.accounts.protocol_state.authority,
        FeelsProtocolError::Unauthorized
    );
    
    // Phase 2: State preparation
    msg!("Phase 2: Preparing state");
    // Nothing to prepare - we'll access params directly in execute
    
    // Phase 3: Core execution
    msg!("Phase 3: Executing logic");
    
    let timestamp = Clock::get()?.unix_timestamp;
    
    // Initialize market field
    let market_key = ctx.accounts.market_field.key();
    let market_field = &mut ctx.accounts.market_field;
    
    // Set pool reference
    market_field.pool = market_key;
    
    // Initialize market scalars
    market_field.S = params.initial_spot;
    market_field.T = 1u128 << 64;  // Initial time = 1.0
    market_field.L = 1u128 << 64;  // Initial leverage = 1.0
    
    // Set domain weights
    market_field.w_s = params.initial_weights[0];
    market_field.w_t = params.initial_weights[1];
    market_field.w_l = params.initial_weights[2];
    market_field.w_tau = params.initial_weights[3];
    
    // Set equal token weights initially
    market_field.omega_0 = 5000; // 50%
    market_field.omega_1 = 5000; // 50%
    
    // Initialize risk parameters
    market_field.sigma_price = 100;     // 1% volatility
    market_field.sigma_rate = 50;       // 0.5% rate volatility  
    market_field.sigma_leverage = 200;  // 2% leverage volatility
    
    // Set TWAPs to initial spot
    market_field.twap_0 = params.initial_spot;
    market_field.twap_1 = 1u128 << 64; // 1.0 for token 1
    
    // Set freshness parameters
    market_field.snapshot_ts = timestamp;
    market_field.max_staleness = 300; // 5 minutes
    
    // Initialize commitment hash (empty for new market)
    market_field.commitment_hash = [0u8; 32];
    market_field._reserved = [0u8; 32];
    
    // Initialize buffer account
    let buffer = &mut ctx.accounts.buffer_account;
    
    buffer.pool = ctx.accounts.market_field.key();
    buffer.tau_value = 0; // Empty buffer initially
    buffer.tau_reserved = 0;
    
    // Set participation coefficients
    buffer.zeta_spot = 3333;     // 33.33%
    buffer.zeta_time = 3333;     // 33.33%
    buffer.zeta_leverage = 3334; // 33.34%
    
    // Initialize fee tracking
    buffer.fee_share_spot = 3333;
    buffer.fee_share_time = 3333;
    buffer.fee_share_leverage = 3334;
    buffer.fee_share_last_update = timestamp;
    
    // Set rebate caps
    buffer.rebate_cap_tx = u64::MAX;    // No limit initially
    buffer.rebate_cap_epoch = u64::MAX; // No limit initially
    buffer.rebate_paid_epoch = 0;
    buffer.epoch_start = timestamp;
    buffer.epoch_duration = 86400; // 24 hours
    
    // Set authority
    buffer.authority = ctx.accounts.authority.key();
    buffer.protocol_fee_recipient = ctx.accounts.protocol_state.treasury;
    
    // Initialize rebate parameters
    buffer.rebate_eta = 5000; // 50% participation rate
    buffer.kappa = 5000; // 50% price improvement clamp
    
    // Initialize statistics
    buffer.total_fees_collected = 0;
    buffer.total_rebates_paid = 0;
    buffer.last_update = timestamp;
    
    // Reserved space
    buffer._reserved = [0u8; 64];
    
    // Initialize TWAP oracle
    let mut twap = ctx.accounts.twap_oracle.load_init()?;
    twap.pool = ctx.accounts.market_field.key();
    twap.observation_index = 0;
    twap.observation_cardinality = 1;
    twap.observation_cardinality_next = 1;
    drop(twap);
    
    // Initialize market data source
    let mut data_source = ctx.accounts.market_data_source.load_init()?;
    data_source.pool = ctx.accounts.market_field.key();
    data_source.primary_provider = ctx.accounts.authority.key();
    data_source.secondary_provider = Pubkey::default();
    data_source.update_frequency = 60; // 1 minute minimum
    data_source.last_update = timestamp;
    data_source.update_count = 0;
    data_source.is_active = 1; // 1 = true for zero-copy
    drop(data_source);
    
    let result = InitializeMarketResult {
        market_field: ctx.accounts.market_field.key(),
        buffer_account: ctx.accounts.buffer_account.key(),
        initial_invariant: params.initial_spot,
    };
    
    // Phase 4: Event emission
    msg!("Phase 4: Emitting events");
    emit!(crate::logic::event::MarketEvent {
        market: ctx.accounts.market_field.key(),
        event_type: crate::logic::event::MarketEventType::Initialized,
        token_0_mint: ctx.accounts.token_0_mint.key(),
        token_1_mint: ctx.accounts.token_1_mint.key(),
        token_0_vault: ctx.accounts.token_0_vault.key(),
        token_1_vault: ctx.accounts.token_1_vault.key(),
        spot_price: params.initial_spot,
        weights: params.initial_weights,
        invariant: params.initial_spot,
        update_source: 0, // 0=Keeper
        sequence: 0, // First update
        previous_commitment: [0u8; 32], // No previous commitment
        timestamp: timestamp,
    });
    
    // Phase 5: Finalization
    msg!("Phase 5: Finalizing");
    msg!("Market initialized:");
    msg!("  Field: {}", result.market_field);
    msg!("  Buffer: {}", result.buffer_account);
    msg!("  Spot: {}", params.initial_spot);
    msg!("  Weights: [{},{},{},{}]", 
        params.initial_weights[0], 
        params.initial_weights[1], 
        params.initial_weights[2], 
        params.initial_weights[3]
    );
    msg!("  Initial Invariant: {}", result.initial_invariant);
    
    Ok(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

