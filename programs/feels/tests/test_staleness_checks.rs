/// Tests for comprehensive staleness checking
use anchor_lang::prelude::*;
use feels::state::*;
use feels::error::FeelsProtocolError;
use feels::logic::field_verification::verify_market_update_enhanced;

#[cfg(test)]
mod staleness_tests {
    use super::*;

    /// Test update frequency violation
    #[test]
    fn test_update_too_frequent() {
        let current_time = 1000;
        
        // Create a data source that was just updated
        let mut data_source = MarketDataSource::default();
        data_source.update_frequency = 60; // 1 minute between updates
        data_source.last_update = current_time - 30; // Only 30 seconds ago
        data_source.is_active = true;
        
        // Try to update too soon
        let result = data_source.check_staleness(current_time);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FeelsProtocolError::UpdateTooFrequent.into()
        );
    }

    /// Test commitment expiration
    #[test]
    fn test_commitment_expired() {
        let current_time = 1000;
        
        // Create an update with expired commitment
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(FieldCommitmentData {
                S: 100,
                T: 100,
                L: 100,
                w_s: 2500,
                w_t: 2500,
                w_l: 2500,
                w_tau: 2500,
                omega_0: 5000,
                omega_1: 5000,
                twap_0: 100,
                twap_1: 100,
                max_staleness: current_time - 100, // Expired 100 seconds ago
            }),
            price_data: None,
            timestamp: current_time - 10,
            sequence: 1,
        };
        
        let mut data_source = MarketDataSource::default();
        data_source.is_active = true;
        data_source.last_sequence = 0;
        data_source.last_update = 0;
        data_source.update_frequency = 0;
        data_source.config.source_type = DATA_SOURCE_TYPE_KEEPER;
        data_source.config.keeper_config.max_staleness = 300;
        
        let market_field = MarketField {
            snapshot_ts: current_time - 100,
            max_staleness: 300,
            ..Default::default()
        };
        
        // Should fail with commitment expired error
        let result = verify_market_update_enhanced(&update, &data_source, &market_field, current_time);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FeelsProtocolError::CommitmentExpired.into()
        );
    }

    /// Test source-level staleness
    #[test]
    fn test_keeper_data_stale() {
        let current_time = 1000;
        
        // Create an update that's too old according to keeper config
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(FieldCommitmentData {
                S: 100,
                T: 100,
                L: 100,
                w_s: 2500,
                w_t: 2500,
                w_l: 2500,
                w_tau: 2500,
                omega_0: 5000,
                omega_1: 5000,
                twap_0: 100,
                twap_1: 100,
                max_staleness: current_time + 3600, // Valid for 1 hour
            }),
            price_data: None,
            timestamp: current_time - 400, // 400 seconds old
            sequence: 1,
        };
        
        let mut data_source = MarketDataSource::default();
        data_source.is_active = true;
        data_source.last_sequence = 0;
        data_source.last_update = 0;
        data_source.update_frequency = 0;
        data_source.config.source_type = DATA_SOURCE_TYPE_KEEPER;
        data_source.config.keeper_config.max_staleness = 300; // Max 5 minutes
        
        let market_field = MarketField {
            snapshot_ts: current_time - 100,
            max_staleness: 600,
            ..Default::default()
        };
        
        // Should fail with data stale error
        let result = verify_market_update_enhanced(&update, &data_source, &market_field, current_time);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FeelsProtocolError::DataStale.into()
        );
    }

    /// Test field-level staleness
    #[test]
    fn test_field_data_stale() {
        let current_time = 1000;
        
        // Create a valid update
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(FieldCommitmentData {
                S: 100,
                T: 100,
                L: 100,
                w_s: 2500,
                w_t: 2500,
                w_l: 2500,
                w_tau: 2500,
                omega_0: 5000,
                omega_1: 5000,
                twap_0: 100,
                twap_1: 100,
                max_staleness: current_time + 3600, // Valid for 1 hour
            }),
            price_data: None,
            timestamp: current_time - 10, // Fresh update
            sequence: 1,
        };
        
        let mut data_source = MarketDataSource::default();
        data_source.is_active = true;
        data_source.last_sequence = 0;
        data_source.last_update = 0;
        data_source.update_frequency = 0;
        data_source.config.source_type = DATA_SOURCE_TYPE_KEEPER;
        data_source.config.keeper_config.max_staleness = 300;
        
        // Market field with stale data
        let market_field = MarketField {
            snapshot_ts: current_time - 400, // 400 seconds old
            max_staleness: 300, // Max 5 minutes
            ..Default::default()
        };
        
        // Should fail with field data stale error
        let result = verify_market_update_enhanced(&update, &data_source, &market_field, current_time);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FeelsProtocolError::DataStale.into()
        );
    }

    /// Test successful update with all checks passing
    #[test]
    fn test_all_staleness_checks_pass() {
        let current_time = 1000;
        
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(FieldCommitmentData {
                S: 100,
                T: 100,
                L: 100,
                w_s: 2500,
                w_t: 2500,
                w_l: 2500,
                w_tau: 2500,
                omega_0: 5000,
                omega_1: 5000,
                twap_0: 100,
                twap_1: 100,
                max_staleness: current_time + 3600, // Valid for 1 hour
            }),
            price_data: None,
            timestamp: current_time - 10, // Fresh update
            sequence: 1,
        };
        
        let mut data_source = MarketDataSource::default();
        data_source.is_active = true;
        data_source.last_sequence = 0;
        data_source.last_update = current_time - 120; // Last update 2 minutes ago
        data_source.update_frequency = 60; // 1 minute minimum
        data_source.config.source_type = DATA_SOURCE_TYPE_KEEPER;
        data_source.config.keeper_config.max_staleness = 300;
        
        let market_field = MarketField {
            snapshot_ts: current_time - 100, // Fresh field data
            max_staleness: 300,
            ..Default::default()
        };
        
        // Should pass all checks
        let result = verify_market_update_enhanced(&update, &data_source, &market_field, current_time);
        assert!(result.is_ok());
    }
}