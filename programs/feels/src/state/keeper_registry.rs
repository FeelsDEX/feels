use anchor_lang::prelude::*;

/// Registry of authorized keepers that can submit TWAP updates
#[account(zero_copy)]
#[repr(C)]
pub struct KeeperRegistry {
    /// Authority that can add/remove keepers
    pub authority: Pubkey,
    
    /// Number of authorized keepers
    pub keeper_count: u32,
    
    /// Maximum number of keepers
    pub max_keepers: u32,
    
    /// Authorized keeper pubkeys
    pub keepers: [Pubkey; 32],
    
    /// Keeper status (true = active, false = inactive)
    pub keeper_active: [bool; 32],
    
    /// When each keeper was added (unix timestamp)
    pub keeper_added_at: [i64; 32],
    
    /// Reserved for future use
    pub _reserved: [u8; 256],
}

impl KeeperRegistry {
    pub const SIZE: usize = 32 + 4 + 4 + (32 * 32) + 32 + (8 * 32) + 256;
    
    /// Check if a keeper is authorized
    pub fn is_keeper_authorized(&self, keeper: &Pubkey) -> bool {
        for i in 0..self.keeper_count as usize {
            if self.keepers[i] == *keeper && self.keeper_active[i] {
                return true;
            }
        }
        false
    }
    
    /// Add a new keeper
    pub fn add_keeper(&mut self, keeper: Pubkey, timestamp: i64) -> Result<()> {
        require!(
            self.keeper_count < self.max_keepers,
            FeelsProtocolError::KeeperRegistryFull
        );
        
        // Check if keeper already exists
        for i in 0..self.keeper_count as usize {
            require!(
                self.keepers[i] != keeper,
                FeelsProtocolError::KeeperAlreadyExists
            );
        }
        
        let index = self.keeper_count as usize;
        self.keepers[index] = keeper;
        self.keeper_active[index] = true;
        self.keeper_added_at[index] = timestamp;
        self.keeper_count += 1;
        
        Ok(())
    }
    
    /// Remove a keeper
    pub fn remove_keeper(&mut self, keeper: &Pubkey) -> Result<()> {
        for i in 0..self.keeper_count as usize {
            if self.keepers[i] == *keeper {
                self.keeper_active[i] = false;
                return Ok(());
            }
        }
        
        Err(FeelsProtocolError::KeeperNotFound.into())
    }
}

use crate::state::FeelsProtocolError;