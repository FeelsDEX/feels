/// Unit test module organization
///
/// Kept tests validate core math and leverage primitives that remain relevant
/// in the new architecture. Outdated tests have been removed; see TODOs in
/// their former files to reimplement against the unified API and physics model.

pub mod leverage_system;
pub mod math_operations;
pub mod math_refactor_validation;
pub mod math_tick;
pub mod math_tick_edge_cases;
