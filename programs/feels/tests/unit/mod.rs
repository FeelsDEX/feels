/// Unit test module organization
///
/// Core unit tests for protocol components organized by category

// Category modules
pub mod buffer;
pub mod math;
pub mod oracle;
pub mod pomm;
pub mod position;
pub mod security;

// Instruction tests subdirectory
pub mod instructions;

// Static compile-time assertions
pub mod static_assertions;
pub mod test_static_assertions;

// Simple example test
pub mod test_simple_example;