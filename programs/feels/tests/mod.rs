/// Test module organization
/// Tests are organized into categories based on their focus and complexity

// Common test utilities
#[cfg(test)]
#[macro_use]
pub mod common;

// Unit tests - low-level component testing
#[cfg(test)]
pub mod unit;

// Integration tests
#[cfg(test)]
pub mod integration;

// Property-based tests
#[cfg(test)]
pub mod property;

// End-to-end tests
#[cfg(test)]
pub mod e2e;

