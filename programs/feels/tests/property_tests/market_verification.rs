/// Property-based tests for market update verification.
/// Ensures all validation rules are consistently enforced.

use anchor_lang::prelude::*;
use proptest::prelude::*;
use feels::state::{
    MarketDataSource, UnifiedMarketUpdate, FieldCommitmentData,
    DATA_SOURCE_TYPE_KEEPER, DATA_SOURCE_TYPE_ORACLE, DATA_SOURCE_TYPE_HYBRID,
};
use feels::logic::field_verification::{
    verify_market_update_enhanced, FieldVerificationProof,
    ConvexBoundPoint, LipschitzSample, OptimalityGapProof,
};

// ============================================================================
// Test Strategies
// ============================================================================

/// Generate valid source types
fn source_types() -> impl Strategy<Value = u8> {
    prop_oneof![
        Just(DATA_SOURCE_TYPE_KEEPER),
        Just(DATA_SOURCE_TYPE_ORACLE),
        Just(DATA_SOURCE_TYPE_HYBRID),
    ]
}

/// Generate timestamps
fn timestamps() -> impl Strategy<Value = i64> {
    prop_oneof![
        // Recent timestamps
        (-300i64..0).prop_map(|delta| Clock::get().unwrap().unix_timestamp + delta),
        // Future timestamps (should fail)
        (1i64..300).prop_map(|delta| Clock::get().unwrap().unix_timestamp + delta),
        // Old timestamps (should fail if too old)
        (-3600i64..-301).prop_map(|delta| Clock::get().unwrap().unix_timestamp + delta),
    ]
}

/// Generate sequence numbers
fn sequence_numbers(current: u64) -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(current + 1),           // Valid next sequence
        Just(current),               // Same (invalid)
        (0u64..current),             // Past (invalid)
        (current + 2..current + 10), // Future skip (invalid)
    ]
}

/// Generate scalar change scenarios
fn scalar_changes() -> impl Strategy<Value = (u128, u128)> {
    let base = 1_000_000_000u128; // Base value
    prop_oneof![
        // Small changes (valid)
        (0u128..100).prop_map(move |pct| {
            let change = base * pct / 1000; // 0-10% change
            (base, base + change)
        }),
        // Large changes (invalid)
        (200u128..1000).prop_map(move |pct| {
            let change = base * pct / 1000; // 20-100% change
            (base, base + change)
        }),
        // Zero to non-zero (always valid)
        Just((0u128, base)),
    ]
}

// ============================================================================
// Verification Properties
// ============================================================================

proptest! {
    /// Test 1: Source authorization enforcement
    #[test]
    fn prop_source_authorization(
        configured_type in source_types(),
        update_source in source_types(),
        sequence in 1u64..1000,
        timestamp in timestamps(),
    ) {
        let mut data_source = MarketDataSource::default();
        data_source.config.source_type = configured_type;
        data_source.last_sequence = sequence - 1;
        
        let update = UnifiedMarketUpdate {
            source: update_source,
            field_commitment: Some(create_valid_field_commitment()),
            price_data: None,
            timestamp,
            sequence,
        };
        
        let current_field = Default::default();
        let current_time = Clock::get().unwrap().unix_timestamp;
        
        let result = verify_market_update_enhanced(
            &update,
            &data_source,
            &current_field,
            current_time,
        );
        
        // Verify authorization rules
        let should_succeed = match configured_type {
            DATA_SOURCE_TYPE_KEEPER => update_source == DATA_SOURCE_TYPE_KEEPER,
            DATA_SOURCE_TYPE_ORACLE => update_source == DATA_SOURCE_TYPE_ORACLE,
            DATA_SOURCE_TYPE_HYBRID => {
                update_source == DATA_SOURCE_TYPE_KEEPER || 
                update_source == DATA_SOURCE_TYPE_ORACLE
            }
            _ => false,
        };
        
        if should_succeed && timestamp <= current_time && timestamp >= current_time - 300 {
            prop_assert!(result.is_ok(), "Valid update should succeed");
        } else {
            prop_assert!(result.is_err(), "Invalid update should fail");
        }
    }
    
    /// Test 2: Sequence number monotonicity
    #[test]
    fn prop_sequence_number_monotonic(
        current_seq in 0u64..1000,
        update_seq in sequence_numbers(current_seq),
    ) {
        let mut data_source = MarketDataSource::default();
        data_source.config.source_type = DATA_SOURCE_TYPE_KEEPER;
        data_source.last_sequence = current_seq;
        
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(create_valid_field_commitment()),
            price_data: None,
            timestamp: Clock::get().unwrap().unix_timestamp,
            sequence: update_seq,
        };
        
        let current_field = Default::default();
        let current_time = Clock::get().unwrap().unix_timestamp;
        
        let result = verify_market_update_enhanced(
            &update,
            &data_source,
            &current_field,
            current_time,
        );
        
        if update_seq > current_seq {
            prop_assert!(result.is_ok(), "Monotonic increase should succeed");
        } else {
            prop_assert!(result.is_err(), "Non-monotonic sequence should fail");
        }
    }
    
    /// Test 3: Rate of change limits
    #[test]
    fn prop_rate_of_change_limits(
        (current_s, new_s) in scalar_changes(),
        (current_t, new_t) in scalar_changes(),
        (current_l, new_l) in scalar_changes(),
    ) {
        let data_source = MarketDataSource::default();
        
        let mut current_field = Default::default();
        current_field.S = current_s;
        current_field.T = current_t;
        current_field.L = current_l;
        
        let mut commitment = create_valid_field_commitment();
        commitment.S = new_s;
        commitment.T = new_t;
        commitment.L = new_l;
        
        let update = UnifiedMarketUpdate {
            source: DATA_SOURCE_TYPE_KEEPER,
            field_commitment: Some(commitment),
            price_data: None,
            timestamp: Clock::get().unwrap().unix_timestamp,
            sequence: 1,
        };
        
        let result = verify_market_update_enhanced(
            &update,
            &data_source,
            &current_field,
            Clock::get().unwrap().unix_timestamp,
        );
        
        // Check if all changes are within 5% limit
        let s_ok = check_rate_limit(current_s, new_s, 500);
        let t_ok = check_rate_limit(current_t, new_t, 500);
        let l_ok = check_rate_limit(current_l, new_l, 500);
        
        if s_ok && t_ok && l_ok {
            prop_assert!(result.is_ok(), "Changes within limit should succeed");
        } else {
            prop_assert!(result.is_err(), "Changes exceeding limit should fail");
        }
    }
    
    /// Test 4: Convex bound verification
    #[test]
    fn prop_convex_bound_verification(
        num_points in 3usize..10,
        violations in 0usize..5,
    ) {
        let field_commitment = Default::default();
        
        let mut proof = FieldVerificationProof {
            convex_bound_points: Vec::new(),
            lipschitz_samples: Vec::new(),
            optimality_gap: OptimalityGapProof {
                current_value: 1000,
                optimal_bound: 1000,
                gap_bps: 0,
            },
            merkle_proof: None,
        };
        
        // Generate valid points
        for i in 0..num_points {
            let valid = i >= violations;
            proof.convex_bound_points.push(ConvexBoundPoint {
                position: [1000, 1000, 1000],
                V: if valid { 100 } else { 200 }, // Violate if needed
                bound: 150,
            });
        }
        
        use feels::logic::field_verification::verify_convex_bound;
        let result = verify_convex_bound(&proof, &field_commitment);
        
        if violations == 0 {
            prop_assert!(result.is_ok(), "No violations should pass");
        } else {
            prop_assert!(result.is_err(), "Violations should fail");
        }
    }
    
    /// Test 5: Lipschitz continuity verification
    #[test]
    fn prop_lipschitz_verification(
        lipschitz_L in 100u64..1000,
        num_samples in 1usize..5,
        violation_ratio in 0f64..2.0,
    ) {
        let mut field_commitment = Default::default();
        field_commitment.lipschitz_L = Some(lipschitz_L);
        
        let mut proof = FieldVerificationProof {
            convex_bound_points: Vec::new(),
            lipschitz_samples: Vec::new(),
            optimality_gap: OptimalityGapProof {
                current_value: 1000,
                optimal_bound: 1000,
                gap_bps: 0,
            },
            merkle_proof: None,
        };
        
        // Generate samples
        for _ in 0..num_samples {
            let pos_diff = 100u128;
            let grad_diff = (pos_diff as f64 * lipschitz_L as f64 * violation_ratio) as u128;
            
            proof.lipschitz_samples.push(LipschitzSample {
                p1: [1000, 1000, 1000],
                p2: [1100, 1100, 1100],
                grad_diff_norm: grad_diff,
                pos_diff_norm: pos_diff,
            });
        }
        
        use feels::logic::field_verification::verify_lipschitz_inequality;
        let result = verify_lipschitz_inequality(&proof, &field_commitment);
        
        if violation_ratio <= 1.0 {
            prop_assert!(result.is_ok(), "Valid Lipschitz bound should pass");
        } else {
            prop_assert!(result.is_err(), "Lipschitz violation should fail");
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_valid_field_commitment() -> FieldCommitmentData {
    FieldCommitmentData {
        S: 1_000_000_000,
        T: 1_000_000_000,
        L: 1_000_000_000,
        w_s: 2500,
        w_t: 2500,
        w_l: 2500,
        w_tau: 2500,
        omega_0: 5000,
        omega_1: 5000,
        twap_0: 1_000_000,
        twap_1: 1_000_000,
        max_staleness: 300,
    }
}

fn check_rate_limit(current: u128, new: u128, max_change_bps: u64) -> bool {
    if current == 0 {
        return true; // Any change from zero is allowed
    }
    
    let change_ratio = if new > current {
        ((new - current) * 10000) / current
    } else {
        ((current - new) * 10000) / current
    };
    
    change_ratio <= max_change_bps as u128
}