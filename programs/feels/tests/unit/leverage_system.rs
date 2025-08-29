use feels::state::leverage::*;
// use anchor_lang::prelude::*;

#[cfg(test)]
mod leverage_system_tests {
    use super::*;

    #[test]
    fn test_risk_profile_linear_protection() {
        // Test linear protection curve
        let params = LeverageParameters {
            max_leverage: 5_000_000, // 5x
            current_ceiling: 5_000_000,
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test 1x leverage (no leverage)
        let profile = RiskProfile::from_leverage(1_000_000, &params).unwrap();
        assert_eq!(profile.leverage, 1_000_000);
        assert_eq!(profile.protection_factor, 1_000_000); // 100% protection
        assert_eq!(profile.max_loss_percentage, 0); // 0% max loss

        // Test 3x leverage
        let profile = RiskProfile::from_leverage(3_000_000, &params).unwrap();
        assert_eq!(profile.leverage, 3_000_000);
        assert_eq!(profile.protection_factor, 500_000); // 50% protection
        assert_eq!(profile.max_loss_percentage, 500_000); // 50% max loss

        // Test 5x leverage (maximum)
        let profile = RiskProfile::from_leverage(5_000_000, &params).unwrap();
        assert_eq!(profile.leverage, 5_000_000);
        assert_eq!(profile.protection_factor, 0); // 0% protection
        assert_eq!(profile.max_loss_percentage, 1_000_000); // 100% max loss
    }

    #[test]
    fn test_risk_profile_exponential_protection() {
        // Test exponential protection curve
        let params = LeverageParameters {
            max_leverage: 10_000_000, // 10x
            current_ceiling: 10_000_000,
            protection_curve: ProtectionCurve::Exponential {
                decay_rate: 500_000, // 0.5
            },
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test 1x leverage
        let profile = RiskProfile::from_leverage(1_000_000, &params).unwrap();
        assert_eq!(profile.protection_factor, 1_000_000); // 100% protection

        // Test 2x leverage
        let profile = RiskProfile::from_leverage(2_000_000, &params).unwrap();
        // protection ≈ 1 / (1 + 0.5 * 1) = 1 / 1.5 ≈ 0.666...
        assert!(profile.protection_factor > 666_000 && profile.protection_factor < 667_000);

        // Test 5x leverage
        let profile = RiskProfile::from_leverage(5_000_000, &params).unwrap();
        // protection ≈ 1 / (1 + 0.5 * 4) = 1 / 3 ≈ 0.333...
        assert!(profile.protection_factor > 333_000 && profile.protection_factor < 334_000);
    }

    #[test]
    fn test_risk_profile_piecewise_protection() {
        // Test piecewise protection curve
        let points: [[u64; 2]; 8] = [
            [2_000_000, 900_000], // 2x -> 90% protection
            [3_000_000, 750_000], // 3x -> 75% protection
            [4_000_000, 600_000], // 4x -> 60% protection
            [5_000_000, 400_000], // 5x -> 40% protection
            [6_000_000, 250_000], // 6x -> 25% protection
            [7_000_000, 100_000], // 7x -> 10% protection
            [8_000_000, 50_000],  // 8x -> 5% protection
            [10_000_000, 0],      // 10x -> 0% protection
        ];

        let params = LeverageParameters {
            max_leverage: 10_000_000,
            current_ceiling: 10_000_000,
            protection_curve: ProtectionCurve::Piecewise { points },
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test various leverage levels
        let profile = RiskProfile::from_leverage(1_500_000, &params).unwrap();
        assert_eq!(profile.protection_factor, 900_000); // First breakpoint

        let profile = RiskProfile::from_leverage(3_500_000, &params).unwrap();
        assert_eq!(profile.protection_factor, 600_000); // Between 3x and 4x

        let profile = RiskProfile::from_leverage(9_000_000, &params).unwrap();
        assert_eq!(profile.protection_factor, 0); // Last breakpoint
    }

    #[test]
    fn test_invalid_leverage() {
        let params = LeverageParameters {
            max_leverage: 5_000_000,
            current_ceiling: 3_000_000, // Lower than max due to market conditions
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test leverage below minimum (< 1x)
        assert!(RiskProfile::from_leverage(500_000, &params).is_err());

        // Test leverage above ceiling
        assert!(RiskProfile::from_leverage(4_000_000, &params).is_err());

        // Test leverage at ceiling (should succeed)
        assert!(RiskProfile::from_leverage(3_000_000, &params).is_ok());
    }

    #[test]
    fn test_fee_multiplier_calculation() {
        let params = LeverageParameters {
            max_leverage: 10_000_000,
            current_ceiling: 10_000_000,
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test fee multiplier increases with leverage
        let profile_1x = RiskProfile::from_leverage(1_000_000, &params).unwrap();
        let profile_2x = RiskProfile::from_leverage(2_000_000, &params).unwrap();
        let profile_4x = RiskProfile::from_leverage(4_000_000, &params).unwrap();

        assert!(profile_1x.fee_multiplier < profile_2x.fee_multiplier);
        assert!(profile_2x.fee_multiplier < profile_4x.fee_multiplier);

        // Fee multiplier should be approximately sqrt(leverage)
        // For 4x leverage (4_000_000), sqrt(4_000_000) = 2000, so fee_multiplier should be ~2000
        println!("4x fee multiplier: {}", profile_4x.fee_multiplier);
        assert!(profile_4x.fee_multiplier > 1800 && profile_4x.fee_multiplier < 2200);
    }

    #[test]
    fn test_margin_ratio_calculation() {
        let params = LeverageParameters {
            max_leverage: 10_000_000,
            current_ceiling: 10_000_000,
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Test margin ratio decreases with leverage
        let profile_2x = RiskProfile::from_leverage(2_000_000, &params).unwrap();
        let profile_5x = RiskProfile::from_leverage(5_000_000, &params).unwrap();
        let profile_10x = RiskProfile::from_leverage(10_000_000, &params).unwrap();

        assert!(profile_2x.required_margin_ratio > profile_5x.required_margin_ratio);
        // Higher leverage should have lower base margin ratio (but higher buffer)
        // The calculation is: base_margin = SCALE^2 / leverage + buffer
        // So 5x should have lower base margin than 2x, but buffer increases with leverage
        println!("2x margin: {}, 5x margin: {}, 10x margin: {}", 
                profile_2x.required_margin_ratio, 
                profile_5x.required_margin_ratio, 
                profile_10x.required_margin_ratio);
        // Just verify they're reasonable values for now
        assert!(profile_5x.required_margin_ratio < profile_2x.required_margin_ratio);

        // For 2x leverage, margin should be ~50% + buffer
        assert!(profile_2x.required_margin_ratio > 500_000);
        
        // For 10x leverage, margin should be ~10% + buffer
        assert!(profile_10x.required_margin_ratio > 100_000);
    }

    #[test]
    fn test_protection_curve_edge_cases() {
        // Test with max leverage = 1 (should fail)
        let params = LeverageParameters {
            max_leverage: 1_000_000,
            current_ceiling: 1_000_000,
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        // Only 1x leverage should work
        assert!(RiskProfile::from_leverage(1_000_000, &params).is_ok());
        assert!(RiskProfile::from_leverage(1_000_001, &params).is_err());

        // Test exponential with zero decay rate
        let params = LeverageParameters {
            max_leverage: 5_000_000,
            current_ceiling: 5_000_000,
            protection_curve: ProtectionCurve::Exponential { decay_rate: 0 },
            last_ceiling_update: 0,
            _padding: [0; 8],
        };

        let profile = RiskProfile::from_leverage(5_000_000, &params).unwrap();
        assert_eq!(profile.protection_factor, 1_000_000); // Should be 100% protection
    }

    #[test]
    fn test_leverage_ceiling_dynamics() {
        // Test that current ceiling can be lower than max leverage
        let mut params = LeverageParameters {
            max_leverage: 10_000_000, // 10x max
            current_ceiling: 5_000_000, // 5x current limit
            protection_curve: ProtectionCurve::Linear,
            last_ceiling_update: 1000,
            _padding: [0; 8],
        };

        // Should succeed at current ceiling
        assert!(RiskProfile::from_leverage(5_000_000, &params).is_ok());

        // Should fail above current ceiling
        assert!(RiskProfile::from_leverage(6_000_000, &params).is_err());

        // Update ceiling
        params.current_ceiling = 7_000_000;
        params.last_ceiling_update = 2000;

        // Now should succeed
        assert!(RiskProfile::from_leverage(6_000_000, &params).is_ok());
    }
}