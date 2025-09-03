/// Comprehensive tests for the canonical math implementation
/// Tests all fixed-point mathematical functions and utilities using library-based implementations
use anchor_lang::prelude::*;
use feels::utils::math::{
    FixedPoint, FixedPointExt, ln_fixed, exp_fixed, pow_fixed, sqrt_fixed,
    calculate_growth_factor, solve_conservation_rebase, verify_conservation
};

// ============================================================================
// FixedPoint Basic Operations Tests
// ============================================================================

#[test]
fn test_fixed_point_construction() {
    // Test various construction methods using library-based FixedPoint
    let zero = FixedPoint::ZERO;
    assert_eq!(zero.value(), 0);
    
    let one = FixedPoint::ONE;
    assert_eq!(one.value(), FixedPoint::SCALE);
    
    let from_int = FixedPoint::from_int(42);
    assert_eq!(from_int.value(), 42 * FixedPoint::SCALE);
    
    let from_u64 = FixedPoint::from_u64(100);
    assert_eq!(from_u64.value(), 100 * FixedPoint::SCALE);
    
    let from_u128 = FixedPoint::from_u128(2).unwrap(); // 2.0
    assert_eq!(from_u128.to_u64(), 2);
}

#[test]
fn test_fixed_point_arithmetic() {
    let a = FixedPoint::from_int(3);  // 3.0
    let b = FixedPoint::from_int(2);  // 2.0
    
    // Addition
    let sum = a.add(b).unwrap();
    assert_eq!(sum.to_u64(), 5);
    
    // Subtraction
    let diff = a.sub(b).unwrap();
    assert_eq!(diff.to_u64(), 1);
    
    // Multiplication
    let product = a.mul(b).unwrap();
    assert_eq!(product.to_u64(), 6);
    
    // Division
    let quotient = a.div(b).unwrap();
    assert_eq!(quotient.to_u64(), 1); // 1.5 truncated to 1
    
    // More precise division test
    let precise_quotient = FixedPoint::from_int(15).div(FixedPoint::from_int(10)).unwrap();
    // Should be 1.5, check if close
    let expected_15 = (15 * FixedPoint::SCALE) / 10;
    assert!((precise_quotient.to_f64() - 1.5).abs() < 0.01);
}

#[test]
fn test_fixed_point_comparison() {
    let a = FixedPoint::from_int(5);
    let b = FixedPoint::from_int(3);
    let c = FixedPoint::from_int(5);
    
    assert!(a > b);
    assert!(b < a);
    assert_eq!(a, c);
    assert!(a >= c);
    assert!(a <= c);
}

#[test]
fn test_fixed_point_negation() {
    let pos = FixedPoint::from_int(42);
    let neg = pos.neg();
    
    assert_eq!(neg.value(), -42 * FixedPoint::SCALE);
    
    let double_neg = neg.neg();
    assert_eq!(double_neg, pos);
}

// ============================================================================
// Natural Logarithm Tests
// ============================================================================

#[test]
fn test_ln_special_cases() {
    // ln(1) = 0 using library-based implementation
    let ln_one = ln_fixed(FixedPoint::ONE).unwrap();
    assert!(ln_one.value().abs() < FixedPoint::SCALE / 1000); // Within 0.001
    
    // ln should fail for zero and negative numbers
    assert!(ln_fixed(FixedPoint::ZERO).is_err());
    assert!(ln_fixed(FixedPoint::from_int(-1)).is_err());
}

#[test]
fn test_ln_properties() {
    let two = FixedPoint::from_int(2);
    let four = FixedPoint::from_int(4);
    let eight = FixedPoint::from_int(8);
    
    let ln2 = ln_fixed(two).unwrap();
    let ln4 = ln_fixed(four).unwrap();
    let ln8 = ln_fixed(eight).unwrap();
    
    // ln(4) should be approximately 2 * ln(2)
    let expected_ln4 = ln2.mul(FixedPoint::from_int(2)).unwrap();
    let diff = (ln4.value() - expected_ln4.value()).abs();
    assert!(diff < FixedPoint::SCALE / 10); // Within 10% tolerance
    
    // ln(8) should be approximately 3 * ln(2)
    let expected_ln8 = ln2.mul(FixedPoint::from_int(3)).unwrap();
    let diff = (ln8.value() - expected_ln8.value()).abs();
    assert!(diff < FixedPoint::SCALE / 10);
}

// ============================================================================
// Exponential Function Tests
// ============================================================================

#[test]
fn test_exp_special_cases() {
    // exp(0) = 1
    let exp_zero = exp_fixed(FixedPoint::ZERO).unwrap();
    let diff = (exp_zero.value() - FixedPoint::ONE.value()).abs();
    assert!(diff < FixedPoint::SCALE / 1000); // Within 0.001
}

#[test]
fn test_exp_ln_inverse() {
    let values = [
        FixedPoint::from_u64(1).unwrap(),
        FixedPoint::from_u64(2).unwrap(),
        FixedPoint::from_u64(3).unwrap(),
        FixedPoint::from_int(5),
    ];
    
    for &val in &values {
        let ln_val = ln_fixed(val).unwrap();
        let exp_ln_val = exp_fixed(ln_val).unwrap();
        
        // exp(ln(x)) should equal x
        let diff = (exp_ln_val.value() - val.value()).abs();
        let tolerance = val.value() / 10; // 10% tolerance
        assert!(diff < tolerance, 
            "exp(ln({})) = {} != {}, diff = {}", 
            val.to_u64(), exp_ln_val.to_u64(), val.to_u64(), diff
        );
    }
}

// ============================================================================
// Power Function Tests
// ============================================================================

#[test]
fn test_pow_integer_exponents() {
    let base = FixedPoint::from_int(2);
    
    // 2^0 = 1
    let pow0 = pow_fixed(base, FixedPoint::ZERO).unwrap();
    assert_eq!(pow0.to_u64(), 1);
    
    // 2^1 = 2
    let pow1 = pow_fixed(base, FixedPoint::ONE).unwrap();
    assert_eq!(pow1.to_u64(), 2);
    
    // 2^2 = 4
    let pow2 = pow_fixed(base, FixedPoint::from_int(2)).unwrap();
    assert_eq!(pow2.to_u64(), 4);
    
    // 2^3 = 8
    let pow3 = pow_fixed(base, FixedPoint::from_int(3)).unwrap();
    assert_eq!(pow3.to_u64(), 8);
}

#[test]
fn test_pow_fractional_exponents() {
    let four = FixedPoint::from_int(4);
    let half = FixedPoint::from_int(1).div(FixedPoint::from_int(2)).unwrap();
    
    // 4^0.5 should be close to 2
    let sqrt4 = pow_fixed(four, half).unwrap();
    let diff = (sqrt4.to_u64() as i64 - 2).abs();
    assert!(diff <= 1); // Should be very close to 2
}

// ============================================================================
// Square Root Tests
// ============================================================================

#[test]
fn test_sqrt_perfect_squares() {
    let test_cases = [
        (0, 0),
        (1, 1),
        (4, 2),
        (9, 3),
        (16, 4),
        (25, 5),
        (100, 10),
    ];
    
    for &(input, expected) in &test_cases {
        let fp_input = FixedPoint::from_int(input);
        let result = sqrt_fixed(fp_input).unwrap();
        assert_eq!(result.to_u64(), expected);
    }
}

#[test]
fn test_sqrt_non_perfect_squares() {
    // Test sqrt(2) ≈ 1.414
    let sqrt2 = sqrt_fixed(FixedPoint::from_int(2)).unwrap();
    // Convert to fixed point representation of 1.414
    let expected = (1414 * FixedPoint::SCALE) / 1000; // 1.414 in Q64
    let diff = (sqrt2.value - expected).abs();
    let tolerance = FixedPoint::SCALE / 100; // 1% tolerance
    assert!(diff < tolerance);
    
    // Test sqrt(10) ≈ 3.162
    let sqrt10 = sqrt_fixed(FixedPoint::from_int(10)).unwrap();
    let expected = (3162 * FixedPoint::SCALE) / 1000; // 3.162 in Q64
    let diff = (sqrt10.value - expected).abs();
    assert!(diff < tolerance);
}

// ============================================================================
// Growth Factor Tests
// ============================================================================

#[test]
fn test_growth_factor_calculation() {
    // Test simple growth: from 100 to 110 should be 1.1x growth
    let old_value = 100u64;
    let new_value = 110u64;
    
    let growth_factor = calculate_growth_factor(old_value, new_value).unwrap();
    
    // Should be close to 1.1 in fixed point
    let expected = (11 * FixedPoint::SCALE) / 10; // 1.1 in Q64
    let diff = (growth_factor - expected).abs();
    let tolerance = FixedPoint::SCALE / 1000; // 0.001 tolerance
    assert!(diff < tolerance);
}

#[test]
fn test_growth_factor_edge_cases() {
    // Same values should give 1.0x growth
    let same_growth = calculate_growth_factor(100, 100).unwrap();
    assert_eq!(same_growth, FixedPoint::SCALE);
    
    // Zero old value should fail
    assert!(calculate_growth_factor(0, 100).is_err());
    
    // Decrease should work (growth < 1.0)
    let decrease = calculate_growth_factor(100, 90).unwrap();
    assert!(decrease < FixedPoint::SCALE);
}

// ============================================================================
// Conservation Tests
// ============================================================================

#[test]
fn test_conservation_exact() {
    // Test exact conservation: w1*ln(g1) + w2*ln(g2) = 0
    // If w1 = w2 = equal weights, then g1 * g2 = 1 (in log space)
    let weights = [FixedPoint::from_u64(5000), FixedPoint::from_u64(5000)];
    let g1 = FixedPoint::from_scaled(FixedPoint::SCALE + (FixedPoint::SCALE >> 4)); // ~1.0625
    let g2 = FixedPoint::from_scaled((FixedPoint::SCALE * FixedPoint::SCALE) / g1.value); // 1/g1 ≈ 0.9412
    
    let growth_factors = [g1, g2];
    let tolerance = FixedPoint::from_scaled(FixedPoint::SCALE / 1000); // 0.001 tolerance
    
    assert!(verify_conservation(&weights, &growth_factors, tolerance).unwrap());
}

#[test]
fn test_conservation_violation() {
    // Test conservation violation
    let weights = [FixedPoint::from_u64(5000), FixedPoint::from_u64(5000)];
    let growth_factors = [
        FixedPoint::from_scaled(FixedPoint::SCALE + (FixedPoint::SCALE >> 2)), // 1.25
        FixedPoint::from_scaled(FixedPoint::SCALE + (FixedPoint::SCALE >> 2)), // 1.25 (both growing)
    ];
    let tolerance = FixedPoint::from_scaled(FixedPoint::SCALE / 1000); // 0.001 tolerance
    
    // This should violate conservation
    assert!(!verify_conservation(&weights, &growth_factors, tolerance).unwrap());
}

#[test]
fn test_conservation_solver() {
    // Test solving for the third factor in a 3-weight system
    let weights = [
        FixedPoint::from_u64(3000), 
        FixedPoint::from_u64(3000), 
        FixedPoint::from_u64(4000)
    ];
    let target_factors = [
        Some(FixedPoint::from_scaled(FixedPoint::SCALE + (FixedPoint::SCALE >> 2))), // 1.25
        Some(FixedPoint::from_scaled(FixedPoint::SCALE - (FixedPoint::SCALE >> 3))), // 0.875
        None, // To be solved
    ];
    
    let solved_factors = solve_conservation_rebase(&weights, &target_factors).unwrap();
    
    // Verify conservation with all factors
    let tolerance = FixedPoint::from_scaled(FixedPoint::SCALE / 1000); // 0.001 tolerance
    assert!(verify_conservation(&weights, &solved_factors, tolerance).unwrap());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_mathematical_identities() {
    let x = FixedPoint::from_int(3);
    let y = FixedPoint::from_int(2);
    
    // Test ln(x*y) = ln(x) + ln(y)
    let xy = x.mul(y).unwrap();
    let ln_xy = ln_fixed(xy).unwrap();
    let ln_x = ln_fixed(x).unwrap();
    let ln_y = ln_fixed(y).unwrap();
    let ln_x_plus_ln_y = ln_x.add(ln_y).unwrap();
    
    let diff = (ln_xy.value - ln_x_plus_ln_y.value).abs();
    let tolerance = FixedPoint::SCALE / 10; // 10% tolerance for approximation
    assert!(diff < tolerance);
}

#[test]
fn test_precision_limits() {
    // Test behavior near precision limits
    let tiny = FixedPoint { value: 1 }; // Smallest positive value
    let huge = FixedPoint { value: i128::MAX >> 1 }; // Large value
    
    // Operations with tiny values
    let tiny_plus_one = tiny.add(FixedPoint::ONE).unwrap();
    assert!(tiny_plus_one > FixedPoint::ONE);
    
    // Test overflow protection
    assert!(huge.add(huge).is_err()); // Should overflow
}

#[test]
fn test_rebase_scenario() {
    // Realistic rebase scenario: 3 domains with different growth rates
    let weights = [
        FixedPoint::from_u64(3333), 
        FixedPoint::from_u64(3333), 
        FixedPoint::from_u64(3334)
    ]; // ~33.33% each
    
    // Market grows 10%, Time decays 5%, Leverage to be solved
    let market_growth = FixedPoint::from_scaled((11 * FixedPoint::SCALE) / 10); // 1.1
    let time_decay = FixedPoint::from_scaled((95 * FixedPoint::SCALE) / 100);   // 0.95
    
    let target_factors = [Some(market_growth), Some(time_decay), None];
    let solved_factors = solve_conservation_rebase(&weights, &target_factors).unwrap();
    
    // Verify the solution
    let tolerance = FixedPoint::from_scaled(FixedPoint::SCALE / 1000);
    assert!(verify_conservation(&weights, &solved_factors, tolerance).unwrap());
    
    // Leverage should compensate (be > 1.0 to offset time decay)
    assert!(solved_factors[2].value > FixedPoint::SCALE);
}