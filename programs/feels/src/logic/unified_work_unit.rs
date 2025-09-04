/// Unified Work Unit Pattern for State Management
/// 
/// Updated version that uses the unified Market account instead of
/// separate MarketField and MarketManager accounts.

use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::error::FeelsProtocolError;
use crate::state::{
    Market, BufferAccount, UnifiedOracle,
    TickArray, TickPositionMetadata, ProtocolState, MarketDataSource,
};

// ============================================================================
// Core Unit of Work Types
// ============================================================================

/// Unique identifier for tracked state
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StateKey {
    Market(Pubkey),
    Buffer(Pubkey),
    UnifiedOracle(Pubkey),
    TickArray(Pubkey),
    Position(Pubkey),
    Protocol(Pubkey),
    MarketDataSource(Pubkey),
}

/// Represents a tracked state change
#[derive(Clone)]
pub enum StateChange {
    Market(Box<Market>),
    Buffer(Box<BufferAccount>),
    UnifiedOracle(Box<UnifiedOracle>),
    TickArray(Box<TickArray>),
    Position(Box<TickPositionMetadata>),
    Protocol(Box<ProtocolState>),
    MarketDataSource(Box<MarketDataSource>),
}

/// Tracks account info for writing back changes
pub struct AccountTracker<'info> {
    pub account_info: &'info AccountInfo<'info>,
    pub is_zero_copy: bool,
}

// ============================================================================
// Unit of Work Implementation
// ============================================================================

/// Unit of Work pattern for managing state changes
pub struct WorkUnit<'info> {
    /// Original loaded states (for rollback if needed)
    original_states: HashMap<StateKey, StateChange>,
    
    /// Pending changes to be committed
    pending_changes: HashMap<StateKey, StateChange>,
    
    /// Account info for each tracked state
    account_trackers: HashMap<StateKey, AccountTracker<'info>>,
    
    /// Track if unit has been committed
    committed: bool,
}

impl<'info> WorkUnit<'info> {
    /// Create a new unit of work
    pub fn new() -> Self {
        Self {
            original_states: HashMap::new(),
            pending_changes: HashMap::new(),
            account_trackers: HashMap::new(),
            committed: false,
        }
    }
    
    // ========================================================================
    // State Loading Functions
    // ========================================================================
    
    /// Load and track unified Market account
    pub fn load_market(
        &mut self,
        account: &'info Account<'info, Market>,
    ) -> Result<&mut Market> {
        let key = StateKey::Market(account.key());
        
        // Store original state if not already loaded
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::Market(Box::new(account.clone().into_inner())),
            );
            
            // Track account info
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account.to_account_info().as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        // Get or create pending change
        if !self.pending_changes.contains_key(&key) {
            let original = self.original_states.get(&key).unwrap();
            if let StateChange::Market(market) = original {
                self.pending_changes.insert(
                    key.clone(),
                    StateChange::Market(market.clone()),
                );
            }
        }
        
        // Return mutable reference to pending state
        if let Some(StateChange::Market(market)) = self.pending_changes.get_mut(&key) {
            Ok(market)
        } else {
            Err(FeelsProtocolError::StateError.into())
        }
    }
    
    /// Load and track BufferAccount
    pub fn load_buffer(
        &mut self,
        account: &'info Account<'info, BufferAccount>,
    ) -> Result<&mut BufferAccount> {
        let key = StateKey::Buffer(account.key());
        
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::Buffer(Box::new(account.clone().into_inner())),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account.to_account_info().as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        if !self.pending_changes.contains_key(&key) {
            let original = self.original_states.get(&key).unwrap();
            if let StateChange::Buffer(buffer) = original {
                self.pending_changes.insert(
                    key.clone(),
                    StateChange::Buffer(buffer.clone()),
                );
            }
        }
        
        if let Some(StateChange::Buffer(buffer)) = self.pending_changes.get_mut(&key) {
            Ok(buffer)
        } else {
            Err(FeelsProtocolError::StateError.into())
        }
    }
    
    /// Load and track TickArray (zero-copy)
    pub fn load_tick_array(
        &mut self,
        account_loader: &'info AccountLoader<'info, TickArray>,
    ) -> Result<TickArray> {
        let key = StateKey::TickArray(account_loader.key());
        let tick_array = account_loader.load()?;
        
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::TickArray(Box::new(tick_array.clone())),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account_loader.to_account_info().as_ref(),
                    is_zero_copy: true,
                },
            );
            
            self.pending_changes.insert(
                key.clone(),
                StateChange::TickArray(Box::new(tick_array.clone())),
            );
        }
        
        Ok(tick_array.clone())
    }
    
    /// Update tick array in pending changes
    pub fn update_tick_array(&mut self, pubkey: Pubkey, tick_array: TickArray) -> Result<()> {
        let key = StateKey::TickArray(pubkey);
        self.pending_changes.insert(
            key,
            StateChange::TickArray(Box::new(tick_array)),
        );
        Ok(())
    }
    
    // ========================================================================
    // State Access Functions
    // ========================================================================
    
    /// Get current market state
    pub fn get_market(&self) -> Result<&Market> {
        for (_, change) in &self.pending_changes {
            if let StateChange::Market(market) = change {
                return Ok(market);
            }
        }
        Err(FeelsProtocolError::NotFound.into())
    }
    
    /// Get mutable market state
    pub fn get_market_mut(&mut self) -> Result<&mut Market> {
        for (_, change) in &mut self.pending_changes {
            if let StateChange::Market(market) = change {
                return Ok(market);
            }
        }
        Err(FeelsProtocolError::NotFound.into())
    }
    
    /// Get buffer state
    pub fn get_buffer(&self) -> Result<&BufferAccount> {
        for (_, change) in &self.pending_changes {
            if let StateChange::Buffer(buffer) = change {
                return Ok(buffer);
            }
        }
        Err(FeelsProtocolError::NotFound.into())
    }
    
    /// Get mutable buffer state
    pub fn get_buffer_mut(&mut self) -> Result<&mut BufferAccount> {
        for (_, change) in &mut self.pending_changes {
            if let StateChange::Buffer(buffer) = change {
                return Ok(buffer);
            }
        }
        Err(FeelsProtocolError::NotFound.into())
    }
    
    /// Check if market has pending changes
    pub fn has_market_changes(&self) -> bool {
        for key in self.pending_changes.keys() {
            if matches!(key, StateKey::Market(_)) {
                return true;
            }
        }
        false
    }
    
    // ========================================================================
    // Commit Functionality
    // ========================================================================
    
    /// Commit all pending changes back to accounts
    pub fn commit(mut self) -> Result<()> {
        require!(!self.committed, FeelsProtocolError::StateError);
        
        // Write each pending change back to its account
        for (key, change) in self.pending_changes.drain() {
            let tracker = self.account_trackers.get(&key)
                .ok_or(FeelsProtocolError::StateError)?;
            
            match change {
                StateChange::Market(market) => {
                    // Serialize and write to account
                    let mut data = tracker.account_info.try_borrow_mut_data()?;
                    market.try_serialize(&mut data.as_mut())?;
                }
                StateChange::Buffer(buffer) => {
                    let mut data = tracker.account_info.try_borrow_mut_data()?;
                    buffer.try_serialize(&mut data.as_mut())?;
                }
                StateChange::TickArray(tick_array) => {
                    if tracker.is_zero_copy {
                        // For zero-copy accounts, we need special handling
                        let mut data = tracker.account_info.try_borrow_mut_data()?;
                        let dst: &mut [u8] = &mut data;
                        let src = bytemuck::bytes_of(&*tick_array);
                        dst[8..8 + src.len()].copy_from_slice(src);
                    }
                }
                StateChange::UnifiedOracle(oracle) => {
                    let mut data = tracker.account_info.try_borrow_mut_data()?;
                    oracle.try_serialize(&mut data.as_mut())?;
                }
                StateChange::Position(position) => {
                    let mut data = tracker.account_info.try_borrow_mut_data()?;
                    position.try_serialize(&mut data.as_mut())?;
                }
                StateChange::Protocol(protocol) => {
                    let mut data = tracker.account_info.try_borrow_mut_data()?;
                    protocol.try_serialize(&mut data.as_mut())?;
                }
                StateChange::MarketDataSource(data_source) => {
                    if tracker.is_zero_copy {
                        let mut data = tracker.account_info.try_borrow_mut_data()?;
                        let dst: &mut [u8] = &mut data;
                        let src = bytemuck::bytes_of(&*data_source);
                        dst[8..8 + src.len()].copy_from_slice(src);
                    }
                }
            }
        }
        
        self.committed = true;
        Ok(())
    }
    
    /// Rollback all pending changes (automatic on drop if not committed)
    pub fn rollback(mut self) {
        self.pending_changes.clear();
        self.committed = true; // Prevent commit on drop
    }
}

impl<'info> Drop for WorkUnit<'info> {
    fn drop(&mut self) {
        if !self.committed && !self.pending_changes.is_empty() {
            msg!("Warning: WorkUnit dropped without commit, changes lost");
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

impl<'info> WorkUnit<'info> {
    /// Update market scalars
    pub fn update_market_scalars(&mut self, s: u128, t: u128, l: u128) -> Result<()> {
        let market = self.get_market_mut()?;
        market.update_scalars(s, t, l);
        Ok(())
    }
    
    /// Update price and tick
    pub fn update_price(&mut self, sqrt_price: u128, tick: i32) -> Result<()> {
        let market = self.get_market_mut()?;
        market.update_price(sqrt_price, tick);
        Ok(())
    }
    
    /// Add liquidity
    pub fn add_liquidity(&mut self, delta: u128) -> Result<()> {
        let market = self.get_market_mut()?;
        market.add_liquidity(delta)
    }
    
    /// Remove liquidity
    pub fn remove_liquidity(&mut self, delta: u128) -> Result<()> {
        let market = self.get_market_mut()?;
        market.remove_liquidity(delta)
    }
    
    /// Record volume
    pub fn record_volume(&mut self, amount_0: u64, amount_1: u64) -> Result<()> {
        let market = self.get_market_mut()?;
        market.record_volume(amount_0, amount_1)
    }
    
    /// Update buffer fees
    pub fn collect_fees(&mut self, token_0: bool, amount: u64) -> Result<()> {
        let buffer = self.get_buffer_mut()?;
        if token_0 {
            buffer.accumulated_fees_0 = buffer.accumulated_fees_0
                .checked_add(amount)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        } else {
            buffer.accumulated_fees_1 = buffer.accumulated_fees_1
                .checked_add(amount)
                .ok_or(FeelsProtocolError::MathOverflow)?;
        }
        Ok(())
    }
}