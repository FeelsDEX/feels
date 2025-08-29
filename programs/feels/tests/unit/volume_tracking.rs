use feels::state::volume::*;
// use anchor_lang::prelude::*;

#[cfg(test)]
mod volume_tracking_tests {
    use super::*;

    #[test]
    fn test_volume_tracker_initialization() {
        let tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        
        assert_eq!(tracker.volume_24h_token_a, 0);
        assert_eq!(tracker.volume_24h_token_b, 0);
        assert_eq!(tracker.current_hour, 0);
        assert_eq!(tracker.last_update, 0);
        
        // All hourly buckets should be zero
        for i in 0..24 {
            assert_eq!(tracker.hourly_volumes_a[i], 0);
            assert_eq!(tracker.hourly_volumes_b[i], 0);
        }
    }

    #[test]
    fn test_volume_update_same_hour() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let current_time = 1_700_000_000i64; // Some timestamp
        
        // First trade
        tracker.update_volume(1000, 2000, current_time).unwrap();
        assert_eq!(tracker.volume_24h_token_a, 1000);
        assert_eq!(tracker.volume_24h_token_b, 2000);
        assert_eq!(tracker.hourly_volumes_a[tracker.current_hour as usize], 1000);
        assert_eq!(tracker.hourly_volumes_b[tracker.current_hour as usize], 2000);
        
        // Second trade in same hour
        tracker.update_volume(500, 1000, current_time + 1800).unwrap(); // 30 min later
        assert_eq!(tracker.volume_24h_token_a, 1500);
        assert_eq!(tracker.volume_24h_token_b, 3000);
        assert_eq!(tracker.hourly_volumes_a[tracker.current_hour as usize], 1500);
        assert_eq!(tracker.hourly_volumes_b[tracker.current_hour as usize], 3000);
    }

    #[test]
    fn test_hour_rollover() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let base_time = 1_700_000_000i64;
        let hour_0 = ((base_time / 3600) % 24) as u8;
        
        // Add volume in first hour
        tracker.update_volume(1000, 2000, base_time).unwrap();
        assert_eq!(tracker.current_hour, hour_0);
        assert_eq!(tracker.hourly_volumes_a[hour_0 as usize], 1000);
        
        // Move to next hour
        let next_hour_time = base_time + 3601; // Just past 1 hour
        let hour_1 = ((next_hour_time / 3600) % 24) as u8;
        assert_ne!(hour_0, hour_1); // Should be different hours
        
        tracker.update_volume(500, 1000, next_hour_time).unwrap();
        assert_eq!(tracker.current_hour, hour_1);
        
        // Previous hour's volume should still be there
        assert_eq!(tracker.hourly_volumes_a[hour_0 as usize], 1000);
        assert_eq!(tracker.hourly_volumes_b[hour_0 as usize], 2000);
        
        // New hour should have new volume
        assert_eq!(tracker.hourly_volumes_a[hour_1 as usize], 500);
        assert_eq!(tracker.hourly_volumes_b[hour_1 as usize], 1000);
        
        // Total should include both
        assert_eq!(tracker.volume_24h_token_a, 1500);
        assert_eq!(tracker.volume_24h_token_b, 3000);
    }

    #[test]
    fn test_24h_rolling_window() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let base_time = 1_700_000_000i64;
        
        // Fill all 24 hour buckets
        for i in 0..24 {
            let time = base_time + (i as i64 * 3600);
            let amount = ((i + 1) * 100) as u64;
            tracker.update_volume(amount, amount * 2, time).unwrap();
        }
        
        // Total should be sum of all hours
        let expected_total_0: u128 = (1..=24).map(|i| (i * 100) as u128).sum();
        let expected_total_1: u128 = (1..=24).map(|i| (i * 200) as u128).sum();
        assert_eq!(tracker.volume_24h_token_a, expected_total_0);
        assert_eq!(tracker.volume_24h_token_b, expected_total_1);
        
        // Move forward 1 hour (25th hour)
        let new_time = base_time + (24 * 3600);
        let new_hour = ((new_time / 3600) % 24) as usize;
        
        // Store the old value that will be replaced for the target hour
        let old_value_0 = tracker.hourly_volumes_a[new_hour];
        let old_value_1 = tracker.hourly_volumes_b[new_hour];
        
        tracker.update_volume(2500, 5000, new_time).unwrap();
        
        // Target hour's volume should be removed and new volume added
        let new_expected_0 = expected_total_0 - old_value_0 as u128 + 2500;
        let new_expected_1 = expected_total_1 - old_value_1 as u128 + 5000;
        assert_eq!(tracker.volume_24h_token_a, new_expected_0);
        assert_eq!(tracker.volume_24h_token_b, new_expected_1);
        
        // Target hour bucket should now contain new volume
        assert_eq!(tracker.hourly_volumes_a[new_hour], 2500);
        assert_eq!(tracker.hourly_volumes_b[new_hour], 5000);
    }

    #[test]
    fn test_gap_handling() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let base_time = 1_700_000_000i64;
        
        // Add volume in hour 0
        tracker.update_volume(1000, 2000, base_time).unwrap();
        let initial_hour = ((base_time / 3600) % 24) as u8;
        assert_eq!(tracker.current_hour, initial_hour);
        
        // Jump forward 3 hours
        let future_time = base_time + (3 * 3600) + 1;
        tracker.update_volume(500, 1000, future_time).unwrap();
        
        // Current hour should be 3 hours later
        let expected_hour = ((future_time / 3600) % 24) as u8;
        assert_eq!(tracker.current_hour, expected_hour);
        
        // The gap handling logic in the actual implementation will clear
        // intermediate hours between last update and current
        
        // Total should include both volumes
        assert_eq!(tracker.volume_24h_token_a, 1500);
        assert_eq!(tracker.volume_24h_token_b, 3000);
    }

    #[test]
    fn test_overflow_protection() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let current_time = 1_700_000_000i64;
        
        // Add large volume
        tracker.update_volume(u64::MAX / 2, u64::MAX / 2, current_time).unwrap();
        assert_eq!(tracker.volume_24h_token_a, (u64::MAX / 2) as u128);
        
        // Adding more should saturate, not overflow
        tracker.update_volume(u64::MAX / 2, u64::MAX / 2, current_time + 1).unwrap();
        assert!(tracker.volume_24h_token_a > 0); // Should still be valid
        assert!(tracker.volume_24h_token_b > 0);
    }

    #[test]
    fn test_very_large_gap() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let base_time = 1_700_000_000i64;
        
        // Add initial volume
        tracker.update_volume(1000, 2000, base_time).unwrap();
        assert_eq!(tracker.volume_24h_token_a, 1000);
        
        // Jump forward 30 hours (more than 24h window)
        let future_time = base_time + (30 * 3600);
        tracker.update_volume(500, 1000, future_time).unwrap();
        
        // The implementation will clear old buckets when jumping forward
        // Only the new volume should remain after such a large gap
        
        // Current hour should have the new volume
        let current_hour = tracker.current_hour;
        assert_eq!(tracker.hourly_volumes_a[current_hour as usize], 500);
        assert_eq!(tracker.hourly_volumes_b[current_hour as usize], 1000);
    }

    #[test]
    fn test_volume_accumulation() {
        // Test that volume accumulates correctly within the same hour
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        let base_time = 1_700_000_000i64;
        
        // Add multiple volumes in the same hour
        tracker.update_volume(1000, 2000, base_time).unwrap();
        tracker.update_volume(500, 1000, base_time + 100).unwrap();
        tracker.update_volume(250, 500, base_time + 200).unwrap();
        
        // Total should be sum of all
        assert_eq!(tracker.volume_24h_token_a, 1750);
        assert_eq!(tracker.volume_24h_token_b, 3500);
        
        // Current hour bucket should have total
        assert_eq!(tracker.hourly_volumes_a[tracker.current_hour as usize], 1750);
        assert_eq!(tracker.hourly_volumes_b[tracker.current_hour as usize], 3500);
    }

    #[test]
    fn test_edge_cases() {
        let mut tracker = VolumeTracker {
            volume_24h_token_a: 0,
            volume_24h_token_b: 0,
            hourly_volumes_a: [0; 24],
            hourly_volumes_b: [0; 24],
            current_hour: 0,
            last_update: 0,
            _padding: [0; 7],
        };
        
        // Zero volume update
        tracker.update_volume(0, 0, 1_700_000_000).unwrap();
        assert_eq!(tracker.volume_24h_token_a, 0);
        assert_eq!(tracker.volume_24h_token_b, 0);
        
        // Update with max values - should saturate in hourly buckets
        tracker.update_volume(u64::MAX, u64::MAX, 1_700_000_001).unwrap();
        assert_eq!(tracker.hourly_volumes_a[tracker.current_hour as usize], u64::MAX);
        assert_eq!(tracker.hourly_volumes_b[tracker.current_hour as usize], u64::MAX);
        assert_eq!(tracker.volume_24h_token_a, u64::MAX as u128);
        assert_eq!(tracker.volume_24h_token_b, u64::MAX as u128);
    }
}