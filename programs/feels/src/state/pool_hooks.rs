/// Pool hook configuration tracked separately from hot path data
/// This account stores hook-related configuration that is rarely accessed
use anchor_lang::prelude::*;

/// Separate account for hook configuration (cold data)
#[account]
pub struct PoolHooks {
    /// Reference back to the pool
    pub pool: Pubkey,
    
    /// Hook registry account (Pubkey::default() if none)
    pub hook_registry: Pubkey,
    
    /// Valence hook session (Pubkey::default() if none)
    pub valence_session: Pubkey,
    
    /// Hook enable flags
    pub hooks_enabled: bool,
    
    /// Last hook update timestamp
    pub last_hook_update: i64,
    
    /// Reserved space for future hook features
    pub _reserved: [u8; 128],
}

impl PoolHooks {
    pub const SIZE: usize = 8 + // discriminator
        32 +                    // pool pubkey
        32 +                    // hook_registry
        32 +                    // valence_session
        1 +                     // hooks_enabled
        8 +                     // last_hook_update
        128;                    // reserved

    /// Seeds for PDA derivation
    pub fn seeds(pool: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"pool_hooks".to_vec(),
            pool.to_bytes().to_vec(),
        ]
    }

    /// Initialize new pool hooks configuration
    pub fn initialize(&mut self, pool: Pubkey) -> Result<()> {
        self.pool = pool;
        self.hook_registry = Pubkey::default();
        self.valence_session = Pubkey::default();
        self.hooks_enabled = false;
        self.last_hook_update = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Check if hook registry is configured
    pub fn has_hook_registry(&self) -> bool {
        self.hook_registry != Pubkey::default()
    }

    /// Check if valence session is active
    pub fn has_valence_session(&self) -> bool {
        self.valence_session != Pubkey::default()
    }

    /// Update hook registry
    pub fn set_hook_registry(&mut self, registry: Pubkey) -> Result<()> {
        self.hook_registry = registry;
        self.last_hook_update = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Update valence session
    pub fn set_valence_session(&mut self, session: Pubkey) -> Result<()> {
        self.valence_session = session;
        self.last_hook_update = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Enable or disable hooks
    pub fn set_hooks_enabled(&mut self, enabled: bool) -> Result<()> {
        self.hooks_enabled = enabled;
        self.last_hook_update = Clock::get()?.unix_timestamp;
        Ok(())
    }
}