use anchor_lang::prelude::*;

/// Token-specific price oracle for accurate valuation
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TokenPriceOracle {
    /// Pool this oracle serves
    pub pool: Pubkey,
    
    /// Token 0 mint
    pub token_0_mint: Pubkey,
    
    /// Token 1 mint
    pub token_1_mint: Pubkey,
    
    /// Oracle provider for token 0 (e.g., Pyth price account)
    pub token_0_oracle: Pubkey,
    
    /// Oracle provider for token 1 (e.g., Pyth price account)
    pub token_1_oracle: Pubkey,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// Token 0 price in USD (Q64 fixed point)
    pub token_0_price: u128,
    
    /// Token 1 price in USD (Q64 fixed point)
    pub token_1_price: u128,
    
    /// Token 0 confidence interval (basis points)
    pub token_0_confidence: u32,
    
    /// Token 1 confidence interval (basis points)
    pub token_1_confidence: u32,
    
    /// Oracle status for token 0
    pub token_0_status: OracleStatus,
    
    /// Oracle status for token 1
    pub token_1_status: OracleStatus,
    
    /// Padding for alignment
    pub _padding: [u8; 6],
    
    /// Reserved for future use
    pub _reserved: [u8; 128],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OracleStatus {
    Inactive = 0,
    Active = 1,
    Stale = 2,
    Offline = 3,
}

unsafe impl bytemuck::Pod for OracleStatus {}
unsafe impl bytemuck::Zeroable for OracleStatus {}

impl TokenPriceOracle {
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 32 + 8 + 16 + 16 + 4 + 4 + 1 + 1 + 6 + 128;
    
    /// Check if both oracles are fresh
    pub fn is_fresh(&self, current_time: i64, max_age: i64) -> bool {
        self.token_0_status == OracleStatus::Active && 
        self.token_1_status == OracleStatus::Active &&
        current_time - self.last_update <= max_age
    }
    
    /// Get token TWAPs with confidence check
    pub fn get_token_twaps(&self) -> Result<(u128, u128)> {
        require!(
            self.token_0_status == OracleStatus::Active,
            FeelsProtocolError::StaleOracle
        );
        require!(
            self.token_1_status == OracleStatus::Active,
            FeelsProtocolError::StaleOracle
        );
        
        Ok((self.token_0_price, self.token_1_price))
    }
    
    /// Calculate pool price from token prices
    pub fn get_pool_price(&self) -> Result<u128> {
        let (price_0, price_1) = self.get_token_twaps()?;
        
        // Pool price = token_0_price / token_1_price in Q64
        let pool_price = (price_0 as u128)
            .checked_mul(1u128 << 64)
            .ok_or(FeelsProtocolError::MathOverflow)?
            .checked_div(price_1 as u128)
            .ok_or(FeelsProtocolError::DivisionByZero)?;
            
        Ok(pool_price)
    }
}

// Initialize token price oracle instruction
#[derive(Accounts)]
pub struct InitializeTokenPriceOracle<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// Market manager
    pub market_manager: AccountLoader<'info, crate::state::MarketManager>,
    
    /// Token price oracle to initialize
    #[account(
        init,
        payer = authority,
        space = 8 + TokenPriceOracle::SIZE,
        seeds = [b"token_price_oracle", market_manager.load()?.market.as_ref()],
        bump,
    )]
    pub token_price_oracle: AccountLoader<'info, TokenPriceOracle>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_token_price_oracle(
    ctx: Context<InitializeTokenPriceOracle>,
    token_0_oracle: Pubkey,
    token_1_oracle: Pubkey,
) -> Result<()> {
    let mut price_oracle = ctx.accounts.token_price_oracle.load_init()?;
    let market = ctx.accounts.market_manager.load()?;
    
    price_oracle.pool = market.market;
    price_oracle.token_0_mint = market.token_0_mint;
    price_oracle.token_1_mint = market.token_1_mint;
    price_oracle.token_0_oracle = token_0_oracle;
    price_oracle.token_1_oracle = token_1_oracle;
    price_oracle.last_update = 0;
    price_oracle.token_0_price = 0;
    price_oracle.token_1_price = 0;
    price_oracle.token_0_confidence = 0;
    price_oracle.token_1_confidence = 0;
    price_oracle.token_0_status = OracleStatus::Inactive;
    price_oracle.token_1_status = OracleStatus::Inactive;
    price_oracle._padding = [0; 6];
    price_oracle._reserved = [0; 128];
    
    msg!("Initialized token price oracle for pool {}", price_oracle.pool);
    
    Ok(())
}

use crate::state::FeelsProtocolError;