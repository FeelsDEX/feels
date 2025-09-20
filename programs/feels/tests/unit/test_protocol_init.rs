//! Minimal test for protocol initialization debugging

use crate::common::*;

#[tokio::test]
async fn test_protocol_initialization_minimal() {
    println!("\n=== Minimal Protocol Initialization Test ===");

    // Create context
    let ctx = match TestContext::new(TestEnvironment::InMemory).await {
        Ok(ctx) => {
            println!("✓ Test context created successfully");
            ctx
        }
        Err(e) => {
            panic!("Failed to create test context: {:?}", e);
        }
    };

    println!("✓ Test completed");
}
