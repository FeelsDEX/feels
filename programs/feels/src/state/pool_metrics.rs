/// Pool metrics tracked separately from hot path data
/// This account stores cold data that is updated less frequently
use anchor_lang::prelude::*;
use crate::state::leverage::LeverageStatistics;
use crate::state::metrics_volume::VolumeTracker;

/// Separate account for pool metrics (cold data)
#[account]
pub struct PoolMetrics {
    /// Reference back to the pool
    pub pool: Pubkey,
    
    /// Last update slot for staleness checks
    pub last_update_slot: u64,
    
    /// Cumulative volume tracking
    pub total_volume_a: u128,
    pub total_volume_b: u128,
    
    /// Volume tracker for fee tiers
    pub volume_tracker: VolumeTracker,
    
    /// Leverage statistics
    pub leverage_stats: LeverageStatistics,
    
    /// Reserved space for future metrics
    pub _reserved: [u8; 128],
}

impl PoolMetrics {
    pub const SIZE: usize = 8 + // discriminator
        32 +                    // pool pubkey
        8 +                     // last_update_slot
        16 + 16 +              // total volumes
        64 +                    // volume tracker size
        32 +                    // leverage stats size
        128;                    // reserved

    /// Seeds for PDA derivation
    pub fn seeds(pool: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"pool_metrics".to_vec(),
            pool.to_bytes().to_vec(),
        ]
    }

    /// Initialize new pool metrics
    pub fn initialize(&mut self, pool: Pubkey) -> Result<()> {
        self.pool = pool;
        self.last_update_slot = Clock::get()?.slot;
        self.total_volume_a = 0;
        self.total_volume_b = 0;
        self.volume_tracker = VolumeTracker::default();
        self.leverage_stats = LeverageStatistics::default();
        Ok(())
    }

    /// Update volume metrics
    pub fn update_volume(&mut self, amount_a: u128, amount_b: u128) -> Result<()> {
        self.total_volume_a = self.total_volume_a.saturating_add(amount_a);
        self.total_volume_b = self.total_volume_b.saturating_add(amount_b);
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Update leverage statistics
    pub fn update_leverage_stats(&mut self, stats: LeverageStatistics) -> Result<()> {
        self.leverage_stats = stats;
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }
}