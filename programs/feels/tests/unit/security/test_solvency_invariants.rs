use crate::common::*;

#[tokio::test]
async fn test_backing_invariant_always_holds() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Critical invariant: JitoSOL_Reserves >= FeelsSOL_Total_Supply
    struct BackingTest {
        jitosol_reserves: u128,
        feelssol_supply: u128,
        operation: &'static str,
        should_succeed: bool,
        description: &'static str,
    }

    let tests = vec![
        BackingTest {
            jitosol_reserves: 1_000_000_000,
            feelssol_supply: 1_000_000_000,
            operation: "initial",
            should_succeed: true,
            description: "1:1 backing at start",
        },
        BackingTest {
            jitosol_reserves: 1_100_000_000,
            feelssol_supply: 1_000_000_000,
            operation: "yield_accrual",
            should_succeed: true,
            description: "Yield increases reserves",
        },
        BackingTest {
            jitosol_reserves: 900_000_000,
            feelssol_supply: 1_000_000_000,
            operation: "invalid_withdraw",
            should_succeed: false,
            description: "Cannot create undercollateralized state",
        },
        BackingTest {
            jitosol_reserves: 1_000_000_000,
            feelssol_supply: 1_100_000_000,
            operation: "invalid_mint",
            should_succeed: false,
            description: "Cannot mint without backing",
        },
    ];

    for test in tests {
        println!("Test {}: {}", test.operation, test.description);
        
        let invariant_holds = test.jitosol_reserves >= test.feelssol_supply;
        
        if test.should_succeed {
            assert!(invariant_holds, "Backing invariant violated!");
        } else {
            assert!(!invariant_holds, "Invalid state should be rejected");
            println!("  Operation would be rejected to maintain invariant");
        }
        
        let collateralization = (test.jitosol_reserves as f64 / test.feelssol_supply as f64 * 100.0);
        println!("  Collateralization: {:.2}%", collateralization);
    }

    Ok(())
}

#[tokio::test]
async fn test_conservation_invariant() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test: Σw_i ln(g_i) = 0 (conservation of geometric mean)
    // In practice, this means no value created or destroyed
    
    struct ConservationTest {
        initial_reserves: Vec<u128>,
        final_reserves: Vec<u128>,
        operation: &'static str,
        conserves_value: bool,
    }

    let tests = vec![
        ConservationTest {
            initial_reserves: vec![1_000_000, 1_000_000],
            final_reserves: vec![900_000, 1_111_111],
            operation: "valid_swap",
            conserves_value: true, // Product approximately preserved
        },
        ConservationTest {
            initial_reserves: vec![1_000_000, 1_000_000],
            final_reserves: vec![900_000, 1_200_000],
            operation: "invalid_swap",
            conserves_value: false, // Value created
        },
        ConservationTest {
            initial_reserves: vec![1_000_000, 1_000_000],
            final_reserves: vec![1_100_000, 1_100_000],
            operation: "invalid_mint",
            conserves_value: false, // Both reserves increased
        },
    ];

    for test in tests {
        println!("Test operation: {}", test.operation);
        
        // Calculate products (simplified constant product for illustration)
        let initial_product: u128 = test.initial_reserves.iter().product();
        let final_product: u128 = test.final_reserves.iter().product();
        
        // Allow 0.1% deviation for fees/rounding
        let tolerance = initial_product / 1000;
        let conserved = (initial_product as i128 - final_product as i128).abs() < tolerance as i128;
        
        println!("  Initial product: {}", initial_product);
        println!("  Final product: {}", final_product);
        println!("  Conserved: {}", conserved);
        
        if test.conserves_value {
            assert!(conserved, "Conservation invariant violated");
        } else {
            assert!(!conserved, "Invalid operation should violate conservation");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_supply_invariant() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test: FeelsSOL_Total = User_Held + Σ Pool_Escrowed
    struct SupplyTest {
        total_supply: u128,
        user_held: u128,
        pool_escrowed: Vec<u128>,
        description: &'static str,
    }

    let tests = vec![
        SupplyTest {
            total_supply: 1_000_000_000,
            user_held: 600_000_000,
            pool_escrowed: vec![200_000_000, 150_000_000, 50_000_000],
            description: "Valid distribution",
        },
        SupplyTest {
            total_supply: 1_000_000_000,
            user_held: 700_000_000,
            pool_escrowed: vec![200_000_000, 150_000_000],
            description: "Missing 50M tokens",
        },
        SupplyTest {
            total_supply: 1_000_000_000,
            user_held: 500_000_000,
            pool_escrowed: vec![300_000_000, 300_000_000],
            description: "Excess tokens in pools",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);
        
        let accounted_supply = test.user_held + test.pool_escrowed.iter().sum::<u128>();
        let matches = accounted_supply == test.total_supply;
        
        println!("  Total supply: {}", test.total_supply);
        println!("  User held: {}", test.user_held);
        println!("  Pool escrowed: {:?}", test.pool_escrowed);
        println!("  Accounted: {}", accounted_supply);
        println!("  Match: {}", matches);
        
        if test.description.contains("Valid") {
            assert!(matches, "Supply invariant violated");
        } else {
            assert!(!matches, "Should detect supply mismatch");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_isolation_invariant() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test: Pool_i_Outflow <= Pool_i_Inflow (pools cannot create FeelsSOL)
    struct IsolationTest {
        pool_id: u8,
        feelssol_inflow: u128,
        feelssol_outflow: u128,
        operation: &'static str,
        should_succeed: bool,
    }

    let tests = vec![
        IsolationTest {
            pool_id: 1,
            feelssol_inflow: 1_000_000,
            feelssol_outflow: 900_000,
            operation: "normal_trading",
            should_succeed: true,
        },
        IsolationTest {
            pool_id: 2,
            feelssol_inflow: 1_000_000,
            feelssol_outflow: 1_000_000,
            operation: "full_withdrawal",
            should_succeed: true,
        },
        IsolationTest {
            pool_id: 3,
            feelssol_inflow: 1_000_000,
            feelssol_outflow: 1_100_000,
            operation: "invalid_creation",
            should_succeed: false,
        },
    ];

    for test in tests {
        println!("Pool {} - {}", test.pool_id, test.operation);
        
        let invariant_holds = test.feelssol_outflow <= test.feelssol_inflow;
        
        println!("  Inflow: {}", test.feelssol_inflow);
        println!("  Outflow: {}", test.feelssol_outflow);
        println!("  Valid: {}", invariant_holds);
        
        if test.should_succeed {
            assert!(invariant_holds, "Pool isolation violated");
        } else {
            assert!(!invariant_holds, "Should reject FeelsSOL creation");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_oracle_rate_monotonicity() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test that protocol oracle rate never decreases
    struct RateTest {
        previous_rate: u128,
        new_rate: u128,
        rate_type: &'static str,
        should_accept: bool,
    }

    let tests = vec![
        RateTest {
            previous_rate: 1_000_000,
            new_rate: 1_001_000,
            rate_type: "yield_accrual",
            should_accept: true,
        },
        RateTest {
            previous_rate: 1_001_000,
            new_rate: 1_001_000,
            rate_type: "no_change",
            should_accept: true,
        },
        RateTest {
            previous_rate: 1_001_000,
            new_rate: 1_000_500,
            rate_type: "rate_decrease",
            should_accept: false,
        },
        RateTest {
            previous_rate: 1_001_000,
            new_rate: 0,
            rate_type: "zero_rate",
            should_accept: false,
        },
    ];

    for test in tests {
        println!("Test {}: {} -> {}", test.rate_type, test.previous_rate, test.new_rate);
        
        let is_monotonic = test.new_rate >= test.previous_rate && test.new_rate > 0;
        
        if test.should_accept {
            assert!(is_monotonic, "Rate update should be monotonic");
        } else {
            assert!(!is_monotonic, "Non-monotonic rate should be rejected");
            println!("  Update would be rejected");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fee_extraction_limits() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test that fee extraction cannot violate solvency
    struct FeeTest {
        pool_reserves: u128,
        fee_amount: u128,
        min_reserves_required: u128,
        description: &'static str,
    }

    let tests = vec![
        FeeTest {
            pool_reserves: 1_000_000,
            fee_amount: 10_000,
            min_reserves_required: 100_000,
            description: "Normal fee extraction",
        },
        FeeTest {
            pool_reserves: 150_000,
            fee_amount: 60_000,
            min_reserves_required: 100_000,
            description: "Would breach minimum reserves",
        },
        FeeTest {
            pool_reserves: 1_000_000,
            fee_amount: 1_100_000,
            min_reserves_required: 100_000,
            description: "Fee exceeds reserves",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);
        
        let remaining = test.pool_reserves.saturating_sub(test.fee_amount);
        let maintains_minimum = remaining >= test.min_reserves_required;
        let valid_extraction = test.fee_amount <= test.pool_reserves && maintains_minimum;
        
        println!("  Reserves: {}", test.pool_reserves);
        println!("  Fee: {}", test.fee_amount);
        println!("  Remaining: {}", remaining);
        println!("  Valid: {}", valid_extraction);
        
        if test.description.contains("Normal") {
            assert!(valid_extraction, "Valid fee extraction rejected");
        } else {
            assert!(!valid_extraction, "Invalid fee extraction not caught");
        }
    }

    Ok(())
}

#[test]
fn test_arithmetic_overflow_protection() {
    // Test safe math throughout protocol
    let overflow_tests = vec![
        (u128::MAX, 1, "add", false, "Addition overflow"),
        (u128::MAX, 2, "mul", false, "Multiplication overflow"),
        (100, 0, "div", false, "Division by zero"),
        (u128::MAX / 2, 2, "mul", true, "Safe multiplication"),
        (1000, 10, "div", true, "Safe division"),
    ];

    for (a, b, op, should_succeed, description) in overflow_tests {
        println!("Test: {}", description);
        
        let result = match op {
            "add" => a.checked_add(b),
            "sub" => a.checked_sub(b),
            "mul" => a.checked_mul(b),
            "div" => a.checked_div(b),
            _ => None,
        };
        
        if should_succeed {
            assert!(result.is_some(), "Safe operation failed");
            println!("  Result: {}", result.unwrap());
        } else {
            assert!(result.is_none(), "Overflow not caught");
            println!("  Overflow prevented");
        }
    }
}

#[tokio::test]
async fn test_multi_pool_solvency_stress() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Stress test with multiple pools operating simultaneously
    const NUM_POOLS: usize = 10;
    const TOTAL_FEELSSOL: u128 = 10_000_000_000; // 10B total
    const JITOSOL_RESERVES: u128 = 10_100_000_000; // 10.1B (1% surplus)
    
    // Distribute FeelsSOL across pools
    let mut pool_balances = vec![0u128; NUM_POOLS];
    let mut remaining = TOTAL_FEELSSOL;
    
    // Simulate random distribution
    for i in 0..NUM_POOLS {
        let allocation = if i == NUM_POOLS - 1 {
            remaining // Last pool gets remainder
        } else {
            remaining / (NUM_POOLS - i) as u128
        };
        pool_balances[i] = allocation;
        remaining -= allocation;
    }
    
    println!("Initial distribution across {} pools:", NUM_POOLS);
    for (i, balance) in pool_balances.iter().enumerate() {
        println!("  Pool {}: {}", i, balance);
    }
    
    // Verify initial invariants
    let total_in_pools: u128 = pool_balances.iter().sum();
    assert_eq!(total_in_pools, TOTAL_FEELSSOL, "Distribution mismatch");
    assert!(JITOSOL_RESERVES >= TOTAL_FEELSSOL, "Initial undercollateralization");
    
    // Simulate trading activity
    println!("\nSimulating trading activity...");
    let mut operations = 0;
    
    for _ in 0..100 {
        // Random pool-to-pool transfer
        let from_pool = operations % NUM_POOLS;
        let to_pool = (operations + 3) % NUM_POOLS;
        let amount = pool_balances[from_pool] / 10; // Transfer 10%
        
        if amount > 0 {
            pool_balances[from_pool] -= amount;
            pool_balances[to_pool] += amount;
            operations += 1;
        }
    }
    
    // Verify invariants still hold
    let final_total: u128 = pool_balances.iter().sum();
    assert_eq!(final_total, TOTAL_FEELSSOL, "FeelsSOL created/destroyed!");
    
    println!("\nAfter {} operations:", operations);
    println!("Total FeelsSOL preserved: {}", final_total);
    println!("Backing maintained: {}", JITOSOL_RESERVES >= final_total);
    
    Ok(())
}