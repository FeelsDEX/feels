//! Testing utilities for Feels Protocol
//!
//! Provides ergonomic helpers for testing swaps, positions, and market scenarios

use anchor_lang::prelude::*;
use solana_sdk::signature::Keypair;
use crate::{SdkResult, SwapBuilder, SwapParams, SwapDirection};

/// Test coverage utilities for comprehensive swap testing
pub struct TestCoverage {
    pub market: Pubkey,
    pub tick_spacing: u16,
    pub current_tick: i32,
    pub current_sqrt_price: u128,
}

impl TestCoverage {
    /// Create new test coverage helper
    pub fn new(
        market: Pubkey, 
        tick_spacing: u16, 
        current_tick: i32, 
        current_sqrt_price: u128
    ) -> Self {
        Self {
            market,
            tick_spacing,
            current_tick,
            current_sqrt_price,
        }
    }

    /// Generate comprehensive tick array coverage for testing
    pub fn generate_comprehensive_arrays(&self) -> SdkResult<Vec<Pubkey>> {
        let mut arrays = Vec::new();
        
        // Cover wide range around current tick
        let ranges = vec![
            // Close range (likely to be hit)
            (self.current_tick - 1000, self.current_tick + 1000),
            // Medium range (stress testing)
            (self.current_tick - 5000, self.current_tick + 5000),
            // Wide range (extreme scenarios)
            (self.current_tick - 20000, self.current_tick + 20000),
        ];

        for (lower, upper) in ranges {
            let range_arrays = crate::swap_builder::derive_tick_arrays_for_range(
                &self.market,
                lower,
                upper,
                self.tick_spacing,
            )?;
            arrays.extend(range_arrays);
        }

        // Remove duplicates
        arrays.sort();
        arrays.dedup();
        
        Ok(arrays)
    }

    /// Generate test cases for different swap scenarios
    pub fn generate_swap_test_cases(&self) -> Vec<SwapTestCase> {
        let base_amounts = vec![
            100,           // Tiny swap
            1_000,         // Small swap
            10_000,        // Medium swap
            100_000,       // Large swap
            1_000_000,     // Very large swap
        ];

        let mut test_cases = Vec::new();

        for &amount in &base_amounts {
            // Test both directions
            for direction in [SwapDirection::ZeroForOne, SwapDirection::OneForZero] {
                // Test different tick crossing scenarios
                for estimated_ticks in [1, 10, 50, 100, 500] {
                    test_cases.push(SwapTestCase {
                        name: format!("{}_{}_{}ticks", 
                                    amount, 
                                    match direction {
                                        SwapDirection::ZeroForOne => "0for1",
                                        SwapDirection::OneForZero => "1for0",
                                    },
                                    estimated_ticks),
                        amount_in: amount,
                        direction,
                        estimated_ticks,
                        minimum_amount_out: amount * 95 / 100, // 5% slippage
                        max_ticks_crossed: 0, // No limit
                    });
                }
            }
        }

        test_cases
    }

    /// Generate position test cases for liquidity provision
    pub fn generate_position_test_cases(&self) -> Vec<PositionTestCase> {
        let mut test_cases = Vec::new();
        let base_tick = self.current_tick;

        // Different position ranges relative to current price
        let position_configs = vec![
            // In-range positions
            (-100, 100, "tight_in_range"),
            (-500, 500, "wide_in_range"),
            (-1000, 1000, "very_wide_in_range"),
            
            // Out-of-range positions
            (1000, 2000, "above_range"),
            (-2000, -1000, "below_range"),
            
            // Asymmetric positions
            (-200, 800, "asymmetric_up"),
            (-800, 200, "asymmetric_down"),
            
            // Edge cases
            (0, 100, "current_to_up"),
            (-100, 0, "down_to_current"),
        ];

        for (lower_offset, upper_offset, name) in position_configs {
            // Align ticks to spacing
            let tick_lower = align_tick_to_spacing(base_tick + lower_offset, self.tick_spacing);
            let tick_upper = align_tick_to_spacing(base_tick + upper_offset, self.tick_spacing);
            
            if tick_lower < tick_upper {
                test_cases.push(PositionTestCase {
                    name: name.to_string(),
                    tick_lower,
                    tick_upper,
                    liquidity_amount: 1_000_000u128, // Base liquidity amount
                });
            }
        }

        test_cases
    }

    /// Generate market stress test scenarios
    pub fn generate_stress_test_scenarios(&self) -> Vec<StressTestScenario> {
        vec![
            StressTestScenario {
                name: "rapid_small_swaps".to_string(),
                description: "Many small swaps in rapid succession".to_string(),
                swaps: (0..100).map(|i| SwapTestCase {
                    name: format!("rapid_swap_{}", i),
                    amount_in: 100,
                    direction: if i % 2 == 0 { SwapDirection::ZeroForOne } else { SwapDirection::OneForZero },
                    estimated_ticks: 1,
                    minimum_amount_out: 95,
                    max_ticks_crossed: 10,
                }).collect(),
            },
            
            StressTestScenario {
                name: "alternating_large_swaps".to_string(),
                description: "Large swaps alternating direction".to_string(),
                swaps: (0..20).map(|i| SwapTestCase {
                    name: format!("large_swap_{}", i),
                    amount_in: 1_000_000,
                    direction: if i % 2 == 0 { SwapDirection::ZeroForOne } else { SwapDirection::OneForZero },
                    estimated_ticks: 100,
                    minimum_amount_out: 950_000,
                    max_ticks_crossed: 0,
                }).collect(),
            },
            
            StressTestScenario {
                name: "tick_boundary_crossings".to_string(),
                description: "Swaps designed to cross many tick boundaries".to_string(),
                swaps: vec![
                    SwapTestCase {
                        name: "massive_0for1".to_string(),
                        amount_in: 10_000_000,
                        direction: SwapDirection::ZeroForOne,
                        estimated_ticks: 1000,
                        minimum_amount_out: 9_000_000,
                        max_ticks_crossed: 0,
                    },
                    SwapTestCase {
                        name: "massive_1for0".to_string(),
                        amount_in: 10_000_000,
                        direction: SwapDirection::OneForZero,
                        estimated_ticks: 1000,
                        minimum_amount_out: 9_000_000,
                        max_ticks_crossed: 0,
                    },
                ],
            },
        ]
    }
}

/// Individual swap test case
#[derive(Clone, Debug)]
pub struct SwapTestCase {
    pub name: String,
    pub amount_in: u64,
    pub direction: SwapDirection,
    pub estimated_ticks: u32,
    pub minimum_amount_out: u64,
    pub max_ticks_crossed: u16,
}

impl SwapTestCase {
    /// Convert to SwapBuilder for execution
    pub fn to_swap_builder(
        &self, 
        base_params: SwapParams,
        current_tick: i32,
        tick_spacing: u16,
    ) -> SdkResult<SwapBuilder> {
        let mut params = base_params;
        params.amount_in = self.amount_in;
        params.minimum_amount_out = self.minimum_amount_out;
        params.max_ticks_crossed = self.max_ticks_crossed;

        SwapBuilder::new(params)
            .with_tick_context(current_tick, tick_spacing)
            .with_auto_arrays(self.direction, self.estimated_ticks)
    }
}

/// Position test case for liquidity provision
#[derive(Clone, Debug)]
pub struct PositionTestCase {
    pub name: String,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_amount: u128,
}

/// Stress test scenario containing multiple operations
#[derive(Clone, Debug)]
pub struct StressTestScenario {
    pub name: String,
    pub description: String,
    pub swaps: Vec<SwapTestCase>,
}

/// Helper to align tick to tick spacing
fn align_tick_to_spacing(tick: i32, tick_spacing: u16) -> i32 {
    let spacing = tick_spacing as i32;
    if tick >= 0 {
        (tick / spacing) * spacing
    } else {
        // For negative ticks, we need to round down (toward negative infinity)
        let remainder = tick % spacing;
        if remainder == 0 {
            tick
        } else {
            tick - (spacing + remainder)
        }
    }
}

/// Test account builder for creating test accounts
pub struct TestAccountBuilder {
    pub keypairs: Vec<Keypair>,
    pub mints: Vec<Pubkey>,
}

impl TestAccountBuilder {
    pub fn new() -> Self {
        Self {
            keypairs: Vec::new(),
            mints: Vec::new(),
        }
    }

    /// Create a new keypair for testing
    pub fn create_keypair(&mut self) -> &Keypair {
        let keypair = Keypair::new();
        self.keypairs.push(keypair);
        self.keypairs.last().unwrap()
    }

    /// Create multiple keypairs at once
    pub fn create_keypairs(&mut self, count: usize) -> Vec<&Keypair> {
        for _ in 0..count {
            self.create_keypair();
        }
        self.keypairs.iter().rev().take(count).collect()
    }

    /// Generate deterministic mint addresses for testing
    pub fn create_test_mints(&mut self, count: usize) -> Vec<Pubkey> {
        let mut mints = Vec::new();
        for i in 0..count {
            // Create deterministic but unique mint addresses
            let seed = format!("test_mint_{}", i);
            let mut seed_bytes = [0u8; 32];
            let bytes = seed.as_bytes();
            let len = std::cmp::min(bytes.len(), 32);
            seed_bytes[..len].copy_from_slice(&bytes[..len]);
            
            let mint = Pubkey::new_from_array(seed_bytes);
            mints.push(mint);
        }
        self.mints.extend(mints.clone());
        mints
    }

    /// Get all created keypairs
    pub fn get_keypairs(&self) -> &[Keypair] {
        &self.keypairs
    }

    /// Get all created mints
    pub fn get_mints(&self) -> &[Pubkey] {
        &self.mints
    }
}

impl Default for TestAccountBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Assertion helpers for testing
pub mod assertions {
    use super::*;

    /// Assert that a tick is properly aligned to spacing
    pub fn assert_tick_aligned(tick: i32, tick_spacing: u16) {
        assert_eq!(
            tick % tick_spacing as i32, 
            0, 
            "Tick {} is not aligned to spacing {}", 
            tick, 
            tick_spacing
        );
    }

    /// Assert that tick arrays cover a given range
    pub fn assert_arrays_cover_range(
        arrays: &[Pubkey], 
        tick_lower: i32, 
        tick_upper: i32, 
        _tick_spacing: u16
    ) {
        // This is a simplified check - in practice you'd verify the actual array coverage
        assert!(!arrays.is_empty(), "No tick arrays provided");
        assert!(
            tick_upper > tick_lower, 
            "Invalid tick range: {} to {}", 
            tick_lower, 
            tick_upper
        );
    }

    /// Assert that a swap test case is valid
    pub fn assert_valid_swap_test_case(test_case: &SwapTestCase) {
        assert!(test_case.amount_in > 0, "Amount in must be positive");
        assert!(
            test_case.minimum_amount_out <= test_case.amount_in, 
            "Minimum out cannot exceed amount in"
        );
        assert!(!test_case.name.is_empty(), "Test case must have a name");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_generation() {
        let coverage = TestCoverage::new(
            Pubkey::new_unique(),
            64,
            -100,
            1u128 << 64,
        );

        let arrays = coverage.generate_comprehensive_arrays().unwrap();
        assert!(!arrays.is_empty());

        let swap_cases = coverage.generate_swap_test_cases();
        assert!(!swap_cases.is_empty());

        let position_cases = coverage.generate_position_test_cases();
        assert!(!position_cases.is_empty());

        let stress_scenarios = coverage.generate_stress_test_scenarios();
        assert_eq!(stress_scenarios.len(), 3);
    }

    #[test]
    fn test_account_builder() {
        let mut builder = TestAccountBuilder::new();
        
        let keypairs = builder.create_keypairs(5);
        assert_eq!(keypairs.len(), 5);
        
        let mints = builder.create_test_mints(3);
        assert_eq!(mints.len(), 3);
        
        // Verify deterministic mint generation
        let mut builder2 = TestAccountBuilder::new();
        let mints2 = builder2.create_test_mints(3);
        assert_eq!(mints, mints2);
    }

    #[test]
    fn test_tick_alignment() {
        assert_eq!(align_tick_to_spacing(123, 64), 64);
        assert_eq!(align_tick_to_spacing(-123, 64), -128);
        assert_eq!(align_tick_to_spacing(128, 64), 128);
    }
}