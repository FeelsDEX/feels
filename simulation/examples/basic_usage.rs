/// Basic usage example for the Feels Protocol simulation framework
///
/// This example demonstrates:
/// - Creating a simulation environment
/// - Configuring simulation parameters
/// - Running basic protocol operations
/// - Understanding price/tick conversions
/// - Demonstrating typical AMM scenarios
use feels_simulation::{FeelsSimulation, SimulationConfig, SimulationResult};

fn main() -> SimulationResult<()> {
    println!("Feels Protocol Simulation - Basic Usage");
    println!("======================================\n");

    // Step 1: Create simulation configuration
    println!("1. Creating simulation configuration...");
    let config = SimulationConfig::localnet();
    println!("   - Program ID: {}", config.program_id);
    println!("   - Cluster: {}\n", config.cluster);

    // Step 2: Initialize simulation environment
    println!("2. Initializing simulation environment...");
    let simulation = FeelsSimulation::new(config);

    // Step 3: Run basic connectivity test
    println!("3. Running basic connectivity test...");
    simulation.run_basic_test()?;
    println!();

    // Step 4: Demonstrate price and tick conversions
    demonstrate_price_conversions();

    // Step 5: Show typical fee tier configurations
    demonstrate_fee_tiers();

    // Step 6: Explain liquidity range concepts
    demonstrate_liquidity_ranges();

    // Step 7: Show typical simulation scenarios
    demonstrate_simulation_scenarios();

    println!("\nBasic usage example completed successfully!");
    println!("For more advanced examples, see other files in the examples directory.");

    Ok(())
}

/// Demonstrates price to tick conversions and the tick system
fn demonstrate_price_conversions() {
    println!("4. Price and Tick System:");
    println!("   The protocol uses a tick-based pricing system where:");
    println!("   - price = 1.0001^tick");
    println!("   - tick = floor(log(price) / log(1.0001))");
    println!("   - sqrt_price_x96 = sqrt(price) * 2^96\n");

    println!("   Common price points and their ticks:");
    let prices: [(f64, &str); 9] = [
        (0.01, "1 cent"),
        (0.1, "10 cents"),
        (0.5, "50 cents"),
        (1.0, "$1 (price parity)"),
        (2.0, "$2"),
        (10.0, "$10"),
        (100.0, "$100"),
        (1000.0, "$1,000"),
        (10000.0, "$10,000"),
    ];

    for (price, description) in prices {
        let tick = (price.ln() / 1.0001_f64.ln()).round() as i32;
        let sqrt_price = price.sqrt();
        let sqrt_price_x96 = (sqrt_price * (1u128 << 96) as f64) as u128;
        println!(
            "   Price: {:>10.2} ({:>15}) -> Tick: {:>7}, sqrt_price_x96: {}",
            price, description, tick, sqrt_price_x96
        );
    }
    println!();
}

/// Demonstrates the fee tier system
fn demonstrate_fee_tiers() {
    println!("5. Fee Tier System:");
    println!("   The protocol supports multiple fee tiers with different tick spacings:\n");

    let fee_tiers: [(u16, u16, &str); 4] = [
        (1, 1, "Ultra-low fee for stable pairs"),
        (5, 10, "Low fee for correlated assets"),
        (30, 60, "Standard fee for most pairs"),
        (100, 200, "High fee for volatile pairs"),
    ];

    for (fee_bps, tick_spacing, description) in fee_tiers {
        println!(
            "   - {:>3} bps ({:>5.2}%): tick spacing = {:>3} - {}",
            fee_bps,
            fee_bps as f64 / 100.0,
            tick_spacing,
            description
        );
    }

    println!("\n   Tick spacing determines the granularity of liquidity positions:");
    println!("   - Smaller spacing = more precise positioning");
    println!("   - Larger spacing = more gas efficient\n");
}

/// Demonstrates liquidity range concepts
fn demonstrate_liquidity_ranges() {
    println!("6. Liquidity Range Examples:");
    println!("   Liquidity providers can concentrate their capital in specific price ranges:\n");

    let current_price: f64 = 2000.0; // Example: FEELS/FeelsSOL at $2000
    let current_tick = (current_price.ln() / 1.0001_f64.ln()).round() as i32;

    println!(
        "   Current price: ${:.2} (tick: {})",
        current_price, current_tick
    );
    println!("   Example liquidity positions:\n");

    let positions: [(f64, f64, &str, &str); 4] = [
        (0.5, 2.0, "±50% range", "Balanced risk/reward"),
        (
            0.9,
            1.1,
            "±10% range",
            "Tight range, higher capital efficiency",
        ),
        (0.99, 1.01, "±1% range", "Very tight, maximum efficiency"),
        (0.25, 4.0, "Wide range", "Similar to traditional AMM"),
    ];

    for (lower_mult, upper_mult, name, description) in positions {
        let lower_price = current_price * lower_mult;
        let upper_price = current_price * upper_mult;
        let lower_tick = (lower_price.ln() / 1.0001_f64.ln()).round() as i32;
        let upper_tick = (upper_price.ln() / 1.0001_f64.ln()).round() as i32;

        // Round to nearest tick spacing (assuming 60 for standard fee tier)
        let tick_spacing = 60;
        let lower_tick_aligned = (lower_tick / tick_spacing) * tick_spacing;
        let upper_tick_aligned = ((upper_tick / tick_spacing) + 1) * tick_spacing;

        println!(
            "   - {:<15} ${:>8.2} - ${:>8.2} (ticks: {} to {})",
            name, lower_price, upper_price, lower_tick_aligned, upper_tick_aligned
        );
        println!("     {}", description);
    }
    println!();
}

/// Demonstrates typical simulation scenarios
fn demonstrate_simulation_scenarios() {
    println!("7. Typical Simulation Scenarios:");
    println!("   The simulation framework can be used to test various scenarios:\n");

    println!("   a) Pool Creation and Initialization:");
    println!("      - Create FeelsSOL/USDC pool with 0.3% fee");
    println!("      - Initialize at price $1.00");
    println!("      - Add initial liquidity\n");

    println!("   b) Liquidity Management:");
    println!("      - Add concentrated liquidity positions");
    println!("      - Remove liquidity and collect fees");
    println!("      - Rebalance positions based on price movement\n");

    println!("   c) Swap Execution:");
    println!("      - Execute small swaps within single tick");
    println!("      - Large swaps crossing multiple ticks");
    println!("      - Test slippage protection\n");

    println!("   d) Fee Collection:");
    println!("      - Track fee accumulation");
    println!("      - Collect LP fees");
    println!("      - Protocol fee collection\n");

    println!("   e) Edge Cases:");
    println!("      - Swaps at price boundaries");
    println!("      - Zero liquidity scenarios");
    println!("      - Maximum tick movements\n");

    println!("   Note: Full simulation requires solana-program-test dependency");
    println!("   which is currently disabled due to edition2024 compatibility.");
}
