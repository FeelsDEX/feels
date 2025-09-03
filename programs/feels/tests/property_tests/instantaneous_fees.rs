/// Property-based tests for instantaneous fee model with κ clamp.
/// Verifies fee calculation, rebate logic, and buffer constraints.

use anchor_lang::prelude::*;
use proptest::prelude::*;
use feels::state::{BufferAccount, buffer::{PriceImprovement, calculate_instantaneous_fee, calculate_price_improvement}};
use feels::logic::instantaneous_fee::{InstantaneousOrderResult, calculate_order_fees_instantaneous};
use feels::constant::Q64;

// ============================================================================
// Test Strategies
// ============================================================================

/// Generate valid work values
fn work_values() -> impl Strategy<Value = i128> {
    prop_oneof![
        // Positive work (fees)
        (1i128..1000000).prop_map(|v| v * (Q64 as i128) / 1000),
        // Negative work (rebates)
        (1i128..1000000).prop_map(|v| -(v * (Q64 as i128) / 1000)),
        // Zero work
        Just(0i128),
    ]
}

/// Generate valid κ values (basis points)
fn kappa_values() -> impl Strategy<Value = u32> {
    0u32..=10000 // 0% to 100%
}

/// Generate valid η values (basis points)
fn eta_values() -> impl Strategy<Value = u32> {
    0u32..=10000 // 0% to 100%
}

/// Generate price improvement scenarios
fn price_improvement() -> impl Strategy<Value = (u128, u128, bool)> {
    prop_oneof![
        // Buy with improvement (execution < oracle)
        (100u128..200, 80u128..100).prop_map(|(oracle, exec)| (oracle * Q64, exec * Q64, true)),
        // Buy without improvement (execution >= oracle)
        (100u128..200, 100u128..220).prop_map(|(oracle, exec)| (oracle * Q64, exec * Q64, true)),
        // Sell with improvement (execution > oracle)
        (100u128..200, 100u128..220).prop_map(|(oracle, exec)| (oracle * Q64, exec * Q64, false)),
        // Sell without improvement (execution <= oracle)
        (100u128..200, 80u128..100).prop_map(|(oracle, exec)| (oracle * Q64, exec * Q64, false)),
    ]
}

/// Generate buffer state
fn buffer_state() -> impl Strategy<Value = (u128, u128, u64, u64)> {
    (
        1000u128..1000000,  // tau_value
        0u128..500000,      // tau_reserved
        100u64..10000,      // rebate_cap_tx
        1000u64..100000,    // rebate_cap_epoch
    )
}

// ============================================================================
// Instantaneous Fee Properties
// ============================================================================

proptest! {
    /// Test 1: Fee formula correctness
    #[test]
    fn prop_instantaneous_fee_formula(
        work in work_values(),
        kappa in kappa_values(),
        improvement_bps in 0u64..1000, // 0% to 10% improvement
    ) {
        let price_improvement = PriceImprovement {
            oracle_price: 100 * Q64,
            execution_price: 95 * Q64,
            improvement_bps,
            is_buy: true,
        };
        
        let mut buffer = BufferAccount::default();
        buffer.kappa = kappa;
        buffer.rebate_eta = 5000; // 50%
        
        let (fee, rebate) = calculate_instantaneous_fee(work, &price_improvement, &buffer).unwrap();
        
        // Verify mutual exclusivity
        prop_assert!(!(fee > 0 && rebate > 0), "Fee and rebate cannot both be positive");
        
        if work > 0 {
            // Positive work case: fee = max(0, W - κ * improvement)
            let kappa_discount = (kappa as u128 * improvement_bps as u128) / 10000;
            let expected_fee = (work as u128).saturating_sub(kappa_discount);
            prop_assert_eq!(fee as u128, expected_fee, "Fee calculation incorrect");
            prop_assert_eq!(rebate, 0, "Rebate should be zero for positive work");
        } else if work < 0 {
            // Negative work case: rebate includes η factor and improvement bonus
            prop_assert_eq!(fee, 0, "Fee should be zero for negative work");
            prop_assert!(rebate > 0, "Rebate should be positive for negative work");
            
            let base_rebate = ((-work) as u128 * buffer.rebate_eta as u128) / 10000;
            let improvement_bonus = (kappa as u128 * improvement_bps as u128) / 10000;
            let expected_rebate = base_rebate + improvement_bonus;
            
            // Allow small rounding error
            prop_assert!(
                (rebate as i128 - expected_rebate as i128).abs() <= 1,
                "Rebate calculation incorrect: expected={}, actual={}", expected_rebate, rebate
            );
        } else {
            // Zero work case
            prop_assert_eq!(fee, 0);
            prop_assert_eq!(rebate, 0);
        }
    }
    
    /// Test 2: Price improvement calculation
    #[test]
    fn prop_price_improvement_calculation(
        (oracle_price, exec_price, is_buy) in price_improvement(),
    ) {
        let improvement = calculate_price_improvement(oracle_price, exec_price, is_buy);
        
        prop_assert_eq!(improvement.oracle_price, oracle_price);
        prop_assert_eq!(improvement.execution_price, exec_price);
        prop_assert_eq!(improvement.is_buy, is_buy);
        
        if is_buy {
            if exec_price < oracle_price {
                // Buy improvement
                let expected_bps = ((oracle_price - exec_price) * 10000) / oracle_price;
                prop_assert_eq!(improvement.improvement_bps, expected_bps as u64);
            } else {
                // No improvement
                prop_assert_eq!(improvement.improvement_bps, 0);
            }
        } else {
            if exec_price > oracle_price {
                // Sell improvement
                let expected_bps = ((exec_price - oracle_price) * 10000) / oracle_price;
                prop_assert_eq!(improvement.improvement_bps, expected_bps as u64);
            } else {
                // No improvement
                prop_assert_eq!(improvement.improvement_bps, 0);
            }
        }
    }
    
    /// Test 3: Buffer constraints respected
    #[test]
    fn prop_buffer_constraints_enforced(
        work in work_values(),
        (tau_value, tau_reserved, cap_tx, cap_epoch) in buffer_state(),
        rebate_paid_epoch in 0u64..50000,
    ) {
        // Skip if work is positive (no rebate)
        if work >= 0 {
            return Ok(());
        }
        
        let mut buffer = BufferAccount::default();
        buffer.tau_value = tau_value;
        buffer.tau_reserved = tau_reserved;
        buffer.rebate_cap_tx = cap_tx;
        buffer.rebate_cap_epoch = cap_epoch;
        buffer.rebate_paid_epoch = rebate_paid_epoch;
        buffer.rebate_eta = 5000;
        buffer.kappa = 1000;
        
        let available_tau = buffer.tau_value.saturating_sub(buffer.tau_reserved);
        let epoch_remaining = buffer.rebate_cap_epoch.saturating_sub(buffer.rebate_paid_epoch);
        
        let price_improvement = PriceImprovement {
            oracle_price: 100 * Q64,
            execution_price: 95 * Q64,
            improvement_bps: 500,
            is_buy: true,
        };
        
        let (_, rebate) = calculate_instantaneous_fee(work, &price_improvement, &buffer).unwrap();
        
        // Verify all caps are respected
        prop_assert!(rebate <= cap_tx, "Transaction cap violated");
        prop_assert!(rebate <= epoch_remaining, "Epoch cap violated");
        prop_assert!(rebate <= available_tau as u64, "Available tau exceeded");
    }
    
    /// Test 4: κ clamp effectiveness
    #[test]
    fn prop_kappa_clamp_reduces_fees(
        base_work in 1000i128..100000,
        kappa in 100u32..5000,
        improvement_bps in 100u64..1000,
    ) {
        let work = base_work * (Q64 as i128) / 10000;
        
        // Buffer with κ
        let mut buffer_with_kappa = BufferAccount::default();
        buffer_with_kappa.kappa = kappa;
        
        // Buffer without κ
        let mut buffer_no_kappa = BufferAccount::default();
        buffer_no_kappa.kappa = 0;
        
        let price_improvement = PriceImprovement {
            oracle_price: 100 * Q64,
            execution_price: 95 * Q64,
            improvement_bps,
            is_buy: true,
        };
        
        let (fee_with_kappa, _) = calculate_instantaneous_fee(work, &price_improvement, &buffer_with_kappa).unwrap();
        let (fee_no_kappa, _) = calculate_instantaneous_fee(work, &price_improvement, &buffer_no_kappa).unwrap();
        
        // With positive κ and price improvement, fees should be reduced
        prop_assert!(fee_with_kappa < fee_no_kappa, "κ clamp should reduce fees");
        
        // Verify reduction amount
        let expected_reduction = (kappa as u128 * improvement_bps as u128) / 10000;
        let actual_reduction = fee_no_kappa - fee_with_kappa;
        prop_assert_eq!(actual_reduction as u128, expected_reduction);
    }
}

// ============================================================================
// Fallback Mode Properties
// ============================================================================

proptest! {
    /// Test 5: Fallback fee calculation
    #[test]
    fn prop_fallback_fees_bounded(
        amount_in in 100u64..1000000,
        sigma_price in 100u64..5000, // 1% to 50% volatility
    ) {
        use feels::logic::instantaneous_fee::calculate_fallback_fees;
        use feels::state::MarketField;
        
        let mut market_field = MarketField::default();
        market_field.sigma_price = sigma_price;
        
        let buffer = BufferAccount::default();
        
        let fee = calculate_fallback_fees(amount_in, &market_field, &buffer).unwrap();
        
        // Base fee is 30 bps
        let base_fee = (amount_in as u128 * 30) / 10000;
        
        // Volatility multiplier between 1x and 3x
        let volatility_multiplier = (sigma_price / 100).min(3).max(1);
        let expected_max_fee = (base_fee * volatility_multiplier as u128) as u64;
        
        prop_assert!(fee <= expected_max_fee, "Fallback fee exceeds maximum");
        prop_assert!(fee > 0, "Fallback fee should be positive");
        
        // Fee should be less than 1% of input in all cases
        prop_assert!(fee < amount_in / 100, "Fallback fee too high");
    }
}

// ============================================================================
// Integration Properties
// ============================================================================

proptest! {
    /// Test 6: Order fee calculation consistency
    #[test]
    fn prop_order_fees_consistent(
        amount_in in 1000u64..1000000,
        amount_out in 1000u64..1000000,
        work in work_values(),
        oracle_price in 50u128..200,
        exec_price in 50u128..200,
        is_token_a_to_b in any::<bool>(),
    ) {
        use feels::state::TwapOracle;
        
        let mut oracle = TwapOracle::default();
        oracle.twap_1_per_0 = oracle_price * Q64;
        oracle.twap_a_per_b = Q64 * Q64 / oracle_price; // Reciprocal
        
        let mut buffer = BufferAccount::default();
        buffer.kappa = 1000;
        buffer.rebate_eta = 5000;
        buffer.tau_value = 1000000;
        
        let result = calculate_order_fees_instantaneous(
            amount_in,
            amount_out,
            is_token_a_to_b,
            work,
            &oracle,
            &buffer,
        ).unwrap();
        
        // Verify result consistency
        prop_assert_eq!(result.amount_in, amount_in);
        prop_assert_eq!(result.amount_out, amount_out);
        prop_assert_eq!(result.work_done, work);
        
        // Fee and rebate are mutually exclusive
        prop_assert!(!(result.fee_amount > 0 && result.rebate_amount > 0));
        
        // Effective fee calculation
        if result.fee_amount > 0 {
            let expected_bps = (result.fee_amount as u128 * 10000) / amount_in as u128;
            prop_assert_eq!(result.effective_fee_bps, expected_bps as u64);
        }
    }
}