/// Unit test module organization
/// 
/// This module organizes unit tests by functional area:
/// - math_operations: Mathematical utilities and arithmetic
/// - instruction_validation: Instruction handlers and account constraints
/// - tick_math: Tick math correctness and safety tests
/// - math_tick_edge_cases: Comprehensive edge case tests for tick math security
pub mod instruction_validation;
pub mod math_operations;
pub mod math_tick;
pub mod math_tick_edge_cases;