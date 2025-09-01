/// Pool rebase configuration tracked separately from hot path data
/// This account stores rebase-related state that is updated periodically
use anchor_lang::prelude::*;

/// Separate account for rebase configuration (cold data)
#[account]
pub struct PoolRebase {
    /// Reference back to the pool
    pub pool: Pubkey,
    
    /// Rebase accumulator account for yield/funding
    pub rebase_accumulator: Pubkey,
    
    /// Last redenomination timestamp
    pub last_redenomination: i64,
    
    /// Redenomination threshold
    pub redenomination_threshold: u64,
    
    /// Last rebase timestamp
    pub last_rebase_timestamp: i64,
    
    /// Rebase epoch duration (seconds)
    pub rebase_epoch_duration: i64,
    
    /// Reserved space for future rebase features
    pub _reserved: [u8; 128],
}

impl PoolRebase {
    pub const SIZE: usize = 8 + // discriminator
        32 +                    // pool pubkey
        32 +                    // rebase_accumulator
        8 +                     // last_redenomination
        8 +                     // redenomination_threshold
        8 +                     // last_rebase_timestamp
        8 +                     // rebase_epoch_duration
        128;                    // reserved

    /// Seeds for PDA derivation
    pub fn seeds(pool: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"pool_rebase".to_vec(),
            pool.to_bytes().to_vec(),
        ]
    }

    /// Initialize new pool rebase configuration
    pub fn initialize(&mut self, pool: Pubkey) -> Result<()> {
        self.pool = pool;
        self.rebase_accumulator = Pubkey::default();
        self.last_redenomination = 0;
        self.redenomination_threshold = 10_000; // Default 1% threshold
        self.last_rebase_timestamp = Clock::get()?.unix_timestamp;
        self.rebase_epoch_duration = 3600; // Default 1 hour
        Ok(())
    }

    /// Check if rebase accumulator is configured
    pub fn has_rebase_accumulator(&self) -> bool {
        self.rebase_accumulator != Pubkey::default()
    }

    /// Update rebase accumulator
    pub fn set_rebase_accumulator(&mut self, accumulator: Pubkey) -> Result<()> {
        self.rebase_accumulator = accumulator;
        Ok(())
    }

    /// Check if redenomination is needed
    pub fn needs_redenomination(&self, current_timestamp: i64, threshold_reached: bool) -> bool {
        let time_elapsed = current_timestamp.saturating_sub(self.last_redenomination);
        let min_interval = 86400; // 24 hours minimum between redenominations
        
        time_elapsed >= min_interval && threshold_reached
    }

    /// Update redenomination timestamp
    pub fn update_redenomination(&mut self) -> Result<()> {
        self.last_redenomination = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Check if rebase epoch has passed
    pub fn is_rebase_due(&self, current_timestamp: i64) -> bool {
        let time_elapsed = current_timestamp.saturating_sub(self.last_rebase_timestamp);
        time_elapsed >= self.rebase_epoch_duration
    }

    /// Update rebase timestamp
    pub fn update_rebase_timestamp(&mut self) -> Result<()> {
        self.last_rebase_timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
}