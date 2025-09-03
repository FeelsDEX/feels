use anchor_lang::prelude::*;
use crate::state::VolatilityObservation;

/// External volatility oracle integration
#[account(zero_copy)]
#[repr(C)]
pub struct VolatilityOracle {
    /// Pool this oracle serves
    pub pool: Pubkey,
    
    /// Oracle provider (e.g., Pyth, Switchboard)
    pub oracle_provider: Pubkey,
    
    /// Price feed address (e.g., Pyth price account)
    pub price_feed: Pubkey,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Current volatility (basis points)
    pub current_volatility_bps: u64,
    
    /// 24hr volatility (basis points)
    pub volatility_24h_bps: u64,
    
    /// 7day volatility (basis points)
    pub volatility_7d_bps: u64,
    
    /// Recent observations for validation
    pub observations: [VolatilityObservation; 24],
    
    /// Confidence level (0-100)
    pub confidence: u8,
    
    /// Oracle status
    pub status: OracleStatus,
    
    /// Number of valid observations
    pub observation_count: u8,
    
    /// Padding for alignment
    pub _padding: [u8; 5],
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OracleStatus {
    Inactive = 0,
    Active = 1,
    Stale = 2,
    Offline = 3,
}

unsafe impl bytemuck::Pod for OracleStatus {}
unsafe impl bytemuck::Zeroable for OracleStatus {}

impl VolatilityOracle {
    pub const SIZE: usize = 32 + 32 + 32 + 8 + 8 + 8 + 8 + (16 * 24) + 1 + 1 + 1 + 5 + 128;
    
    /// Check if oracle data is fresh
    pub fn is_fresh(&self, current_time: i64, max_age: i64) -> bool {
        self.status == OracleStatus::Active && 
        current_time - self.last_update <= max_age
    }
    
    /// Get volatility with fallback
    pub fn get_volatility(&self, timeframe: VolatilityTimeframe) -> u64 {
        match timeframe {
            VolatilityTimeframe::Current => self.current_volatility_bps,
            VolatilityTimeframe::Day => self.volatility_24h_bps,
            VolatilityTimeframe::Week => self.volatility_7d_bps,
        }
    }
    
    /// Convert to VolatilityObservation for compatibility
    pub fn to_observation(&self) -> VolatilityObservation {
        // Convert volatility to log return squared format
        // This is an approximation: vol_bps^2 / 10000 * 1e6
        let log_return_squared = (self.current_volatility_bps as u128)
            .saturating_mul(self.current_volatility_bps as u128)
            .saturating_div(10000)
            .saturating_mul(1_000_000)
            .saturating_div(10000)
            .min(u32::MAX as u128) as u32;
            
        VolatilityObservation {
            timestamp: self.last_update,
            log_return_squared,
            _padding: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolatilityTimeframe {
    Current,
    Day,
    Week,
}

// Initialize volatility oracle instruction
#[derive(Accounts)]
pub struct InitializeVolatilityOracle<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Market field
    pub market_field: Account<'info, crate::state::MarketField>,
    
    /// Volatility oracle to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + VolatilityOracle::SIZE,
        seeds = [b"volatility_oracle", market_field.pool.as_ref()],
        bump,
    )]
    pub volatility_oracle: AccountLoader<'info, VolatilityOracle>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_volatility_oracle(
    ctx: Context<InitializeVolatilityOracle>,
    oracle_provider: Pubkey,
    price_feed: Pubkey,
) -> Result<()> {
    let mut volatility_oracle = ctx.accounts.volatility_oracle.load_init()?;
    
    volatility_oracle.pool = ctx.accounts.market_field.pool;
    volatility_oracle.oracle_provider = oracle_provider;
    volatility_oracle.price_feed = price_feed;
    volatility_oracle.last_update = 0;
    volatility_oracle.current_volatility_bps = 0;
    volatility_oracle.volatility_24h_bps = 0;
    volatility_oracle.volatility_7d_bps = 0;
    volatility_oracle.confidence = 0;
    volatility_oracle.status = OracleStatus::Inactive;
    volatility_oracle.observations = [VolatilityObservation { timestamp: 0, log_return_squared: 0, _padding: 0 }; 24];
    volatility_oracle.observation_count = 0;
    volatility_oracle._padding = [0; 5];
    volatility_oracle._reserved = [0; 128];
    
    msg!("Initialized volatility oracle for pool {}", volatility_oracle.pool);
    
    Ok(())
}