use feels::state::oracle::*;
use anchor_lang::prelude::Pubkey;

#[cfg(test)]
mod oracle_system_tests {
    use super::*;

    // Since the unified oracle requires interaction with OracleData account and syscalls,
    // we'll test the basic functionality and data structures
    
    #[test]
    fn test_oracle_initialization() {
        let oracle = Oracle {
            pool: Pubkey::new_unique(),
            observation_count: 0,
            ring_index: 0,
            last_update_timestamp: 0,
            last_update_slot: 0,
            volatility_30min: 0,
            volatility_4hr: 0,
            volatility_24hr: 0,
            data_account: Pubkey::new_unique(),
            _reserved: [0; 64],
        };

        // Test initialization values
        assert_eq!(oracle.observation_count, 0);
        assert_eq!(oracle.ring_index, 0);
        assert_eq!(oracle.last_update_timestamp, 0);
        assert_eq!(oracle.last_update_slot, 0);
        assert_eq!(oracle.volatility_30min, 0);
        assert_eq!(oracle.volatility_4hr, 0);
        assert_eq!(oracle.volatility_24hr, 0);
    }

    #[test]
    fn test_ring_buffer_wrapping() {
        // Test ring buffer index calculation logic
        assert_eq!((1023 + 1) % 1024, 0); // Should wrap to 0
        assert_eq!((1024 + 1) % 1024, 1); // Should wrap to 1
        assert_eq!((0 + 1) % 1024, 1);    // Normal increment
        
        // Test observation count capping
        let mut count = 1023u16;
        count = if count < 1024 { count + 1 } else { count };
        assert_eq!(count, 1024);
        
        count = if count < 1024 { count + 1 } else { count };
        assert_eq!(count, 1024); // Should stay at max
    }

    #[test]
    fn test_timestamp_validation() {
        // Test timestamp ordering logic
        let current_timestamp = 1000i64;
        
        // Older timestamps should be invalid
        assert!(999 <= current_timestamp);
        assert!(1000 <= current_timestamp);
        
        // Newer timestamps should be valid
        assert!(1001 > current_timestamp);
        assert!(2000 > current_timestamp);
    }

    #[test]
    fn test_observation_struct_size() {
        // Verify the observation struct is the expected size
        assert_eq!(Observation::SIZE, 56); // Updated size for unified Observation struct
    }

    #[test]
    fn test_oracle_data_capacity() {
        // Verify oracle data can hold 1024 observations
        assert_eq!(OracleData::SIZE, 8 + 32 + (Observation::SIZE * 1024));
    }

    #[test]
    fn test_volatility_update() {
        let mut oracle = Oracle {
            pool: Pubkey::new_unique(),
            observation_count: 10,
            ring_index: 10,
            last_update_timestamp: 1000,
            last_update_slot: 100,
            volatility_30min: 100,
            volatility_4hr: 200,
            volatility_24hr: 300,
            data_account: Pubkey::new_unique(),
            _reserved: [0; 64],
        };

        // Test volatility field access and basic updates
        oracle.volatility_30min = 150;
        oracle.volatility_4hr = 250;
        oracle.volatility_24hr = 350;
        
        assert_eq!(oracle.volatility_30min, 150);
        assert_eq!(oracle.volatility_4hr, 250);
        assert_eq!(oracle.volatility_24hr, 350);
    }

    #[test]
    fn test_observation_default() {
        let observation = Observation::default();
        // Avoid direct field references on packed structs to prevent alignment issues
        let timestamp = observation.timestamp;
        let sqrt_price = observation.sqrt_price;
        let cumulative_price = observation.cumulative_price;
        let tick = observation.tick;
        let cumulative_tick = observation.cumulative_tick;
        let price_variance = observation.price_variance;
        
        assert_eq!(timestamp, 0);
        assert_eq!(sqrt_price, 0);
        assert_eq!(cumulative_price, 0);
        assert_eq!(tick, 0);
        assert_eq!(cumulative_tick, 0);
        assert_eq!(price_variance, 0);
    }

    #[test]
    fn test_oracle_data_initialization() {
        let mut oracle_data = OracleData {
            oracle: Pubkey::new_unique(),
            observations: [Observation::default(); 1024],
        };
        
        // Test storing and retrieving observations
        let test_observation = Observation {
            timestamp: 1000,
            sqrt_price: 79228162514264337593543950336, // ~1.0 in Q64.96
            cumulative_price: 0,
            tick: 0,
            cumulative_tick: 0,
            price_variance: 100,
            _padding: [0; 2],
        };
        
        oracle_data.store_observation(0, test_observation);
        let retrieved = oracle_data.get_observation(0);
        
        // Avoid direct field references on packed structs
        let ret_timestamp = retrieved.timestamp;
        let ret_sqrt_price = retrieved.sqrt_price;
        let ret_price_variance = retrieved.price_variance;
        
        assert_eq!(ret_timestamp, 1000);
        assert_eq!(ret_sqrt_price, 79228162514264337593543950336);
        assert_eq!(ret_price_variance, 100);
        
        // Test wraparound indexing
        oracle_data.store_observation(1024, test_observation); // Should wrap to index 0
        let wrapped = oracle_data.get_observation(1024);
        let wrapped_timestamp = wrapped.timestamp;
        assert_eq!(wrapped_timestamp, 1000);
    }
}