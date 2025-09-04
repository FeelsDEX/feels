/// State Access Layer - Centralized interface for all on-chain state operations
/// This abstracts away direct account access, providing clean APIs for state management
use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::FeelsError;
use feels_core::constants::*;
use std::collections::HashMap;

// ============================================================================
// State Context - Single entry point for all state access
// ============================================================================

/// Unified state context that manages all state operations
pub struct StateContext<'info> {
    /// Market state access
    pub market: MarketStateAccess<'info>,
    /// Tick state access
    pub ticks: TickStateAccess<'info>,
    /// Position state access
    pub positions: PositionStateAccess<'info>,
    /// Buffer/fee state access
    pub buffer: BufferStateAccess<'info>,
}

impl<'info> StateContext<'info> {
    /// Create new state context from instruction accounts
    pub fn new(
        market_field: &'info Account<'info, MarketField>,
        market_manager: &'info AccountLoader<'info, MarketManager>,
        buffer_account: &'info Account<'info, BufferAccount>,
        tick_arrays: Vec<&'info AccountLoader<'info, TickArray>>,
    ) -> Result<Self> {
        Ok(Self {
            market: MarketStateAccess::new(market_field, market_manager)?,
            ticks: TickStateAccess::new(tick_arrays)?,
            positions: PositionStateAccess::new(),
            buffer: BufferStateAccess::new(buffer_account),
        })
    }
    
    /// Create new state context from WorkUnit - integrates with unit of work pattern
    pub fn new_from_work_unit(
        work_unit: &mut crate::logic::WorkUnit<'info>,
        tick_arrays: Vec<AccountLoader<'info, TickArray>>,
    ) -> Result<Self> {
        // Extract the market state from work unit
        let market_state = work_unit.get_market_state()?;
        let market_field = work_unit.get_market_field()?;
        
        // Create market state access with pending changes from work unit
        let market_access = MarketStateAccess {
            field: market_field,
            manager_loader: work_unit.market_manager.clone(),
            state: market_state.clone(),
            modified: work_unit.has_market_changes(),
        };
        
        // Create tick state access that will track changes through work unit
        let tick_access = TickStateAccess::new(tick_arrays)?;
        
        // Buffer access can be created normally
        let buffer_access = BufferStateAccess::new(work_unit.circular_buffer.clone());
        
        Ok(Self {
            market: market_access,
            ticks: tick_access,
            buffer: buffer_access,
        })
    }
    
    /// Commit all state changes
    pub fn commit(self) -> Result<()> {
        self.market.commit()?;
        self.ticks.commit()?;
        self.buffer.commit()?;
        Ok(())
    }
}

// ============================================================================
// Market State Access
// ============================================================================

pub struct MarketStateAccess<'info> {
    /// Immutable market field
    field: &'info Account<'info, MarketField>,
    /// Market manager loader
    manager_loader: &'info AccountLoader<'info, MarketManager>,
    /// Cached mutable state
    state: MarketState,
    /// Track if state was modified
    modified: bool,
}

impl<'info> MarketStateAccess<'info> {
    fn new(
        field: &'info Account<'info, MarketField>,
        manager_loader: &'info AccountLoader<'info, MarketManager>,
    ) -> Result<Self> {
        let manager = manager_loader.load()?;
        
        // Validate market
        require!(manager.is_initialized, FeelsError::NotInitialized);
        require!(!field.is_paused, FeelsError::InvalidOperation);
        
        Ok(Self {
            field,
            manager_loader,
            state: MarketState {
                sqrt_price: manager.sqrt_price,
                current_tick: manager.current_tick,
                liquidity: manager.liquidity,
                fee_growth_global_0: manager.fee_growth_global_0,
                fee_growth_global_1: manager.fee_growth_global_1,
                feelssol_reserves: manager.feelssol_reserves,
                total_volume_0: manager.total_volume_0,
                total_volume_1: manager.total_volume_1,
            },
            modified: false,
        })
    }
    
    /// Get current price
    pub fn sqrt_price(&self) -> u128 {
        self.state.sqrt_price
    }
    
    /// Get current tick
    pub fn current_tick(&self) -> i32 {
        self.state.current_tick
    }
    
    /// Get current liquidity
    pub fn liquidity(&self) -> u128 {
        self.state.liquidity
    }
    
    /// Get market parameters
    pub fn params(&self) -> &MarketField {
        self.field
    }
    
    /// Update price and tick after swap
    pub fn update_price(&mut self, sqrt_price: u128, tick: i32) {
        self.state.sqrt_price = sqrt_price;
        self.state.current_tick = tick;
        self.modified = true;
    }
    
    /// Update liquidity
    pub fn update_liquidity(&mut self, liquidity: u128) {
        self.state.liquidity = liquidity;
        self.modified = true;
    }
    
    /// Update fee growth
    pub fn update_fee_growth(&mut self, token_0: bool, fee_growth: u128) {
        if token_0 {
            self.state.fee_growth_global_0 = fee_growth;
        } else {
            self.state.fee_growth_global_1 = fee_growth;
        }
        self.modified = true;
    }
    
    /// Record volume
    pub fn record_volume(&mut self, token_0: bool, amount: u64) -> Result<()> {
        if token_0 {
            self.state.total_volume_0 = self.state.total_volume_0
                .checked_add(amount as u128)
                .ok_or(FeelsError::MathOverflow)?;
        } else {
            self.state.total_volume_1 = self.state.total_volume_1
                .checked_add(amount as u128)
                .ok_or(FeelsError::MathOverflow)?;
        }
        self.modified = true;
        Ok(())
    }
    
    /// Update FeelsSOL reserves
    pub fn update_reserves(&mut self, new_reserves: u64) {
        self.state.feelssol_reserves = new_reserves;
        self.modified = true;
    }
    
    /// Commit changes back to on-chain state
    fn commit(self) -> Result<()> {
        if self.modified {
            let mut manager = self.manager_loader.load_mut()?;
            manager.sqrt_price = self.state.sqrt_price;
            manager.current_tick = self.state.current_tick;
            manager.liquidity = self.state.liquidity;
            manager.fee_growth_global_0 = self.state.fee_growth_global_0;
            manager.fee_growth_global_1 = self.state.fee_growth_global_1;
            manager.feelssol_reserves = self.state.feelssol_reserves;
            manager.total_volume_0 = self.state.total_volume_0;
            manager.total_volume_1 = self.state.total_volume_1;
            manager.last_update = Clock::get()?.unix_timestamp;
        }
        Ok(())
    }
}

// ============================================================================
// Tick State Access
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
    
    /// Find next initialized tick using bitmap-optimized search
    pub fn find_next_initialized(
        &self,
        current: i32,
        search_down: bool,
    ) -> Result<(i32, bool)> {
        // First check current array if we have it
        let array_start = self.get_array_start(current);
        if let Some(array_loader) = self.arrays.get(&array_start) {
            let array = array_loader.load()?;
            
            // Build a bitmap for this array for efficient search
            let mut bitmap = 0u64;
            let offset = (current - array_start) as usize;
            
            // Set bits for initialized ticks
            for (i, tick) in array.ticks.iter().enumerate() {
                if tick.initialized {
                    bitmap |= 1u64 << i;
                }
            }
            
            // Use bitmap to find next initialized tick
            if search_down {
                // Search for previous set bit
                if offset > 0 {
                    let mask = (1u64 << offset) - 1;
                    let masked = bitmap & mask;
                    if masked != 0 {
                        let bit_pos = 63 - masked.leading_zeros() as usize;
                        return Ok((array_start + bit_pos as i32, true));
                    }
                }
            } else {
                // Search for next set bit
                if offset < TICK_ARRAY_SIZE - 1 {
                    let mask = u64::MAX << (offset + 1);
                    let masked = bitmap & mask;
                    if masked != 0 {
                        let bit_pos = masked.trailing_zeros() as usize;
                        return Ok((array_start + bit_pos as i32, true));
                    }
                }
            }
        }
        
        // If not found in current array, we need to search other arrays
        // Return boundary to signal continuation needed
        if search_down {
            Ok((array_start - 1, false))
        } else {
            Ok((array_start + TICK_ARRAY_SIZE as i32, false))
        }
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
    
    /// Cross tick and update fee growth
    pub fn cross_tick(
        &mut self,
        tick: i32,
        fee_growth_global_0: u128,
        fee_growth_global_1: u128,
    ) -> Result<i128> {
        let mut tick_data = self.get_tick(tick)?;
        
        // Flip fee growth
        tick_data.fee_growth_outside_0 = fee_growth_global_0
            .wrapping_sub(tick_data.fee_growth_outside_0);
        tick_data.fee_growth_outside_1 = fee_growth_global_1
            .wrapping_sub(tick_data.fee_growth_outside_1);
        
        self.modifications.insert(tick, TickModification {
            liquidity_net: tick_data.liquidity_net,
            liquidity_gross: tick_data.liquidity_gross,
            fee_growth_outside_0: tick_data.fee_growth_outside_0,
            fee_growth_outside_1: tick_data.fee_growth_outside_1,
            initialized: tick_data.initialized,
        });
        
        Ok(tick_data.liquidity_net)
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
// Position State Access
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
    
    /// Update position fees
    pub fn update_position_fees(
        &mut self,
        id: u64,
        fee_growth_inside_0: u128,
        fee_growth_inside_1: u128,
    ) -> Result<()> {
        let position = self.positions.get_mut(&id)
            .ok_or(FeelsError::NotFound)?;
        
        position.fee_growth_inside_0 = fee_growth_inside_0;
        position.fee_growth_inside_1 = fee_growth_inside_1;
        
        Ok(())
    }
}

// ============================================================================
// Buffer State Access
// ============================================================================

pub struct BufferStateAccess<'info> {
    /// Buffer account
    account: &'info Account<'info, BufferAccount>,
    /// Cached state
    state: BufferState,
    /// Track modifications
    modified: bool,
}

impl<'info> BufferStateAccess<'info> {
    fn new(account: &'info Account<'info, BufferAccount>) -> Self {
        Self {
            account,
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
    
    /// Record rebates paid
    pub fn pay_rebate(&mut self, token_0: bool, amount: u64) -> Result<()> {
        if token_0 {
            self.state.rebates_paid_0 = self.state.rebates_paid_0
                .checked_add(amount)
                .ok_or(FeelsError::MathOverflow)?;
        } else {
            self.state.rebates_paid_1 = self.state.rebates_paid_1
                .checked_add(amount)
                .ok_or(FeelsError::MathOverflow)?;
        }
        self.modified = true;
        Ok(())
    }
    
    fn commit(self) -> Result<()> {
        if self.modified {
            let account = &mut self.account.try_borrow_mut_data()?;
            // Would update account data directly
        }
        Ok(())
    }
}

// ============================================================================
// State Types
// ============================================================================

#[derive(Debug, Clone)]
struct MarketState {
    sqrt_price: u128,
    current_tick: i32,
    liquidity: u128,
    fee_growth_global_0: u128,
    fee_growth_global_1: u128,
    feelssol_reserves: u64,
    total_volume_0: u128,
    total_volume_1: u128,
}

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