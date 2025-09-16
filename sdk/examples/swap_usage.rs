//! Example usage of the Feels SDK SwapBuilder
//!
//! Demonstrates ergonomic swap construction with automatic tick array management

use anchor_lang::prelude::*;
use feels_sdk::{SwapBuilder, SwapDirection, SwapParams, TestAccountBuilder, TestCoverage};
use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    // Example 1: Basic swap with automatic tick arrays
    let basic_swap_example = || -> std::result::Result<(), Box<dyn Error>> {
        println!("=== Basic Swap Example ===");

        let user = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let params = SwapParams {
            market,
            oracle: Pubkey::default(), // No oracle
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            vault_authority: Pubkey::new_unique(),
            buffer: Pubkey::new_unique(),
            user_token_in: Pubkey::new_unique(),
            user_token_out: Pubkey::new_unique(),
            protocol_config: Pubkey::new_unique(),
            protocol_treasury: Pubkey::new_unique(),
            protocol_token: None,
            creator_token_account: None,
            amount_in: 1_000_000,        // 1 token
            minimum_amount_out: 950_000, // 5% slippage
            max_ticks_crossed: 0,        // No limit
            max_fee_bps: 0,              // No fee cap
        };

        let swap_instruction = SwapBuilder::new(params)
            .with_tick_context(-1000, 64) // Current tick and spacing
            .with_auto_arrays(SwapDirection::ZeroForOne, 50) // Auto-derive arrays
            .unwrap()
            .build(&user)?;

        println!(
            "Built swap instruction with {} accounts",
            swap_instruction.accounts.len()
        );
        println!("Program ID: {}", swap_instruction.program_id);

        Ok(())
    };

    // Example 2: Manual tick array specification
    let manual_arrays_example = || -> std::result::Result<(), Box<dyn Error>> {
        println!("\n=== Manual Arrays Example ===");

        let user = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let params = SwapParams {
            market,
            oracle: Pubkey::default(),
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            vault_authority: Pubkey::new_unique(),
            buffer: Pubkey::new_unique(),
            user_token_in: Pubkey::new_unique(),
            user_token_out: Pubkey::new_unique(),
            protocol_config: Pubkey::new_unique(),
            protocol_treasury: Pubkey::new_unique(),
            protocol_token: None,
            creator_token_account: None,
            amount_in: 5_000_000,
            minimum_amount_out: 4_800_000,
            max_ticks_crossed: 100,
            max_fee_bps: 0,
        };

        // Manually specify tick arrays for precise control
        let tick_arrays = vec![
            Pubkey::new_unique(), // Array covering current range
            Pubkey::new_unique(), // Array covering next range down
            Pubkey::new_unique(), // Array covering next range up
        ];

        let _swap_instruction = SwapBuilder::new(params)
            .with_tick_arrays(tick_arrays)
            .build(&user)?;

        println!("Built swap with manually specified arrays");

        Ok(())
    };

    // Example 3: Range-based array derivation
    let range_based_example = || -> std::result::Result<(), Box<dyn Error>> {
        println!("\n=== Range-Based Arrays Example ===");

        let user = Pubkey::new_unique();
        let market = Pubkey::new_unique();

        let params = SwapParams {
            market,
            oracle: Pubkey::new_unique(), // With oracle
            vault_0: Pubkey::new_unique(),
            vault_1: Pubkey::new_unique(),
            vault_authority: Pubkey::new_unique(),
            buffer: Pubkey::new_unique(),
            user_token_in: Pubkey::new_unique(),
            user_token_out: Pubkey::new_unique(),
            protocol_config: Pubkey::new_unique(),
            protocol_treasury: Pubkey::new_unique(),
            protocol_token: None,
            creator_token_account: None,
            amount_in: 2_000_000,
            minimum_amount_out: 1_950_000,
            max_ticks_crossed: 200,
            max_fee_bps: 0,
        };

        let _swap_instruction = SwapBuilder::new(params)
            .with_tick_range(-5000, 5000, 128)? // Cover wide range with 128 spacing
            .build(&user)?;

        println!("Built swap with range-derived arrays");

        Ok(())
    };

    // Example 4: Test coverage generation
    let test_coverage_example = || -> std::result::Result<(), Box<dyn Error>> {
        println!("\n=== Test Coverage Example ===");

        let market = Pubkey::new_unique();
        let coverage = TestCoverage::new(market, 64, -100, 1u128 << 64);

        // Generate comprehensive test arrays
        let test_arrays = coverage.generate_comprehensive_arrays()?;
        println!(
            "Generated {} test arrays for comprehensive coverage",
            test_arrays.len()
        );

        // Generate swap test cases
        let swap_cases = coverage.generate_swap_test_cases();
        println!("Generated {} swap test cases", swap_cases.len());

        // Show some examples
        for (i, case) in swap_cases.iter().take(3).enumerate() {
            println!(
                "  Test {}: {} - {} tokens, {:?}",
                i + 1,
                case.name,
                case.amount_in,
                case.direction
            );
        }

        // Generate position test cases
        let position_cases = coverage.generate_position_test_cases();
        println!("Generated {} position test cases", position_cases.len());

        // Generate stress test scenarios
        let stress_scenarios = coverage.generate_stress_test_scenarios();
        println!("Generated {} stress test scenarios", stress_scenarios.len());

        for scenario in &stress_scenarios {
            println!(
                "  Scenario: {} ({} swaps) - {}",
                scenario.name,
                scenario.swaps.len(),
                scenario.description
            );
        }

        Ok(())
    };

    // Example 5: Test account builder
    let account_builder_example = || -> std::result::Result<(), Box<dyn Error>> {
        println!("\n=== Account Builder Example ===");

        let mut builder = TestAccountBuilder::new();

        // Create test keypairs
        let keypairs = builder.create_keypairs(5);
        println!("Created {} test keypairs", keypairs.len());

        // Create deterministic test mints
        let mints = builder.create_test_mints(3);
        println!("Created {} deterministic test mints", mints.len());

        for (i, mint) in mints.iter().enumerate() {
            println!("  Mint {}: {}", i, mint);
        }

        Ok(())
    };

    // Run all examples
    basic_swap_example()?;
    manual_arrays_example()?;
    range_based_example()?;
    test_coverage_example()?;
    account_builder_example()?;

    println!("\nAll examples completed successfully!");

    Ok(())
}

// Example of a more complete integration showing how to use with actual market data
#[allow(dead_code)]
fn advanced_swap_example() -> std::result::Result<(), Box<dyn Error>> {
    println!("\n=== Advanced Integration Example ===");

    // Simulated market state (in practice, you'd fetch this from chain)
    let market_state = MarketState {
        market: Pubkey::new_unique(),
        current_tick: -887,
        tick_spacing: 64,
        sqrt_price: 79228162514264337593543950336u128, // ~1.0
        liquidity: 1_000_000_000u128,
    };

    let user = Pubkey::new_unique();

    // Build swap with market context
    let params = SwapParams {
        market: market_state.market,
        oracle: Pubkey::default(),
        vault_0: Pubkey::new_unique(),
        vault_1: Pubkey::new_unique(),
        vault_authority: Pubkey::new_unique(),
        buffer: Pubkey::new_unique(),
        user_token_in: Pubkey::new_unique(),
        user_token_out: Pubkey::new_unique(),
        protocol_config: Pubkey::new_unique(),
        protocol_treasury: Pubkey::new_unique(),
        protocol_token: None,
        creator_token_account: None,
        amount_in: 1_000_000,
        minimum_amount_out: 0, // Will calculate slippage
        max_ticks_crossed: 0,
        max_fee_bps: 0,
    };

    // Calculate expected output and slippage
    let direction = SwapDirection::ZeroForOne;
    let estimated_output = estimate_swap_output(
        params.amount_in,
        market_state.sqrt_price,
        market_state.liquidity,
        direction,
    );
    let minimum_with_slippage = (estimated_output as f64 * 0.95) as u64; // 5% slippage

    // Estimate tick movement for array planning
    let estimated_ticks = feels_sdk::swap_builder::estimate_ticks_for_swap(
        params.amount_in,
        market_state.liquidity,
        market_state.sqrt_price,
        direction,
    );

    let mut updated_params = params;
    updated_params.minimum_amount_out = minimum_with_slippage;

    let swap_instruction = SwapBuilder::new(updated_params)
        .with_tick_context(market_state.current_tick, market_state.tick_spacing)
        .with_auto_arrays(direction, estimated_ticks)?
        .build(&user)?;

    println!("Advanced swap built with:");
    println!("  Expected output: {}", estimated_output);
    println!("  Minimum output: {}", minimum_with_slippage);
    println!("  Estimated ticks: {}", estimated_ticks);
    println!(
        "  Arrays provided: {}",
        swap_instruction.accounts.len() - 10
    ); // Subtract base accounts

    Ok(())
}

// Helper struct for example
#[allow(dead_code)]
struct MarketState {
    market: Pubkey,
    current_tick: i32,
    tick_spacing: u16,
    sqrt_price: u128,
    liquidity: u128,
}

// Simplified swap output estimation (for example purposes)
fn estimate_swap_output(
    amount_in: u64,
    _sqrt_price: u128,
    liquidity: u128,
    _direction: SwapDirection,
) -> u64 {
    // Very simplified calculation - real implementation would be more complex
    if liquidity == 0 {
        return 0;
    }

    let price_impact = (amount_in as u128 * 1000) / liquidity;
    let output_ratio = 1000u128.saturating_sub(price_impact);

    ((amount_in as u128 * output_ratio) / 1000) as u64
}
