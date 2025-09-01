/// Instruction to update market field data for client-side routing.
/// Updates market scalars and TWAPs based on current pool state.
use anchor_lang::prelude::*;
use crate::state::{Pool, MarketField, TwapOracle, MarketDataSource, MarketUpdate, verify_market_update};
use crate::logic::field_update::{update_market_field, update_risk_parameters};
use crate::error::FeelsProtocolError;

// ============================================================================
// Update Market Field
// ============================================================================

#[derive(Accounts)]
pub struct UpdateMarketField<'info> {
    /// Pool being updated
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market field data
    #[account(
        mut,
        seeds = [b"market_field", pool.key().as_ref()],
        bump,
        constraint = market_field.load()?.pool == pool.key() @ FeelsProtocolError::InvalidPool
    )]
    pub market_field: AccountLoader<'info, MarketField>,
    
    /// Market data source
    #[account(
        mut,
        seeds = [b"market_data_source", pool.key().as_ref()],
        bump,
        constraint = market_data_source.load()?.pool == pool.key() @ FeelsProtocolError::InvalidPool
    )]
    pub market_data_source: AccountLoader<'info, MarketDataSource>,
    
    /// Authority allowed to update (must be authorized provider)
    pub authority: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Update market field parameters using unified market data source
pub fn handler(
    ctx: Context<UpdateMarketField>,
    update: MarketUpdate,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    
    // Load accounts
    let pool = ctx.accounts.pool.load()?;
    let mut market_field = ctx.accounts.market_field.load_mut()?;
    let mut market_source = ctx.accounts.market_data_source.load_mut()?;
    
    // Verify the update
    verify_market_update(
        &update,
        &market_source,
        &market_field,
        current_time,
    )?;
    
    // Apply the update based on type
    match update {
        MarketUpdate::ParamSnapshot(snapshot) => {
            // Convert to field update params
            let params = snapshot.to_field_update_params();
            
            // Update field data
            market_field.S = params.S;
            market_field.T = params.T;
            market_field.L = params.L;
            market_field.twap_a = params.twap_a;
            market_field.twap_b = params.twap_b;
            
            // Update risk parameters if provided
            if params.sigma_price > 0 {
                market_field.sigma_price = params.sigma_price;
            }
            if params.sigma_rate > 0 {
                market_field.sigma_rate = params.sigma_rate;
            }
            if params.sigma_leverage > 0 {
                market_field.sigma_leverage = params.sigma_leverage;
            }
            
            market_field.snapshot_ts = snapshot.timestamp;
        }
        MarketUpdate::GradientCommitment(_) => {
            return Err(FeelsProtocolError::NotImplemented {
                feature: "GradientCommitment".to_string()
            }.into());
        }
    }
    
    // Update source metadata
    market_source.last_update = current_time;
    market_source.update_count += 1;
    
    // Emit event
    emit!(MarketFieldUpdated {
        pool: ctx.accounts.pool.key(),
        S: market_field.S,
        T: market_field.T,
        L: market_field.L,
        twap_a: market_field.twap_a,
        twap_b: market_field.twap_b,
        timestamp: current_time,
    });
    
    Ok(())
}

// ============================================================================
// Parameters
// ============================================================================

// Parameters removed - now using MarketUpdate directly

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct MarketFieldUpdated {
    pub pool: Pubkey,
    pub S: u128,
    pub T: u128,
    pub L: u128,
    pub twap_a: u128,
    pub twap_b: u128,
    pub timestamp: i64,
}

// ============================================================================
// Initialize Market Field
// ============================================================================

#[derive(Accounts)]
pub struct InitializeMarketField<'info> {
    /// Pool to create field for
    #[account(mut)]
    pub pool: AccountLoader<'info, Pool>,
    
    /// Market field account to initialize
    #[account(
        init,
        seeds = [b"market_field", pool.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + MarketField::SIZE,
    )]
    pub market_field: AccountLoader<'info, MarketField>,
    
    /// Pool authority
    pub authority: Signer<'info>,
    
    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Initialize market field for a pool
pub fn initialize_handler(
    ctx: Context<InitializeMarketField>,
    params: InitializeMarketFieldParams,
) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    
    // Verify authority
    require!(
        ctx.accounts.authority.key() == pool.authority,
        FeelsProtocolError::InvalidAuthority
    );
    
    // Initialize market field
    let mut market_field = ctx.accounts.market_field.load_init()?;
    
    market_field.pool = ctx.accounts.pool.key();
    
    // Set initial scalars
    market_field.S = params.initial_S;
    market_field.T = params.initial_T;
    market_field.L = params.initial_L;
    
    // Set domain weights
    market_field.w_s = params.w_s;
    market_field.w_t = params.w_t;
    market_field.w_l = params.w_l;
    market_field.w_tau = params.w_tau;
    
    // Set spot weights (default to equal)
    market_field.omega_a = params.omega_a.unwrap_or(5000);
    market_field.omega_b = params.omega_b.unwrap_or(5000);
    
    // Set risk scalers
    market_field.sigma_price = params.sigma_price;
    market_field.sigma_rate = params.sigma_rate;
    market_field.sigma_leverage = params.sigma_leverage;
    
    // Set initial TWAPs
    market_field.twap_a = params.initial_twap_a;
    market_field.twap_b = params.initial_twap_b;
    
    // Set freshness parameters
    market_field.snapshot_ts = Clock::get()?.unix_timestamp;
    market_field.max_staleness = params.max_staleness;
    
    // Validate
    market_field.validate()?;
    
    emit!(MarketFieldInitialized {
        pool: ctx.accounts.pool.key(),
        market_field: ctx.accounts.market_field.key(),
    });
    
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeMarketFieldParams {
    /// Initial market scalars
    pub initial_S: u128,
    pub initial_T: u128,
    pub initial_L: u128,
    
    /// Domain weights (must sum to 10000)
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
    
    /// Spot value weights (optional, defaults to 5000/5000)
    pub omega_a: Option<u32>,
    pub omega_b: Option<u32>,
    
    /// Risk scalers (basis points)
    pub sigma_price: u64,
    pub sigma_rate: u64,
    pub sigma_leverage: u64,
    
    /// Initial TWAPs
    pub initial_twap_a: u128,
    pub initial_twap_b: u128,
    
    /// Maximum staleness (seconds)
    pub max_staleness: i64,
}

#[event]
pub struct MarketFieldInitialized {
    pub pool: Pubkey,
    pub market_field: Pubkey,
}