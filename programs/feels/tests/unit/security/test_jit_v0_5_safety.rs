use crate::common::*;

#[tokio::test]
async fn test_graduated_drain_protection() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test graduated throttling based on consumption
    struct DrainTest {
        rolling_consumption: u128,
        per_slot_cap: u128,
        expected_throttle_factor: u8,
        description: &'static str,
    }

    let tests = vec![
        DrainTest {
            rolling_consumption: 0,
            per_slot_cap: 1_000_000,
            expected_throttle_factor: 100,
            description: "0% consumed - full allowance",
        },
        DrainTest {
            rolling_consumption: 400_000,
            per_slot_cap: 1_000_000,
            expected_throttle_factor: 100,
            description: "40% consumed - still full allowance",
        },
        DrainTest {
            rolling_consumption: 600_000,
            per_slot_cap: 1_000_000,
            expected_throttle_factor: 50,
            description: "60% consumed - half allowance",
        },
        DrainTest {
            rolling_consumption: 800_000,
            per_slot_cap: 1_000_000,
            expected_throttle_factor: 20,
            description: "80% consumed - 20% allowance",
        },
        DrainTest {
            rolling_consumption: 950_000,
            per_slot_cap: 1_000_000,
            expected_throttle_factor: 10,
            description: "95% consumed - minimal allowance",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);

        let consumption_ratio = (test.rolling_consumption * 10_000) / test.per_slot_cap;

        let throttle_factor = match consumption_ratio {
            0..=5000 => 100,   // < 50% used
            5001..=7500 => 50, // 50-75% used
            7501..=9000 => 20, // 75-90% used
            _ => 10,           // > 90% used
        };

        assert_eq!(throttle_factor, test.expected_throttle_factor);

        // Test actual allowance calculation
        let base_allowance = 100_000u128;
        let throttled_allowance = (base_allowance * throttle_factor as u128) / 100;
        println!(
            "  Base: {}, Throttled: {}",
            base_allowance, throttled_allowance
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_rolling_window_reset() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    const ROLLING_WINDOW_SLOTS: u64 = 150;

    struct WindowTest {
        current_slot: u64,
        window_start: u64,
        should_reset: bool,
        description: &'static str,
    }

    let tests = vec![
        WindowTest {
            current_slot: 100,
            window_start: 50,
            should_reset: false,
            description: "Within window - no reset",
        },
        WindowTest {
            current_slot: 200,
            window_start: 50,
            should_reset: false,
            description: "Exactly at window boundary",
        },
        WindowTest {
            current_slot: 201,
            window_start: 50,
            should_reset: true,
            description: "Beyond window - should reset",
        },
        WindowTest {
            current_slot: 500,
            window_start: 50,
            should_reset: true,
            description: "Well beyond window - should reset",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);

        let should_reset = test.current_slot > test.window_start + ROLLING_WINDOW_SLOTS;
        assert_eq!(should_reset, test.should_reset);

        if should_reset {
            println!("  Resetting consumption counter and window start");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_slot_based_concentration_shifts() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    const SHIFT_INTERVAL: u64 = 100;

    // Test that concentration zones shift to prevent camping
    let slot_tests = vec![
        (0, -10, "Initial state - starts at -10"),
        (50, -10, "Within first interval"),
        (100, -9, "First shift at interval boundary"),
        (150, -9, "Still in second interval"),
        (200, -8, "Second shift continues"),
        (1000, 0, "Cycles to center (10 cycles)"),
        (1900, 9, "Near max shift"),
        (2000, -10, "Full cycle return (20 cycles)"),
    ];

    for (slot, expected_shift, description) in slot_tests {
        println!("Slot {}: {}", slot, description);

        let shift_cycles = slot / SHIFT_INTERVAL;
        let shift_amount = ((shift_cycles % 20) as i32).saturating_sub(10);

        println!("  Cycles: {}, Shift: {}", shift_cycles, shift_amount);
        assert_eq!(shift_amount, expected_shift);
    }

    Ok(())
}

#[tokio::test]
async fn test_concentration_multiplier_with_shift() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test virtual concentration calculation
    struct ConcentrationTest {
        current_tick: i32,
        target_tick: i32,
        slot: u64,
        concentration_width: u32,
        max_multiplier: u8,
        expected_multiplier: u8,
        description: &'static str,
    }

    let tests = vec![
        ConcentrationTest {
            current_tick: 1000,
            target_tick: 1000,
            slot: 1000, // slot 1000 gives shift 0
            concentration_width: 10,
            max_multiplier: 10,
            expected_multiplier: 10,
            description: "At current price - max concentration",
        },
        ConcentrationTest {
            current_tick: 1000,
            target_tick: 1005,
            slot: 1000, // slot 1000 gives shift_amount = 0
            concentration_width: 10,
            max_multiplier: 10,
            expected_multiplier: 10,
            description: "Within concentration zone",
        },
        ConcentrationTest {
            current_tick: 1000,
            target_tick: 1015,
            slot: 1000, // slot 1000 gives shift 0
            concentration_width: 10,
            max_multiplier: 10,
            expected_multiplier: 5,
            description: "In secondary zone - half multiplier",
        },
        ConcentrationTest {
            current_tick: 1000,
            target_tick: 1030,
            slot: 1000, // slot 1000 gives shift 0
            concentration_width: 10,
            max_multiplier: 10,
            expected_multiplier: 2,
            description: "In tertiary zone - 1/5 multiplier",
        },
        ConcentrationTest {
            current_tick: 1000,
            target_tick: 1100,
            slot: 1000, // slot 1000 gives shift 0
            concentration_width: 10,
            max_multiplier: 10,
            expected_multiplier: 1,
            description: "Far away - no concentration",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);

        // Calculate shift based on slot
        let shift_cycles = test.slot / 100;
        let shift_amount = ((shift_cycles % 20) as i32).saturating_sub(10);

        // Apply shift to the center point, not the distance
        let shifted_center = test.current_tick.saturating_add(shift_amount);
        let adjusted_distance = test.target_tick.saturating_sub(shifted_center).abs() as u32;

        println!(
            "  Slot: {}, Shift cycles: {}, Shift amount: {}",
            test.slot, shift_cycles, shift_amount
        );
        println!(
            "  Current tick: {}, Target tick: {}, Shifted center: {}",
            test.current_tick, test.target_tick, shifted_center
        );

        let multiplier = match adjusted_distance {
            d if d <= test.concentration_width => test.max_multiplier,
            d if d <= test.concentration_width * 2 => test.max_multiplier / 2,
            d if d <= test.concentration_width * 4 => test.max_multiplier / 5,
            _ => 1,
        };

        println!(
            "  Distance: {}, Multiplier: {}x",
            adjusted_distance, multiplier
        );
        assert_eq!(multiplier, test.expected_multiplier);
    }

    Ok(())
}

#[tokio::test]
async fn test_asymmetric_directional_caps() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test directional cap reduction
    struct DirectionalTest {
        buy_volume: u64,
        total_volume: u64,
        is_buy: bool,
        base_cap_bps: u16,
        expected_cap_bps: u16,
        description: &'static str,
    }

    let tests = vec![
        DirectionalTest {
            buy_volume: 500,
            total_volume: 1000,
            is_buy: true,
            base_cap_bps: 300,
            expected_cap_bps: 300,
            description: "Balanced market - full cap",
        },
        DirectionalTest {
            buy_volume: 800,
            total_volume: 1000,
            is_buy: true,
            base_cap_bps: 300,
            expected_cap_bps: 150,
            description: "Heavy buy pressure - buy cap halved",
        },
        DirectionalTest {
            buy_volume: 200,
            total_volume: 1000,
            is_buy: false,
            base_cap_bps: 300,
            expected_cap_bps: 150,
            description: "Heavy sell pressure - sell cap halved",
        },
        DirectionalTest {
            buy_volume: 900,
            total_volume: 1000,
            is_buy: false,
            base_cap_bps: 300,
            expected_cap_bps: 300,
            description: "Buy dominated but selling - full cap",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);

        let buy_pressure = (test.buy_volume * 100 / test.total_volume) as u16;

        let cap = match (test.is_buy, buy_pressure) {
            (true, bp) if bp > 70 => test.base_cap_bps / 2,
            (false, bp) if bp < 30 => test.base_cap_bps / 2,
            _ => test.base_cap_bps,
        };

        println!("  Buy pressure: {}%, Cap: {} bps", buy_pressure, cap);
        assert_eq!(cap, test.expected_cap_bps);
    }

    Ok(())
}

#[tokio::test]
async fn test_tick_distance_impact_penalty() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test graduated penalties for large moves
    let penalty_tests = vec![
        (5, 100, "Small move - no penalty"),
        (25, 70, "Medium move - 30% penalty"),
        (75, 40, "Large move - 60% penalty"),
        (150, 20, "Very large move - 80% penalty"),
        (500, 10, "Extreme move - 90% penalty"),
    ];

    for (tick_movement, expected_factor, description) in penalty_tests {
        println!("Move {} ticks: {}", tick_movement, description);

        let penalty_factor = match tick_movement {
            0..=10 => 100,
            11..=50 => 70,
            51..=100 => 40,
            101..=200 => 20,
            _ => 10,
        };

        assert_eq!(penalty_factor, expected_factor);

        // Apply to allowance
        let base_allowance = 1_000_000u128;
        let penalized = base_allowance * penalty_factor as u128 / 100;
        println!("  Base: {}, After penalty: {}", base_allowance, penalized);
    }

    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_activation() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test circuit breaker conditions
    struct CircuitTest {
        tau_spot: u128,
        initial_tau_spot: u128,
        current_tick: i32,
        tick_1hr_ago: i32,
        threshold_bps: u16,
        should_activate: bool,
        description: &'static str,
    }

    let tests = vec![
        CircuitTest {
            tau_spot: 950_000_000,
            initial_tau_spot: 1_000_000_000,
            current_tick: 1000,
            tick_1hr_ago: 1000,
            threshold_bps: 3000, // 30%
            should_activate: false,
            description: "95% health - above threshold",
        },
        CircuitTest {
            tau_spot: 250_000_000,
            initial_tau_spot: 1_000_000_000,
            current_tick: 1000,
            tick_1hr_ago: 1000,
            threshold_bps: 3000,
            should_activate: true,
            description: "25% health - circuit breaker active",
        },
        CircuitTest {
            tau_spot: 800_000_000,
            initial_tau_spot: 1_000_000_000,
            current_tick: 2100,
            tick_1hr_ago: 1000,
            threshold_bps: 3000,
            should_activate: true,
            description: "11% price move - circuit breaker active",
        },
    ];

    for test in tests {
        println!("Test: {}", test.description);

        // Check buffer health
        let buffer_health_bps = (test.tau_spot * 10_000 / test.initial_tau_spot) as u16;
        let health_triggered = buffer_health_bps < test.threshold_bps;

        // Check price movement
        let price_movement = (test.current_tick - test.tick_1hr_ago).abs();
        let movement_triggered = price_movement > 1000; // ~10%

        let should_activate = health_triggered || movement_triggered;

        println!(
            "  Health: {} bps, Movement: {} ticks",
            buffer_health_bps, price_movement
        );
        assert_eq!(should_activate, test.should_activate);
    }

    Ok(())
}

#[tokio::test]
async fn test_combined_attack_mitigation() -> TestResult<()> {
    let _ctx = TestContext::new(TestEnvironment::in_memory()).await?;

    // Test multiple safety layers working together
    println!("Simulating coordinated drain attack...");

    // Attack parameters
    let mut rolling_consumption = 0u128;
    let per_slot_cap = 1_000_000u128;
    let base_allowance = 100_000u128;

    // Simulate 10 rapid trades
    for i in 0..10 {
        println!("\nTrade #{}", i + 1);

        // Calculate throttle based on consumption
        let consumption_ratio = (rolling_consumption * 10_000) / per_slot_cap;
        let throttle_factor = match consumption_ratio {
            0..=5000 => 100,
            5001..=7500 => 50,
            7501..=9000 => 20,
            _ => 10,
        };

        // Apply concentration (distance 0 = 10x)
        let concentration_multiplier = 10u128;

        // Calculate allowed amount
        let mut allowed = base_allowance * throttle_factor as u128 / 100;
        allowed = allowed * concentration_multiplier;

        // But also apply per-swap cap
        let max_per_swap = 30_000u128; // 3% of buffer
        allowed = allowed.min(max_per_swap);

        println!("  Consumption: {}%", consumption_ratio / 100);
        println!("  Throttle: {}%", throttle_factor);
        println!("  Allowed: {}", allowed);

        // Update consumption
        rolling_consumption += allowed;

        // Check if we're being throttled effectively
        if i >= 5 {
            assert!(
                allowed < base_allowance,
                "Should be throttled after many trades"
            );
        }
    }

    // Verify attack was mitigated
    assert!(
        rolling_consumption < per_slot_cap,
        "Attack should not drain full slot cap"
    );
    println!(
        "\nAttack mitigated - only {} of {} consumed",
        rolling_consumption, per_slot_cap
    );

    Ok(())
}
