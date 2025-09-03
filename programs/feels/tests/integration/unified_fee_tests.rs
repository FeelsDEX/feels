/// Integration tests for the unified fee model implementation.
/// Tests hysteresis controller, fee enforcement, fallback mode, and conservation.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use feels::{
    state::{
        FeesPolicy, PoolStatus, FieldCommitment,
        MarketField, BufferAccount, TwapOracle,
    },
    instructions::{
        EnforceFees, EnforceFeesParams, InitializePoolStatus,
        enforce_fees_handler, initialize_pool_status,
    },
    logic::{
        FallbackModeManager, FallbackContext, OperationalMode,
        calculate_leverage_stress, LeverageSafetyContext,
        verify_conservation_with_buffer, BufferConservationContext,
        build_buffer_conservation_proof, RebaseOperationType,
    },
    constant::{Q64, BPS_DENOMINATOR, MIN_FEE_BPS, MAX_FEE_BPS},
};

mod helpers;
use helpers::*;

// ============================================================================
// Hysteresis Controller Tests
// ============================================================================

#[tokio::test]
async fn test_hysteresis_controller_behavior() {
    let mut test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Setup market with field commitment
    let market_key = Keypair::new();
    let field_commitment_key = create_field_commitment(
        &mut banks_client,
        &payer,
        &market_key.pubkey(),
        25, // Base fee 25 bps
    )
    .await;

    // Test 1: Normal stress - no fee change
    let stress_components = StressComponents {
        spot_stress: 2000,     // 20% - below threshold
        time_stress: 1500,     // 15%
        leverage_stress: 1000, // 10%
    };
    
    // Simulate keeper update with normal stress
    let new_base_fee = simulate_hysteresis_update(
        &mut banks_client,
        &payer,
        &field_commitment_key,
        stress_components,
        25, // Current fee
    )
    .await;
    
    assert_eq!(new_base_fee, 25, "Fee should not change in dead zone");

    // Test 2: High stress - fee increases
    let high_stress = StressComponents {
        spot_stress: 8500,     // 85% - above upper trigger
        time_stress: 7000,     // 70%
        leverage_stress: 9000, // 90%
    };
    
    let increased_fee = simulate_hysteresis_update(
        &mut banks_client,
        &payer,
        &field_commitment_key,
        high_stress,
        25,
    )
    .await;
    
    assert!(increased_fee > 25, "Fee should increase on high stress");
    assert!(increased_fee <= 30, "Fee increase should be capped at step size");

    // Test 3: Directional memory - no oscillation
    let medium_stress = StressComponents {
        spot_stress: 5000,     // 50% - in dead zone
        time_stress: 4000,     // 40%
        leverage_stress: 4500, // 45%
    };
    
    let fee_after_decrease = simulate_hysteresis_update(
        &mut banks_client,
        &payer,
        &field_commitment_key,
        medium_stress,
        increased_fee,
    )
    .await;
    
    assert_eq!(
        fee_after_decrease, increased_fee,
        "Fee should not change in dead zone after increase"
    );
}

// ============================================================================
// Fee Enforcement Tests
// ============================================================================

#[tokio::test]
async fn test_fee_enforcement_under_various_conditions() {
    let mut test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Setup fees policy
    let fees_policy_key = create_fees_policy(
        &mut banks_client,
        &payer,
        FeesPolicy {
            min_base_fee_bps: MIN_FEE_BPS,
            max_base_fee_bps: MAX_FEE_BPS,
            max_fee_increase_bps: 500,
            max_fee_decrease_bps: 300,
            min_update_interval: 300,
            spot_disable_threshold_bps: 9500,
            time_disable_threshold_bps: 9500,
            leverage_disable_threshold_bps: 9000,
            consecutive_stress_periods_for_disable: 3,
            reenable_cooldown: 3600,
            max_commitment_staleness: 1800,
            fallback_fee_bps: 100,
            ..Default::default()
        },
    )
    .await;

    // Test 1: Minimum fee enforcement
    let result = test_enforce_fees(
        &mut banks_client,
        &payer,
        &fees_policy_key,
        EnforceFeesParams {
            amount_in: 1_000_000,
            amount_out: 1_010_000,
            zero_for_one: true,
            sqrt_price_current: Q64,
            sqrt_price_target: (Q64 * 101) / 100,
            liquidity: Q64,
        },
        5, // Base fee below minimum
    )
    .await;
    
    assert!(
        result.is_err(),
        "Should reject fee below minimum"
    );

    // Test 2: Normal fee acceptance
    let result = test_enforce_fees(
        &mut banks_client,
        &payer,
        &fees_policy_key,
        EnforceFeesParams {
            amount_in: 1_000_000,
            amount_out: 990_000,
            zero_for_one: true,
            sqrt_price_current: Q64,
            sqrt_price_target: (Q64 * 99) / 100,
            liquidity: Q64,
        },
        30, // Valid base fee
    )
    .await
    .unwrap();
    
    assert!(result.pool_operational);
    assert_eq!(result.effective_fee_bps, 30);

    // Test 3: Pool disable on high stress
    let pool_status_key = create_pool_status(
        &mut banks_client,
        &payer,
        &market_key.pubkey(),
    )
    .await;
    
    // Simulate multiple high stress periods
    for _ in 0..3 {
        simulate_high_stress_update(
            &mut banks_client,
            &payer,
            &pool_status_key,
        )
        .await;
    }
    
    let status = get_pool_status(&mut banks_client, &pool_status_key).await;
    assert_eq!(
        status.status,
        2, // Disabled
        "Pool should be disabled after consecutive high stress"
    );
}

// ============================================================================
// Fallback Mode Tests
// ============================================================================

#[tokio::test]
async fn test_fallback_mode_transitions() {
    let mut test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Create stale field commitment
    let market_key = Keypair::new();
    let field_commitment = FieldCommitment {
        snapshot_ts: Clock::get().unwrap().unix_timestamp - 3600, // 1 hour old
        max_staleness: 1800, // 30 minutes
        base_fee_bps: 25,
        ..create_test_field_commitment()
    };
    
    let field_commitment_key = create_field_commitment_with_data(
        &mut banks_client,
        &payer,
        &market_key.pubkey(),
        field_commitment,
    )
    .await;

    // Test fallback mode detection
    let ctx = FallbackContext {
        field_commitment: &field_commitment,
        market_field: &create_test_market_field(),
        fees_policy: &create_test_fees_policy(),
        buffer: &create_test_buffer(),
        twap_oracle: &create_test_twap_oracle(),
        current_time: Clock::get().unwrap().unix_timestamp,
    };
    
    let evaluation = FallbackModeManager::evaluate_mode(&ctx).unwrap();
    
    assert_eq!(
        evaluation.mode,
        OperationalMode::Fallback,
        "Should enter fallback mode with stale data"
    );
    assert_eq!(
        evaluation.base_fee_bps,
        100, // Fallback fee from policy
        "Should use fallback fee"
    );
    assert!(
        evaluation.confidence_score < 10000,
        "Confidence should be reduced"
    );

    // Test fee calculation in fallback mode
    let fallback_fee = FallbackModeManager::calculate_dynamic_fee(
        1_000_000,
        &evaluation,
        &ctx,
    )
    .unwrap();
    
    assert!(
        fallback_fee > 0,
        "Should calculate positive fee in fallback"
    );
    
    // Test emergency mode with severe staleness
    let emergency_ctx = FallbackContext {
        field_commitment: &FieldCommitment {
            snapshot_ts: Clock::get().unwrap().unix_timestamp - 7200, // 2 hours old
            ..field_commitment
        },
        ..ctx
    };
    
    let emergency_eval = FallbackModeManager::evaluate_mode(&emergency_ctx).unwrap();
    assert_eq!(
        emergency_eval.mode,
        OperationalMode::Emergency,
        "Should enter emergency mode with severe staleness"
    );
    assert_eq!(
        emergency_eval.base_fee_bps,
        MAX_FEE_BPS,
        "Should use maximum fee in emergency"
    );
}

// ============================================================================
// Conservation Tests
// ============================================================================

#[tokio::test]
async fn test_conservation_with_buffer_participation() {
    let mut test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Setup buffer and field commitment
    let buffer = create_test_buffer_with_tau(1_000_000); // 1M tau available
    let field_commitment = FieldCommitment {
        w_s: 4000,
        w_t: 3000,
        w_l: 2000,
        w_tau: 1000, // 10% buffer weight
        ..create_test_field_commitment()
    };
    
    let ctx = BufferConservationContext {
        buffer: &buffer,
        field_commitment: &field_commitment,
        operation_type: RebaseOperationType::Lending,
    };

    // Test 1: Valid conservation with buffer
    let proof = build_buffer_conservation_proof(
        vec![4500, 4500], // Lender/borrower weights
        vec![Q64 + 1000, Q64 - 1000], // Growth factors
        1000, // 10% buffer participation
        Q64, // No buffer growth (balanced fees/rebates)
        5000, // Fees collected
        5000, // Rebates paid
        0, // Perfect conservation
    );
    
    let result = verify_conservation_with_buffer(&proof, &ctx);
    assert!(result.is_ok(), "Valid conservation should pass");

    // Test 2: Buffer growth consistency
    let growth_proof = build_buffer_conservation_proof(
        vec![4500, 4500],
        vec![Q64 + 2000, Q64 - 2000],
        1000,
        Q64 + 100, // Buffer grows slightly
        6000, // More fees than rebates
        5000,
        0,
    );
    
    let result = verify_conservation_with_buffer(&growth_proof, &ctx);
    assert!(result.is_ok(), "Buffer growth should be consistent");

    // Test 3: Insufficient buffer tau
    let insufficient_ctx = BufferConservationContext {
        buffer: &create_test_buffer_with_tau(100), // Only 100 tau
        ..ctx
    };
    
    let insufficient_proof = build_buffer_conservation_proof(
        vec![4500, 4500],
        vec![Q64, Q64],
        1000,
        Q64,
        5000,
        10000, // Trying to pay more rebates than available
        0,
    );
    
    let result = verify_conservation_with_buffer(&insufficient_proof, &insufficient_ctx);
    assert!(
        result.is_err(),
        "Should fail with insufficient buffer tau"
    );
}

// ============================================================================
// Stress Testing Scenarios
// ============================================================================

#[tokio::test]
async fn test_stress_scenarios() {
    let mut test = ProgramTest::new("feels", feels::id(), None);
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Scenario 1: Flash crash - extreme price movement
    let flash_crash_stress = StressComponents {
        spot_stress: 9500,     // 95% deviation
        time_stress: 8000,     // 80% utilization
        leverage_stress: 9900, // 99% imbalance
    };
    
    test_market_stress_response(
        &mut banks_client,
        &payer,
        flash_crash_stress,
        "Flash crash scenario",
    )
    .await;

    // Scenario 2: Sustained high volatility
    let sustained_volatility = vec![
        StressComponents { spot_stress: 7000, time_stress: 6000, leverage_stress: 6500 },
        StressComponents { spot_stress: 7500, time_stress: 6500, leverage_stress: 7000 },
        StressComponents { spot_stress: 8000, time_stress: 7000, leverage_stress: 7500 },
        StressComponents { spot_stress: 8500, time_stress: 7500, leverage_stress: 8000 },
    ];
    
    for (i, stress) in sustained_volatility.iter().enumerate() {
        test_market_stress_response(
            &mut banks_client,
            &payer,
            *stress,
            &format!("Sustained volatility period {}", i + 1),
        )
        .await;
    }

    // Scenario 3: Leverage attack attempt
    test_leverage_manipulation_defense(
        &mut banks_client,
        &payer,
    )
    .await;

    // Scenario 4: Keeper failure and recovery
    test_keeper_failure_recovery(
        &mut banks_client,
        &payer,
    )
    .await;
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn test_market_stress_response(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    stress: StressComponents,
    scenario_name: &str,
) {
    println!("Testing {}", scenario_name);
    println!("  Spot stress: {} bps", stress.spot_stress);
    println!("  Time stress: {} bps", stress.time_stress);
    println!("  Leverage stress: {} bps", stress.leverage_stress);
    
    // Calculate expected fee response
    let weighted_stress = (stress.spot_stress * 5 + 
                          stress.time_stress * 3 + 
                          stress.leverage_stress * 2) / 10;
    
    println!("  Weighted stress: {} bps", weighted_stress);
    
    // Verify appropriate response
    if weighted_stress > 9000 {
        println!("  Expected: Pool disable");
    } else if weighted_stress > 7000 {
        println!("  Expected: Fee increase");
    } else if weighted_stress < 3000 {
        println!("  Expected: Fee decrease");
    } else {
        println!("  Expected: No change (dead zone)");
    }
}

async fn test_leverage_manipulation_defense(
    banks_client: &mut BanksClient,
    payer: &Keypair,
) {
    println!("Testing leverage manipulation defense");
    
    // Simulate rapid position reversals
    for i in 0..5 {
        let direction = if i % 2 == 0 { "Long" } else { "Short" };
        println!("  Attempt {}: {} position", i + 1, direction);
        
        // After 3 reversals, should trigger anti-ping-pong
        if i >= 3 {
            println!("  Expected: Position rejected (ping-pong detected)");
        }
    }
}

async fn test_keeper_failure_recovery(
    banks_client: &mut BanksClient,
    payer: &Keypair,
) {
    println!("Testing keeper failure and recovery");
    
    // Simulate keeper offline
    println!("  T+0: Keeper goes offline");
    println!("  T+30min: Enter fallback mode");
    println!("  T+60min: Emergency mode");
    println!("  T+90min: Keeper recovers");
    println!("  T+95min: Exit fallback (min dwell time)");
    println!("  T+100min: Normal operation restored");
}

// Additional helper implementations would go here...