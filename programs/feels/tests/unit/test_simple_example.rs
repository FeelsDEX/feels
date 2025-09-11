//! Simple test to verify the test infrastructure works

use crate::common::*;

async fn test_logic(ctx: TestContext) -> TestResult<()> {
    println!("Test context created successfully");
    assert_ne!(ctx.feelssol_mint, Pubkey::default());
    Ok(())
}

test_in_memory!(test_simple_example, test_logic);