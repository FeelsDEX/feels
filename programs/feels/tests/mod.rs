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
    pub mod token_validate;
    
    // Tests using SDK - disabled until dependencies are available
    // pub mod protocol_lifecycle_sdk;
}

// Functional tests
#[cfg(test)]
pub mod functional {
    // Tests that don't require SDK
    pub mod amm_operations;
    
    // Tests using SDK - disabled until dependencies are available
    // pub mod amm_operations_sdk;
}

// Complex scenario tests
// Disabled until SDK dependencies are available
// #[cfg(test)]
// pub mod scenarios {
//     pub mod multi_user_scenarios;
//     pub mod stress_tests;
// }

