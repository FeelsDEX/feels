/// Test module organization
/// Tests are organized into categories based on their focus and complexity
// Unit tests - low-level component testing
#[cfg(test)]
pub mod unit;

// Integration tests
#[cfg(test)]
pub mod integration {
    // Tests that don't require SDK
    pub mod protocol_lifecycle;
    pub mod token_validation;

    // Tests using SDK - disabled until dependencies are available
}

// Functional tests
#[cfg(test)]
pub mod functional {
    // Tests that don't require SDK
    pub mod amm_operations;

    // Tests using SDK - disabled until dependencies are available
}

// Complex scenario tests - disabled until test dependencies are available

// Property-based tests
#[cfg(test)]
pub mod property_tests;
