//! # Unified Market Account
//! 
//! A single, authoritative market account that combines thermodynamic physics
//! parameters with traditional AMM state. This simplifies the protocol by
//! eliminating the need to synchronize between separate accounts.

use anchor_lang::prelude::*;
use crate::error::FeelsError;
use crate::constants::*;

/// Unified market account - single source of truth for all market state
#[account]
#[derive(Debug)]
pub struct Market {
    // ========== ACCOUNT METADATA ==========
    
    /// Account discriminator
    pub discriminator: [u8; 8],
    
    /// Protocol version
    pub version: u8,
    
    /// Is market initialized
    pub is_initialized: bool,
    
    /// Is market paused
    pub is_paused: bool,
    
    /// Padding for alignment
    pub _padding: [u8; 5],
    
    // ========== MARKET IDENTITY ==========
    
    /// Unique market identifier (pool address)
    pub pool: Pubkey,
    
    /// Token 0 mint
    pub token_0: Pubkey,
    
    /// Token 1 mint
    pub token_1: Pubkey,
    
    /// Token 0 vault
    pub vault_0: Pubkey,
    
    /// Token 1 vault
    pub vault_1: Pubkey,
    
    // ========== 3D THERMODYNAMIC STATE ==========
    // Core position P = (S,T,L) in the energy landscape
    
    /// Spot dimension scalar (Q64 fixed-point)
    pub S: u128,
    
    /// Time dimension scalar (Q64 fixed-point)
    pub T: u128,
    
    /// Leverage dimension scalar (Q64 fixed-point)
    pub L: u128,
    
    // ========== DOMAIN WEIGHTS ==========
    // Controls relative influence of each dimension
    
    /// Spot weight (basis points, part of 10000 total)
    pub w_s: u32,
    
    /// Time weight (basis points)
    pub w_t: u32,
    
    /// Leverage weight (basis points)
    pub w_l: u32,
    
    /// Buffer weight (basis points, independent)
    pub w_tau: u32,
    
    /// Token 0 weight in spot dimension
    pub omega_0: u32,
    
    /// Token 1 weight in spot dimension
    pub omega_1: u32,
    
    // ========== VOLATILITY & RISK ==========
    
    /// Price volatility (basis points)
    pub sigma_price: u64,
    
    /// Rate volatility (basis points)
    pub sigma_rate: u64,
    
    /// Leverage volatility (basis points)
    pub sigma_leverage: u64,
    
    // ========== AMM CORE STATE ==========
    
    /// Current sqrt price (Q64 fixed-point)
    pub sqrt_price: u128,
    
    /// Current tick
    pub current_tick: i32,
    
    /// Current liquidity
    pub liquidity: u128,
    
    // ========== FEE STATE ==========
    
    /// Base fee rate (basis points)
    pub base_fee_bps: u16,
    
    /// Maximum fee rate (basis points)
    pub max_fee_bps: u16,
    
    /// Fee growth global token 0
    pub fee_growth_global_0: [u64; 4],
    
    /// Fee growth global token 1
    pub fee_growth_global_1: [u64; 4],
    
    /// Protocol fees owed token 0
    pub protocol_fees_0: u64,
    
    /// Protocol fees owed token 1
    pub protocol_fees_1: u64,
    
    // ========== VOLUME & STATISTICS ==========
    
    /// Total volume token 0
    pub total_volume_0: u128,
    
    /// Total volume token 1
    pub total_volume_1: u128,
    
    /// Total volume in feelssol
    pub total_volume_feelssol: u64,
    
    /// feelssol reserves for rebates
    pub feelssol_reserves: u64,
    
    // ========== ORACLE DATA ==========
    
    /// Time-weighted average price 0
    pub twap_0: u128,
    
    /// Time-weighted average price 1
    pub twap_1: u128,
    
    /// Last oracle update timestamp
    pub last_oracle_update: i64,
    
    /// Oracle buffer account
    pub oracle_buffer: Pubkey,
    
    // ========== ACCESS CONTROL ==========
    
    /// Market authority
    pub authority: Pubkey,
    
    /// Fee recipient
    pub fee_recipient: Pubkey,
}

impl Market {
    /// Size of the market account
    pub const LEN: usize = 8 + // discriminator
        1 + // version
        1 + // is_initialized
        1 + // is_paused
        5 + // padding
        32 + // pool
        32 + // token_0
        32 + // token_1
        32 + // vault_0
        32 + // vault_1
        16 + // S
        16 + // T
        16 + // L
        4 + // w_s
        4 + // w_t
        4 + // w_l
        4 + // w_tau
        4 + // omega_0
        4 + // omega_1
        8 + // sigma_price
        8 + // sigma_rate
        8 + // sigma_leverage
        16 + // sqrt_price
        4 + // current_tick
        16 + // liquidity
        2 + // base_fee_bps
        2 + // max_fee_bps
        32 + // fee_growth_global_0
        32 + // fee_growth_global_1
        8 + // protocol_fees_0
        8 + // protocol_fees_1
        16 + // total_volume_0
        16 + // total_volume_1
        8 + // total_volume_feelssol
        8 + // feelssol_reserves
        16 + // twap_0
        16 + // twap_1
        8 + // last_oracle_update
        32 + // oracle_buffer
        32 + // authority
        32; // fee_recipient
    
    /// Initialize a new market
    pub fn initialize(
        &mut self,
        pool: Pubkey,
        token_0: Pubkey,
        token_1: Pubkey,
        vault_0: Pubkey,
        vault_1: Pubkey,
        sqrt_price: u128,
        weights: DomainWeights,
        authority: Pubkey,
    ) -> Result<()> {
        // Validate weights
        weights.validate()?;
        
        // Set discriminator
        self.discriminator = Market::discriminator();
        self.version = 1;
        self.is_initialized = true;
        self.is_paused = false;
        
        // Set identity
        self.pool = pool;
        self.token_0 = token_0;
        self.token_1 = token_1;
        self.vault_0 = vault_0;
        self.vault_1 = vault_1;
        
        // Initialize thermodynamic state at neutral position
        self.S = Q64;
        self.T = Q64;
        self.L = Q64;
        
        // Set weights
        self.w_s = weights.w_s;
        self.w_t = weights.w_t;
        self.w_l = weights.w_l;
        self.w_tau = weights.w_tau;
        self.omega_0 = 5000; // Default 50/50 for spot tokens
        self.omega_1 = 5000;
        
        // Initialize volatility at moderate levels
        self.sigma_price = 100; // 1%
        self.sigma_rate = 50; // 0.5%
        self.sigma_leverage = 200; // 2%
        
        // Initialize AMM state
        self.sqrt_price = sqrt_price;
        self.current_tick = tick_math::get_tick_at_sqrt_price(sqrt_price)?;
        self.liquidity = 0;
        
        // Initialize fees
        self.base_fee_bps = 30; // 0.3%
        self.max_fee_bps = 300; // 3%
        self.fee_growth_global_0 = [0; 4];
        self.fee_growth_global_1 = [0; 4];
        self.protocol_fees_0 = 0;
        self.protocol_fees_1 = 0;
        
        // Initialize volume tracking
        self.total_volume_0 = 0;
        self.total_volume_1 = 0;
        self.total_volume_feelssol = 0;
        self.feelssol_reserves = 0;
        
        // Initialize oracle
        self.twap_0 = sqrt_price;
        self.twap_1 = sqrt_price;
        self.last_oracle_update = Clock::get()?.unix_timestamp;
        
        // Set authority
        self.authority = authority;
        self.fee_recipient = authority; // Default to authority
        
        Ok(())
    }
    
    /// Get domain weights as a struct
    pub fn get_domain_weights(&self) -> DomainWeights {
        DomainWeights {
            w_s: self.w_s,
            w_t: self.w_t,
            w_l: self.w_l,
            w_tau: self.w_tau,
        }
    }
    
    /// Update thermodynamic scalars
    pub fn update_scalars(&mut self, s: u128, t: u128, l: u128) {
        self.S = s;
        self.T = t;
        self.L = l;
    }
    
    /// Update price and tick
    pub fn update_price(&mut self, sqrt_price: u128, tick: i32) {
        self.sqrt_price = sqrt_price;
        self.current_tick = tick;
    }
    
    /// Add liquidity
    pub fn add_liquidity(&mut self, delta: u128) -> Result<()> {
        self.liquidity = self.liquidity
            .checked_add(delta)
            .ok_or(FeelsError::MathOverflow)?;
        Ok(())
    }
    
    /// Remove liquidity
    pub fn remove_liquidity(&mut self, delta: u128) -> Result<()> {
        self.liquidity = self.liquidity
            .checked_sub(delta)
            .ok_or(FeelsError::MathUnderflow)?;
        Ok(())
    }
    
    /// Update fee growth
    pub fn update_fee_growth(&mut self, fee_0: [u64; 4], fee_1: [u64; 4]) {
        self.fee_growth_global_0 = fee_0;
        self.fee_growth_global_1 = fee_1;
    }
    
    /// Record volume
    pub fn record_volume(&mut self, amount_0: u64, amount_1: u64) -> Result<()> {
        self.total_volume_0 = self.total_volume_0
            .checked_add(amount_0 as u128)
            .ok_or(FeelsError::MathOverflow)?;
        self.total_volume_1 = self.total_volume_1
            .checked_add(amount_1 as u128)
            .ok_or(FeelsError::MathOverflow)?;
        Ok(())
    }
}

/// Domain weights helper (matches the existing structure)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct DomainWeights {
    pub w_s: u32,
    pub w_t: u32,
    pub w_l: u32,
    pub w_tau: u32,
}

impl DomainWeights {
    pub fn validate(&self) -> Result<()> {
        // Trading weights must sum to 10000
        let trade_sum = self.w_s + self.w_t + self.w_l;
        require!(
            trade_sum == 10000,
            FeelsError::InvalidWeightSum
        );
        
        // Buffer weight must be reasonable
        require!(
            self.w_tau <= 5000,
            FeelsError::InvalidWeights
        );
        
        Ok(())
    }
}

// Re-import necessary modules
use crate::math::tick as tick_math;