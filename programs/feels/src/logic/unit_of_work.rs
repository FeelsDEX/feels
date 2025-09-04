/// Unit of Work Pattern for State Management
/// 
/// Tracks all loaded accounts and pending state changes during instruction execution.
/// Provides atomic commit functionality to write all changes back to accounts.
/// Simplifies transaction management and improves error handling.

use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::error::FeelsProtocolError;
use crate::state::{
    MarketField, BufferAccount, MarketManager, UnifiedOracle,
    TickArray, TickPositionMetadata, ProtocolState, MarketDataSource,
};

// ============================================================================
// Core Unit of Work Types
// ============================================================================

/// Unique identifier for tracked state
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StateKey {
    MarketField(Pubkey),
    Buffer(Pubkey),
    MarketManager(Pubkey),
    UnifiedOracle(Pubkey),
    TickArray(Pubkey),
    Position(Pubkey),
    Protocol(Pubkey),
    MarketDataSource(Pubkey),
}

/// Represents a tracked state change
#[derive(Clone)]
pub enum StateChange {
    MarketField(Box<MarketField>),
    Buffer(Box<BufferAccount>),
    MarketManager(Box<MarketManager>),
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
    
    /// Load and track MarketField account
    pub fn load_market_field(
        &mut self,
        account: &'info Account<'info, MarketField>,
    ) -> Result<&mut MarketField> {
        let key = StateKey::MarketField(account.key());
        
        // Store original state if not already loaded
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::MarketField(Box::new(account.clone().into_inner())),
            );
            
            // Track account info
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account.as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        // Create or get pending change
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::MarketField(field)) => (**field).clone(),
            _ => account.clone().into_inner(),
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::MarketField(Box::new(state)),
        );
        
        // Return mutable reference to pending state
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::MarketField(field) => Ok(field.as_mut()),
            _ => unreachable!(),
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
                    account_info: account.as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::Buffer(buffer)) => (**buffer).clone(),
            _ => account.clone().into_inner(),
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::Buffer(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::Buffer(buffer) => Ok(buffer.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track MarketManager (zero-copy)
    pub fn load_market_manager(
        &mut self,
        loader: &'info AccountLoader<'info, MarketManager>,
    ) -> Result<&mut MarketManager> {
        let key = StateKey::MarketManager(loader.key());
        
        if !self.original_states.contains_key(&key) {
            let original = loader.load()?;
            self.original_states.insert(
                key.clone(),
                StateChange::MarketManager(Box::new(*original)),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: loader.as_ref(),
                    is_zero_copy: true,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::MarketManager(mgr)) => (**mgr).clone(),
            _ => *loader.load()?,
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::MarketManager(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::MarketManager(mgr) => Ok(mgr.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track UnifiedOracle (zero-copy)
    pub fn load_twap_oracle(
        &mut self,
        loader: &'info AccountLoader<'info, UnifiedOracle>,
    ) -> Result<&mut UnifiedOracle> {
        let key = StateKey::UnifiedOracle(loader.key());
        
        if !self.original_states.contains_key(&key) {
            let original = loader.load()?;
            self.original_states.insert(
                key.clone(),
                StateChange::UnifiedOracle(Box::new(*original)),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: loader.as_ref(),
                    is_zero_copy: true,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::UnifiedOracle(oracle)) => (**oracle).clone(),
            _ => *loader.load()?,
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::UnifiedOracle(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::UnifiedOracle(oracle) => Ok(oracle.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track Position
    pub fn load_position(
        &mut self,
        account: &'info Account<'info, TickPositionMetadata>,
    ) -> Result<&mut TickPositionMetadata> {
        let key = StateKey::Position(account.key());
        
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::Position(Box::new(account.clone().into_inner())),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account.as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::Position(pos)) => (**pos).clone(),
            _ => account.clone().into_inner(),
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::Position(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::Position(pos) => Ok(pos.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track TickArray (zero-copy)
    pub fn load_tick_array(
        &mut self,
        loader: &'info AccountLoader<'info, TickArray>,
    ) -> Result<&mut TickArray> {
        let key = StateKey::TickArray(loader.key());
        
        if !self.original_states.contains_key(&key) {
            let original = loader.load()?;
            self.original_states.insert(
                key.clone(),
                StateChange::TickArray(Box::new(*original)),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: loader.as_ref(),
                    is_zero_copy: true,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::TickArray(arr)) => (**arr).clone(),
            _ => *loader.load()?,
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::TickArray(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::TickArray(arr) => Ok(arr.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track ProtocolState
    pub fn load_protocol_state(
        &mut self,
        account: &'info Account<'info, ProtocolState>,
    ) -> Result<&mut ProtocolState> {
        let key = StateKey::Protocol(account.key());
        
        if !self.original_states.contains_key(&key) {
            self.original_states.insert(
                key.clone(),
                StateChange::Protocol(Box::new(account.clone().into_inner())),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: account.as_ref(),
                    is_zero_copy: false,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::Protocol(proto)) => (**proto).clone(),
            _ => account.clone().into_inner(),
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::Protocol(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::Protocol(proto) => Ok(proto.as_mut()),
            _ => unreachable!(),
        }
    }
    
    /// Load and track MarketDataSource (zero-copy)
    pub fn load_market_data_source(
        &mut self,
        loader: &'info AccountLoader<'info, MarketDataSource>,
    ) -> Result<&mut MarketDataSource> {
        let key = StateKey::MarketDataSource(loader.key());
        
        if !self.original_states.contains_key(&key) {
            let original = loader.load()?;
            self.original_states.insert(
                key.clone(),
                StateChange::MarketDataSource(Box::new(*original)),
            );
            
            self.account_trackers.insert(
                key.clone(),
                AccountTracker {
                    account_info: loader.as_ref(),
                    is_zero_copy: true,
                },
            );
        }
        
        let state = match self.pending_changes.get(&key) {
            Some(StateChange::MarketDataSource(mds)) => (**mds).clone(),
            _ => *loader.load()?,
        };
        
        self.pending_changes.insert(
            key.clone(),
            StateChange::MarketDataSource(Box::new(state)),
        );
        
        match self.pending_changes.get_mut(&key).unwrap() {
            StateChange::MarketDataSource(mds) => Ok(mds.as_mut()),
            _ => unreachable!(),
        }
    }
    
    // ========================================================================
    // State Access Functions
    // ========================================================================
    
    /// Get mutable reference to tracked MarketField
    pub fn get_market_field_mut(&mut self, key: &Pubkey) -> Result<&mut MarketField> {
        let state_key = StateKey::MarketField(*key);
        match self.pending_changes.get_mut(&state_key) {
            Some(StateChange::MarketField(field)) => Ok(field.as_mut()),
            _ => Err(FeelsProtocolError::InvalidAccountData.into()),
        }
    }
    
    /// Get immutable reference to tracked MarketField
    pub fn get_market_field(&self, key: &Pubkey) -> Result<&MarketField> {
        let state_key = StateKey::MarketField(*key);
        match self.pending_changes.get(&state_key) {
            Some(StateChange::MarketField(field)) => Ok(field.as_ref()),
            _ => Err(FeelsProtocolError::InvalidAccountData.into()),
        }
    }
    
    /// Get mutable reference to tracked BufferAccount
    pub fn get_buffer_mut(&mut self, key: &Pubkey) -> Result<&mut BufferAccount> {
        let state_key = StateKey::Buffer(*key);
        match self.pending_changes.get_mut(&state_key) {
            Some(StateChange::Buffer(buffer)) => Ok(buffer.as_mut()),
            _ => Err(FeelsProtocolError::InvalidAccountData.into()),
        }
    }
    
    /// Get mutable reference to tracked MarketManager
    pub fn get_market_manager_mut(&mut self, key: &Pubkey) -> Result<&mut MarketManager> {
        let state_key = StateKey::MarketManager(*key);
        match self.pending_changes.get_mut(&state_key) {
            Some(StateChange::MarketManager(mgr)) => Ok(mgr.as_mut()),
            _ => Err(FeelsProtocolError::InvalidAccountData.into()),
        }
    }
    
    /// Get mutable reference to tracked MarketDataSource
    pub fn get_market_data_source_mut(&mut self, key: &Pubkey) -> Result<&mut MarketDataSource> {
        let state_key = StateKey::MarketDataSource(*key);
        match self.pending_changes.get_mut(&state_key) {
            Some(StateChange::MarketDataSource(mds)) => Ok(mds.as_mut()),
            _ => Err(FeelsProtocolError::InvalidAccountData.into()),
        }
    }
    
    // ========================================================================
    // Commit & Rollback
    // ========================================================================
    
    /// Commit all pending changes to accounts
    pub fn commit(mut self) -> Result<()> {
        // Mark as committed to prevent double commits
        if self.committed {
            return Err(FeelsProtocolError::InvalidState.into());
        }
        
        // Write each pending change back to its account
        for (key, change) in self.pending_changes.iter() {
            let tracker = self.account_trackers.get(key)
                .ok_or(FeelsProtocolError::InvalidAccountData)?;
            
            match change {
                StateChange::MarketField(field) => {
                    self.write_account(tracker.account_info, field.as_ref())?;
                }
                StateChange::Buffer(buffer) => {
                    self.write_account(tracker.account_info, buffer.as_ref())?;
                }
                StateChange::MarketManager(mgr) => {
                    self.write_zero_copy(tracker.account_info, mgr.as_ref())?;
                }
                StateChange::UnifiedOracle(oracle) => {
                    self.write_zero_copy(tracker.account_info, oracle.as_ref())?;
                }
                StateChange::TickArray(arr) => {
                    self.write_zero_copy(tracker.account_info, arr.as_ref())?;
                }
                StateChange::Position(pos) => {
                    self.write_account(tracker.account_info, pos.as_ref())?;
                }
                StateChange::Protocol(proto) => {
                    self.write_account(tracker.account_info, proto.as_ref())?;
                }
                StateChange::MarketDataSource(mds) => {
                    self.write_zero_copy(tracker.account_info, mds.as_ref())?;
                }
            }
        }
        
        self.committed = true;
        Ok(())
    }
    
    /// Write regular account data
    fn write_account<T: AccountSerialize>(
        &self,
        account_info: &AccountInfo,
        data: &T,
    ) -> Result<()> {
        let mut data_slice = account_info.try_borrow_mut_data()?;
        let mut cursor = std::io::Cursor::new(&mut data_slice[..]);
        data.try_serialize(&mut cursor)?;
        Ok(())
    }
    
    /// Write zero-copy account data
    fn write_zero_copy<T: bytemuck::Pod>(
        &self,
        account_info: &AccountInfo,
        data: &T,
    ) -> Result<()> {
        let mut data_slice = account_info.try_borrow_mut_data()?;
        let dst = &mut data_slice[8..]; // Skip discriminator
        let src = bytemuck::bytes_of(data);
        dst.copy_from_slice(src);
        Ok(())
    }
    
    /// Check if there are any pending changes
    pub fn has_changes(&self) -> bool {
        !self.pending_changes.is_empty()
    }
    
    /// Get the number of tracked accounts
    pub fn tracked_count(&self) -> usize {
        self.account_trackers.len()
    }
}

// ============================================================================
// Drop Implementation for Safety
// ============================================================================

impl<'info> Drop for WorkUnit<'info> {
    fn drop(&mut self) {
        // Log warning if unit of work was not committed
        if !self.committed && self.has_changes() {
            msg!("WARNING: WorkUnit dropped with uncommitted changes!");
        }
    }
}

// ============================================================================
// Helper Functions for Common Patterns
// ============================================================================

/// Create and initialize a WorkUnit with market accounts
pub fn create_market_work_unit<'info>(
    market_field: &'info Account<'info, MarketField>,
    buffer_account: &'info Account<'info, BufferAccount>,
    market_manager: &'info AccountLoader<'info, MarketManager>,
) -> Result<WorkUnit<'info>> {
    let mut unit = WorkUnit::new();
    
    // Load core market accounts
    unit.load_market_field(market_field)?;
    unit.load_buffer(buffer_account)?;
    unit.load_market_manager(market_manager)?;
    
    Ok(unit)
}

/// Create and initialize a WorkUnit with position accounts
pub fn create_position_work_unit<'info>(
    position: &'info Account<'info, TickPositionMetadata>,
    market_field: &'info Account<'info, MarketField>,
    buffer_account: &'info Account<'info, BufferAccount>,
) -> Result<WorkUnit<'info>> {
    let mut unit = WorkUnit::new();
    
    // Load position and related accounts
    unit.load_position(position)?;
    unit.load_market_field(market_field)?;
    unit.load_buffer(buffer_account)?;
    
    Ok(unit)
}

// ============================================================================
// Additional WorkUnit Methods
// ============================================================================

impl<'info> WorkUnit<'info> {
    /// Get reference to market field for external use
    pub fn get_market_field_ref(&self) -> Result<&MarketField> {
        // Find the first loaded MarketField
        for (_, change) in &self.pending_changes {
            if let StateChange::MarketField(field) = change {
                return Ok(field.as_ref());
            }
        }
        Err(FeelsProtocolError::InvalidAccountData.into())
    }
    
    /// Get reference to market manager for external use
    pub fn get_market_manager_ref(&self) -> Result<&MarketManager> {
        // Find the first loaded MarketManager
        for (_, change) in &self.pending_changes {
            if let StateChange::MarketManager(mgr) = change {
                return Ok(mgr.as_ref());
            }
        }
        Err(FeelsProtocolError::InvalidAccountData.into())
    }
    
    /// Get reference to buffer account for external use
    pub fn get_buffer_ref(&self) -> Result<&BufferAccount> {
        // Find the first loaded BufferAccount
        for (_, change) in &self.pending_changes {
            if let StateChange::Buffer(buffer) = change {
                return Ok(buffer.as_ref());
            }
        }
        Err(FeelsProtocolError::InvalidAccountData.into())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_work_unit_creation() {
        let unit = WorkUnit::new();
        assert_eq!(unit.tracked_count(), 0);
        assert!(!unit.has_changes());
    }
    
    #[test]
    fn test_state_tracking() {
        // Test would require mock accounts
        // Placeholder for actual tests
    }
}