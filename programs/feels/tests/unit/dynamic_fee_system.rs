use feels::state::fee::*;
// use anchor_lang::prelude::*;

#[cfg(test)]
mod dynamic_fee_tests {
    use super::*;

    fn create_test_config() -> DynamicFeeConfig {
        DynamicFeeConfig {
            base_fee: 300, // 3 bps
            min_fee: 100,  // 1 bps
            max_fee: 1000, // 10 bps
            _padding: 0,
            volatility_coefficient: 2_000_000, // 2.0
            volume_discount_threshold: 100_000_000, // 100M
        }
    }

    #[test]
    fn test_base_fee() {
        let config = create_test_config();
        
        // Low volatility, low volume should return base fee
        let fee = config.calculate_fee(100, 1_000_000); // 1% volatility, 1M volume
        assert_eq!(fee, 300, "Should return base fee for normal conditions");
    }

    #[test]
    fn test_high_volatility_adjustment() {
        let config = create_test_config();
        
        // High volatility should increase fee
        let fee = config.calculate_fee(600, 1_000_000); // 6% volatility
        assert_eq!(fee, 450, "Should increase fee by 50% for high volatility");
        
        // Medium volatility
        let fee = config.calculate_fee(400, 1_000_000); // 4% volatility
        assert_eq!(fee, 360, "Should increase fee by 20% for medium volatility");
    }

    #[test]
    fn test_volume_discount() {
        let config = create_test_config();
        
        // High volume should give discount
        let fee = config.calculate_fee(100, 200_000_000); // 200M volume
        assert_eq!(fee, 270, "Should discount by 10% for high volume");
    }

    #[test]
    fn test_combined_adjustments() {
        let config = create_test_config();
        
        // High volatility + high volume
        let fee = config.calculate_fee(600, 200_000_000);
        // Base 300 * 1.5 (volatility) * 0.9 (volume) = 405
        assert_eq!(fee, 405, "Should apply both adjustments");
    }

    #[test]
    fn test_fee_clamping() {
        let config = DynamicFeeConfig {
            base_fee: 500,
            min_fee: 200,
            max_fee: 600,
            _padding: 0,
            volatility_coefficient: 2_000_000,
            volume_discount_threshold: 100_000_000,
        };
        
        // Test max clamping with high volatility
        let fee = config.calculate_fee(1000, 0); // 10% volatility
        assert_eq!(fee, 600, "Should clamp to max fee");
        
        // Test min clamping with volume discount
        let config2 = DynamicFeeConfig {
            base_fee: 150,
            min_fee: 200,
            max_fee: 1000,
            _padding: 0,
            volatility_coefficient: 2_000_000,
            volume_discount_threshold: 100_000_000,
        };
        
        let fee = config2.calculate_fee(100, 200_000_000);
        assert_eq!(fee, 200, "Should clamp to min fee");
    }

    #[test]
    fn test_edge_cases() {
        let config = create_test_config();
        
        // Zero volatility
        let fee = config.calculate_fee(0, 0);
        assert_eq!(fee, 300, "Should handle zero volatility");
        
        // Extreme volatility
        let fee = config.calculate_fee(10000, 0);
        assert_eq!(fee, 450, "Should handle extreme volatility");
        
        // Exactly at volume threshold
        let fee = config.calculate_fee(100, 100_000_000);
        assert_eq!(fee, 300, "Should not discount at exact threshold");
    }
}