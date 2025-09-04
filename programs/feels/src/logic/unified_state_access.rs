/// Unified State Access Layer - Simplified interface for unified Market account
/// 
/// This module provides a clean API for accessing and modifying state
/// with the unified Market account that combines MarketField and MarketManager.

use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::FeelsError;
use feels_core::constants::*;
use std::collections::HashMap;

// ============================================================================
// Unified State Context - Single entry point for all state access
// ============================================================================

/// Simplified state context that manages all state operations with unified Market
pub struct UnifiedStateContext<'info> {
    /// Market state access
    pub market: MarketAccess<'info>,
    /// Tick state access
    pub ticks: TickStateAccess<'info>,
    /// Position state access
    pub positions: PositionStateAccess<'info>,
    /// Buffer/fee state access
    pub buffer: BufferStateAccess<'info>,
}

impl<'info> UnifiedStateContext<'info> {
    /// Create new state context from instruction accounts
    pub fn new(
        market: &'info Account<'info, Market>,
        buffer_account: &'info Account<'info, BufferAccount>,
        tick_arrays: Vec<&'info AccountLoader<'info, TickArray>>,
    ) -> Result<Self> {
        Ok(Self {
            market: MarketAccess::new(market)?,
            ticks: TickStateAccess::new(tick_arrays)?,
            positions: PositionStateAccess::new(),
            buffer: BufferStateAccess::new(buffer_account),
        })
    }
    
    /// Create new state context from UnifiedWorkUnit
    pub fn new_from_work_unit(
        work_unit: &mut crate::logic::UnifiedWorkUnit<'info>,
        tick_arrays: Vec<AccountLoader<'info, TickArray>>,
    ) -> Result<Self> {
        // Extract the market from work unit
        let market = work_unit.get_market()?;
        
        // Create market access with pending changes from work unit
        let market_access = MarketAccess {
            market,
            modified: work_unit.has_market_changes(),
        };
        
        // Create tick state access that will track changes through work unit
        let tick_arrays_refs: Vec<&'info AccountLoader<'info, TickArray>> = 
            tick_arrays.iter().collect();
        let tick_access = TickStateAccess::new(tick_arrays_refs)?;
        
        // Buffer access from work unit
        let buffer = work_unit.get_buffer()?;
        let buffer_access = BufferStateAccess {
            state: BufferState {
                accumulated_fees_0: buffer.accumulated_fees_0,
                accumulated_fees_1: buffer.accumulated_fees_1,
                rebates_paid_0: buffer.rebates_paid_0,
                rebates_paid_1: buffer.rebates_paid_1,
            },
            modified: true, // Assume modified if in work unit
        };
        
        Ok(Self {
            market: market_access,
            ticks: tick_access,
            positions: PositionStateAccess::new(),
            buffer: buffer_access,
        })
    }
    
    /// Commit all state changes
    pub fn commit(self) -> Result<()> {
        self.ticks.commit()?;
        // Market and buffer commits are handled by WorkUnit
        Ok(())
    }
}

// ============================================================================
// Market Access - Unified Market Account
// ============================================================================

pub struct MarketAccess<'info> {
    /// Reference to unified market account
    market: &'info Market,
    /// Track if state was modified
    modified: bool,
}

impl<'info> MarketAccess<'info> {
    fn new(market: &'info Account<'info, Market>) -> Result<Self> {
        // Validate market
        require!(market.is_initialized, FeelsError::NotInitialized);
        require!(!market.is_paused, FeelsError::InvalidOperation);
        
        Ok(Self {
            market,
            modified: false,
        })
    }
    
    // ========== Thermodynamic State Getters ==========
    
    /// Get S scalar
    pub fn s(&self) -> u128 {
        self.market.S
    }
    
    /// Get T scalar
    pub fn t(&self) -> u128 {
        self.market.T
    }
    
    /// Get L scalar
    pub fn l(&self) -> u128 {
        self.market.L
    }
    
    /// Get domain weights
    pub fn weights(&self) -> DomainWeights {
        self.market.get_domain_weights()
    }
    
    // ========== AMM State Getters ==========
    
    /// Get current price
    pub fn sqrt_price(&self) -> u128 {
        self.market.sqrt_price
    }
    
    /// Get current tick
    pub fn current_tick(&self) -> i32 {
        self.market.current_tick
    }
    
    /// Get current liquidity
    pub fn liquidity(&self) -> u128 {
        self.market.liquidity
    }
    
    /// Get fee parameters
    pub fn base_fee_bps(&self) -> u16 {
        self.market.base_fee_bps
    }
    
    /// Get max fee
    pub fn max_fee_bps(&self) -> u16 {
        self.market.max_fee_bps
    }
    
    // ========== Token Information ==========
    
    /// Get token 0 mint
    pub fn token_0(&self) -> Pubkey {
        self.market.token_0
    }
    
    /// Get token 1 mint
    pub fn token_1(&self) -> Pubkey {
        self.market.token_1
    }
    
    /// Get vault 0
    pub fn vault_0(&self) -> Pubkey {
        self.market.vault_0
    }
    
    /// Get vault 1
    pub fn vault_1(&self) -> Pubkey {
        self.market.vault_1
    }
    
    // ========== Volatility Parameters ==========
    
    /// Get price volatility
    pub fn sigma_price(&self) -> u64 {
        self.market.sigma_price
    }
    
    /// Get rate volatility
    pub fn sigma_rate(&self) -> u64 {
        self.market.sigma_rate
    }
    
    /// Get leverage volatility
    pub fn sigma_leverage(&self) -> u64 {
        self.market.sigma_leverage
    }
    
    // ========== Full Market Reference ==========
    
    /// Get full market reference for complex operations
    pub fn market(&self) -> &Market {
        self.market
    }
}

// ============================================================================
// Tick State Access (Same as before)
// ============================================================================

pub struct TickStateAccess<'info> {
    /// Tick array loaders by start index
    arrays: HashMap<i32, &'info AccountLoader<'info, TickArray>>,
    /// Cached tick modifications
    modifications: HashMap<i32, TickModification>,
}

impl<'info> TickStateAccess<'info> {
    fn new(tick_arrays: Vec<&'info AccountLoader<'info, TickArray>>) -> Result<Self> {
        let mut arrays = HashMap::new();
        
        for array_loader in tick_arrays {
            let array = array_loader.load()?;
            arrays.insert(array.start_tick_index, array_loader);
        }
        
        Ok(Self {
            arrays,
            modifications: HashMap::new(),
        })
    }
    
    /// Get tick data
    pub fn get_tick(&self, tick: i32) -> Result<Tick> {
        let array_start = self.get_array_start(tick);
        let offset = self.get_array_offset(tick)?;
        
        // Check modifications first
        if let Some(mod_) = self.modifications.get(&tick) {
            return Ok(mod_.to_tick());
        }
        
        // Load from array
        let array_loader = self.arrays.get(&array_start)
            .ok_or(FeelsError::TickArrayNotFound)?;
        let array = array_loader.load()?;
        
        Ok(array.ticks[offset].clone())
    }
    
    /// Update tick liquidity
    pub fn update_tick(
        &mut self,
        tick: i32,
        liquidity_delta: i128,
        fee_growth_0: u128,
        fee_growth_1: u128,
    ) -> Result<i128> {
        let mut tick_data = self.get_tick(tick)?;
        let liquidity_before = tick_data.liquidity_net;
        
        // Update liquidity
        tick_data.liquidity_net = tick_data.liquidity_net
            .saturating_add(liquidity_delta);
            
        if liquidity_delta > 0 {
            tick_data.liquidity_gross = tick_data.liquidity_gross
                .checked_add(liquidity_delta as u128)
                .ok_or(FeelsError::MathOverflow)?;
        } else {
            tick_data.liquidity_gross = tick_data.liquidity_gross
                .checked_sub((-liquidity_delta) as u128)
                .ok_or(FeelsError::MathUnderflow)?;
        }
        
        // Initialize if needed
        if !tick_data.initialized && tick_data.liquidity_gross > 0 {
            tick_data.initialized = true;
            tick_data.fee_growth_outside_0 = fee_growth_0;
            tick_data.fee_growth_outside_1 = fee_growth_1;
        }
        
        // Store modification
        self.modifications.insert(tick, TickModification {
            liquidity_net: tick_data.liquidity_net,
            liquidity_gross: tick_data.liquidity_gross,
            fee_growth_outside_0: tick_data.fee_growth_outside_0,
            fee_growth_outside_1: tick_data.fee_growth_outside_1,
            initialized: tick_data.initialized,
        });
        
        Ok(liquidity_before)
    }
    
    /// Commit changes back to on-chain state
    fn commit(self) -> Result<()> {
        for (tick, modification) in self.modifications {
            let array_start = self.get_array_start(tick);
            let offset = self.get_array_offset(tick)?;
            
            if let Some(array_loader) = self.arrays.get(&array_start) {
                let mut array = array_loader.load_mut()?;
                let tick_ref = &mut array.ticks[offset];
                
                tick_ref.liquidity_net = modification.liquidity_net;
                tick_ref.liquidity_gross = modification.liquidity_gross;
                tick_ref.fee_growth_outside_0 = modification.fee_growth_outside_0;
                tick_ref.fee_growth_outside_1 = modification.fee_growth_outside_1;
                tick_ref.initialized = modification.initialized;
            }
        }
        Ok(())
    }
    
    fn get_array_start(&self, tick: i32) -> i32 {
        (tick / TICK_ARRAY_SIZE as i32) * TICK_ARRAY_SIZE as i32
    }
    
    fn get_array_offset(&self, tick: i32) -> Result<usize> {
        let offset = (tick - self.get_array_start(tick)) as usize;
        require!(offset < TICK_ARRAY_SIZE, FeelsError::InvalidTickIndex);
        Ok(offset)
    }
}

// ============================================================================
// Position State Access (Same as before)
// ============================================================================

pub struct PositionStateAccess<'info> {
    /// Position modifications
    positions: HashMap<u64, PositionData>,
    /// Next position ID
    next_id: u64,
}

impl<'info> PositionStateAccess<'info> {
    fn new() -> Self {
        Self {
            positions: HashMap::new(),
            next_id: Clock::get().unwrap().unix_timestamp as u64,
        }
    }
    
    /// Create new position
    pub fn create_position(
        &mut self,
        owner: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        self.positions.insert(id, PositionData {
            owner,
            tick_lower,
            tick_upper,
            liquidity,
            fee_growth_inside_0: 0,
            fee_growth_inside_1: 0,
            tokens_owed_0: 0,
            tokens_owed_1: 0,
        });
        
        id
    }
}

// ============================================================================
// Buffer State Access (Same as before)
// ============================================================================

pub struct BufferStateAccess<'info> {
    /// Cached state
    state: BufferState,
    /// Track modifications
    modified: bool,
}

impl<'info> BufferStateAccess<'info> {
    fn new(account: &'info Account<'info, BufferAccount>) -> Self {
        Self {
            state: BufferState {
                accumulated_fees_0: account.accumulated_fees_0,
                accumulated_fees_1: account.accumulated_fees_1,
                rebates_paid_0: account.rebates_paid_0,
                rebates_paid_1: account.rebates_paid_1,
            },
            modified: false,
        }
    }
    
    /// Record fees collected
    pub fn collect_fees(&mut self, token_0: bool, amount: u64) -> Result<()> {
        if token_0 {
            self.state.accumulated_fees_0 = self.state.accumulated_fees_0
                .checked_add(amount)
                .ok_or(FeelsError::MathOverflow)?;
        } else {
            self.state.accumulated_fees_1 = self.state.accumulated_fees_1
                .checked_add(amount)
                .ok_or(FeelsError::MathOverflow)?;
        }
        self.modified = true;
        Ok(())
    }
}

// ============================================================================
// Helper Types
// ============================================================================

#[derive(Debug, Clone)]
struct TickModification {
    liquidity_net: i128,
    liquidity_gross: u128,
    fee_growth_outside_0: u128,
    fee_growth_outside_1: u128,
    initialized: bool,
}

impl TickModification {
    fn to_tick(&self) -> Tick {
        Tick {
            liquidity_net: self.liquidity_net,
            liquidity_gross: self.liquidity_gross,
            fee_growth_outside_0: self.fee_growth_outside_0,
            fee_growth_outside_1: self.fee_growth_outside_1,
            initialized: self.initialized,
            _padding: [0; 76],
        }
    }
}

#[derive(Debug, Clone)]
struct PositionData {
    owner: Pubkey,
    tick_lower: i32,
    tick_upper: i32,
    liquidity: u128,
    fee_growth_inside_0: u128,
    fee_growth_inside_1: u128,
    tokens_owed_0: u64,
    tokens_owed_1: u64,
}

#[derive(Debug, Clone)]
struct BufferState {
    accumulated_fees_0: u64,
    accumulated_fees_1: u64,
    rebates_paid_0: u64,
    rebates_paid_1: u64,
}