/// Unit test module organization
///
/// Core unit tests for protocol components

// Static compile-time assertions
pub mod static_assertions;
pub mod test_static_assertions;

// Buffer tests
pub mod test_buffer_overflow;

// Position safety tests
pub mod test_close_position_safety;

// Dust control tests
pub mod test_dust_control;

// Fee calculation tests
pub mod test_fee_growth;
pub mod test_fee_rounding;

// Security tests
pub mod test_initialization_race_condition_fix;
pub mod test_launch_security;
pub mod test_reentrancy_guard;

// Oracle tests
pub mod test_observation_offsets;
pub mod test_oracle_timestamp_security;

// POMM tests
pub mod test_pomm_saturation;
pub mod test_pomm_security;

// Tick array tests
pub mod test_tick_array_griefing;

// Instruction tests subdirectory
pub mod instructions;

// State tests subdirectory
pub mod state;

// Test infrastructure verification
pub mod test_infrastructure;

// Simple example test
pub mod test_simple_example;