/// Volume tracking system for monitoring 24-hour rolling trading volumes
/// used in dynamic fee calculations and market analysis.

use anchor_lang::prelude::*;

// ============================================================================
// Volume Tracker
// ============================================================================

/// Volume tracker for dynamic fee calculations
#[derive(Clone, Copy, Debug, Default, AnchorSerialize, AnchorDeserialize)]
pub struct VolumeTracker {
    /// 24-hour rolling volume for token a
    pub volume_24h_token_a: u128,
    /// 24-hour rolling volume for token b
    pub volume_24h_token_b: u128,
    /// Hourly buckets for rolling calculation (24 buckets)
    pub hourly_volumes_a: [u64; 24],
    pub hourly_volumes_b: [u64; 24],
    /// Current hour index
    pub current_hour: u8,
    /// Last update timestamp
    pub last_update: i64,
    /// Padding
    pub _padding: [u8; 7],
}

impl VolumeTracker {
    /// Update volume with a new trade
    pub fn update_volume(
        &mut self,
        amount_a: u64,
        amount_b: u64,
        current_timestamp: i64,
    ) -> Result<()> {
        let current_hour = ((current_timestamp / 3600) % 24) as u8;

        // If we've moved to a new hour, reset that bucket
        if current_hour != self.current_hour {
            // Clear hours between last update and now
            let mut hour = (self.current_hour + 1) % 24;
            while hour != current_hour {
                self.hourly_volumes_a[hour as usize] = 0;
                self.hourly_volumes_b[hour as usize] = 0;
                self.volume_24h_token_a = self
                    .volume_24h_token_a
                    .saturating_sub(self.hourly_volumes_a[hour as usize] as u128);
                self.volume_24h_token_b = self
                    .volume_24h_token_b
                    .saturating_sub(self.hourly_volumes_b[hour as usize] as u128);
                hour = (hour + 1) % 24;
            }

            // Reset current hour bucket
            self.volume_24h_token_a = self
                .volume_24h_token_a
                .saturating_sub(self.hourly_volumes_a[current_hour as usize] as u128);
            self.volume_24h_token_b = self
                .volume_24h_token_b
                .saturating_sub(self.hourly_volumes_b[current_hour as usize] as u128);
            self.hourly_volumes_a[current_hour as usize] = 0;
            self.hourly_volumes_b[current_hour as usize] = 0;

            self.current_hour = current_hour;
        }

        // Add new volume
        self.hourly_volumes_a[current_hour as usize] =
            self.hourly_volumes_a[current_hour as usize].saturating_add(amount_a);
        self.hourly_volumes_b[current_hour as usize] =
            self.hourly_volumes_b[current_hour as usize].saturating_add(amount_b);

        self.volume_24h_token_a = self.volume_24h_token_a.saturating_add(amount_a as u128);
        self.volume_24h_token_b = self.volume_24h_token_b.saturating_add(amount_b as u128);

        self.last_update = current_timestamp;
        Ok(())
    }
}
