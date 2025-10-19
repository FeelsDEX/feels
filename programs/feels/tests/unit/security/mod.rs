// Test modules for security

pub mod test_initialization_race_condition;
pub mod test_launch_security;
pub mod test_reentrancy_guard;
pub mod test_tick_array_griefing;
pub mod test_update_floor_validation;

// New critical security tests
pub mod test_floor_monotonicity;
pub mod test_mint_authority_revocation;
pub mod test_safety_controller;
pub mod test_solvency_invariants;

// Consolidated security tests
pub mod test_jit_safety_consolidated;
pub mod test_oracle_security_consolidated;
