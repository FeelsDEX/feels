/// Consolidated pool metrics account that combines essential metrics from multiple sources.
/// This reduces the number of accounts required per pool while keeping hot-path data separate.
use anchor_lang::prelude::*;
use crate::state::leverage::LeverageStatistics;
use crate::state::metrics_volume::VolumeTracker;

/// Consolidated metrics account combining volume, lending, and volatility data
#[account]
pub struct PoolMetricsConsolidated {
    /// Reference back to the pool
    pub pool: Pubkey,
    
    /// Last update slot for staleness checks
    pub last_update_slot: u64,
    
    // ========== Volume Metrics ==========
    
    /// Cumulative volume tracking
    pub total_volume_a: u128,
    pub total_volume_b: u128,
    
    /// 24-hour rolling volume tracker
    pub volume_tracker: VolumeTracker,
    
    // ========== Leverage Metrics ==========
    
    /// Leverage statistics
    pub leverage_stats: LeverageStatistics,
    
    // ========== Lending Metrics (Core Fields) ==========
    
    /// Total supplied across all durations
    pub total_supplied: u128,
    
    /// Total borrowed across all durations
    pub total_borrowed: u128,
    
    /// Current utilization rate (basis points)
    pub utilization_rate: u16,
    
    /// Combined stress score (0-10000 basis points)
    pub lending_stress_score: u16,
    
    /// Total flash loan volume
    pub flash_volume_total: u128,
    
    /// Total flash loan count
    pub flash_count_total: u64,
    
    /// Flash loan burst detection count
    pub flash_burst_count: u16,
    
    /// Last flash loan timestamp (for burst detection)
    pub last_flash_timestamp: i64,
    
    // ========== Volatility Summary ==========
    
    /// Composite volatility score (basis points)
    pub volatility_composite: u16,
    
    /// Volatility percentile rank (0-100)
    pub volatility_percentile: u8,
    
    /// Last volatility update timestamp
    pub last_volatility_update: i64,
    
    /// Recent volatility spike flag
    pub volatility_spike_detected: bool,
    
    // ========== Fee Metrics ==========
    
    /// Cumulative fees collected in token A
    pub fees_collected_a: u128,
    
    /// Cumulative fees collected in token B
    pub fees_collected_b: u128,
    
    /// Average fee rate over last 24h (basis points)
    pub avg_fee_rate_24h: u16,
    
    // ========== Reserved Space ==========
    
    /// Reserved space for future metrics
    pub _reserved: [u8; 64],
}

impl PoolMetricsConsolidated {
    pub const SIZE: usize = 8 +     // discriminator
        32 +                        // pool pubkey
        8 +                         // last_update_slot
        16 + 16 +                   // total volumes
        VolumeTracker::SIZE +       // volume tracker (416 bytes)
        LeverageStatistics::SIZE +  // leverage stats (32 bytes)
        16 +                        // total_supplied
        16 +                        // total_borrowed
        2 +                         // utilization_rate
        2 +                         // lending_stress_score
        16 +                        // flash_volume_total
        8 +                         // flash_count_total
        2 +                         // flash_burst_count
        8 +                         // last_flash_timestamp
        2 +                         // volatility_composite
        1 +                         // volatility_percentile
        8 +                         // last_volatility_update
        1 +                         // volatility_spike_detected
        16 +                        // fees_collected_a
        16 +                        // fees_collected_b
        2 +                         // avg_fee_rate_24h
        64;                         // reserved

    /// Seeds for PDA derivation
    pub fn seeds(pool: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"pool_metrics_v2".to_vec(),
            pool.to_bytes().to_vec(),
        ]
    }

    /// Initialize new consolidated pool metrics
    pub fn initialize(&mut self, pool: Pubkey) -> Result<()> {
        self.pool = pool;
        self.last_update_slot = Clock::get()?.slot;
        self.total_volume_a = 0;
        self.total_volume_b = 0;
        self.volume_tracker = VolumeTracker::default();
        self.leverage_stats = LeverageStatistics::default();
        self.total_supplied = 0;
        self.total_borrowed = 0;
        self.utilization_rate = 0;
        self.lending_stress_score = 0;
        self.flash_volume_total = 0;
        self.flash_count_total = 0;
        self.flash_burst_count = 0;
        self.last_flash_timestamp = 0;
        self.volatility_composite = 0;
        self.volatility_percentile = 0;
        self.last_volatility_update = 0;
        self.volatility_spike_detected = false;
        self.fees_collected_a = 0;
        self.fees_collected_b = 0;
        self.avg_fee_rate_24h = 0;
        Ok(())
    }

    /// Update volume metrics
    pub fn update_volume(&mut self, amount_a: u128, amount_b: u128) -> Result<()> {
        self.total_volume_a = self.total_volume_a.saturating_add(amount_a);
        self.total_volume_b = self.total_volume_b.saturating_add(amount_b);
        
        // Update rolling volume tracker
        let current_time = Clock::get()?.unix_timestamp;
        self.volume_tracker.add_volume(amount_a, amount_b, current_time)?;
        
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Update lending metrics
    pub fn update_lending_metrics(
        &mut self,
        supplied: u128,
        borrowed: u128,
        flash_volume: u128,
        is_flash: bool,
    ) -> Result<()> {
        self.total_supplied = supplied;
        self.total_borrowed = borrowed;
        
        // Calculate utilization
        if supplied > 0 {
            self.utilization_rate = ((borrowed.saturating_mul(10_000)) / supplied) as u16;
        } else {
            self.utilization_rate = 0;
        }
        
        // Update flash loan metrics
        if is_flash {
            self.flash_volume_total = self.flash_volume_total.saturating_add(flash_volume);
            self.flash_count_total = self.flash_count_total.saturating_add(1);
            
            // Burst detection
            let current_time = Clock::get()?.unix_timestamp;
            if current_time - self.last_flash_timestamp < 60 { // Within 1 minute
                self.flash_burst_count = self.flash_burst_count.saturating_add(1);
            } else {
                self.flash_burst_count = 1;
            }
            self.last_flash_timestamp = current_time;
        }
        
        // Update stress score based on utilization and flash activity
        self.calculate_stress_score()?;
        
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Update volatility summary (called by external volatility tracker)
    pub fn update_volatility_summary(
        &mut self,
        composite_vol: u16,
        percentile: u8,
        spike_detected: bool,
    ) -> Result<()> {
        self.volatility_composite = composite_vol;
        self.volatility_percentile = percentile;
        self.volatility_spike_detected = spike_detected;
        self.last_volatility_update = Clock::get()?.unix_timestamp;
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Update fee collection metrics
    pub fn update_fees(&mut self, fee_a: u128, fee_b: u128) -> Result<()> {
        self.fees_collected_a = self.fees_collected_a.saturating_add(fee_a);
        self.fees_collected_b = self.fees_collected_b.saturating_add(fee_b);
        
        // Update average fee rate based on volume
        let (volume_24h_a, volume_24h_b) = self.volume_tracker.get_24h_volume();
        let total_volume = volume_24h_a.saturating_add(volume_24h_b);
        let total_fees = fee_a.saturating_add(fee_b);
        
        if total_volume > 0 {
            self.avg_fee_rate_24h = ((total_fees.saturating_mul(10_000)) / total_volume) as u16;
        }
        
        self.last_update_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Calculate combined stress score
    fn calculate_stress_score(&mut self) -> Result<()> {
        let mut score = 0u16;
        
        // Utilization component (0-5000 bps)
        score = score.saturating_add(self.utilization_rate / 2);
        
        // Flash loan burst component (0-2500 bps)
        if self.flash_burst_count > 10 {
            score = score.saturating_add(2500);
        } else {
            score = score.saturating_add((self.flash_burst_count as u16) * 250);
        }
        
        // Volatility component (0-2500 bps)
        let vol_component = (self.volatility_composite.min(2500) * self.volatility_percentile as u16) / 100;
        score = score.saturating_add(vol_component);
        
        self.lending_stress_score = score.min(10_000);
        Ok(())
    }

    /// Get risk-adjusted fee multiplier based on metrics
    pub fn get_fee_multiplier(&self) -> u16 {
        // Base multiplier is 10000 (1x)
        let mut multiplier = 10_000u16;
        
        // Add stress-based component (up to +50%)
        let stress_addition = (self.lending_stress_score / 2).min(5_000);
        multiplier = multiplier.saturating_add(stress_addition);
        
        // Add volatility-based component (up to +30%)
        if self.volatility_spike_detected {
            multiplier = multiplier.saturating_add(3_000);
        } else {
            let vol_addition = (self.volatility_composite / 3).min(3_000);
            multiplier = multiplier.saturating_add(vol_addition);
        }
        
        multiplier.min(20_000) // Cap at 2x
    }
}

// ============================================================================
// Migration Helper
// ============================================================================

/// Helper to migrate from old separate accounts to consolidated
pub fn migrate_to_consolidated(
    old_metrics: &crate::state::PoolMetrics,
    lending_metrics: Option<&crate::state::metrics_lending::LendingMetrics>,
    volatility_summary: Option<(u16, u8, bool)>,
) -> PoolMetricsConsolidated {
    let mut consolidated = PoolMetricsConsolidated::default();
    
    // Copy existing data
    consolidated.pool = old_metrics.pool;
    consolidated.last_update_slot = old_metrics.last_update_slot;
    consolidated.total_volume_a = old_metrics.total_volume_a;
    consolidated.total_volume_b = old_metrics.total_volume_b;
    consolidated.volume_tracker = old_metrics.volume_tracker.clone();
    consolidated.leverage_stats = old_metrics.leverage_stats.clone();
    
    // Add lending metrics if available
    if let Some(lending) = lending_metrics {
        consolidated.total_supplied = lending.total_supplied;
        consolidated.total_borrowed = lending.total_borrowed;
        consolidated.utilization_rate = lending.utilization_rate;
        consolidated.lending_stress_score = lending.combined_stress_score;
        consolidated.flash_volume_total = lending.flash_loan_tracker.total_volume;
        consolidated.flash_count_total = lending.flash_loan_tracker.total_count;
    }
    
    // Add volatility summary if available
    if let Some((composite, percentile, spike)) = volatility_summary {
        consolidated.volatility_composite = composite;
        consolidated.volatility_percentile = percentile;
        consolidated.volatility_spike_detected = spike;
    }
    
    consolidated
}