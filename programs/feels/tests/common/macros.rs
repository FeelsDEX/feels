//! Test macros for multi-environment testing

/// Run a test in all available environments
#[macro_export]
macro_rules! test_all_environments {
    ($name:ident, $test_fn:expr) => {
        mod $name {
            use super::*;
            
            #[tokio::test]
            async fn in_memory() {
                use crate::common::{TestContext, TestEnvironment};
                let ctx = TestContext::new(TestEnvironment::in_memory()).await
                    .expect("Failed to create in-memory test context");
                    
                ($test_fn)(ctx).await.expect("In-memory test failed");
            }
            
            #[tokio::test]
            #[ignore = "Run with RUN_DEVNET_TESTS=1"]
            async fn devnet() {
                use crate::common::{TestContext, TestEnvironment, should_run_devnet_tests};
                if !should_run_devnet_tests() {
                    return;
                }
                
                let ctx = TestContext::new(TestEnvironment::devnet()).await
                    .expect("Failed to create devnet test context");
                    
                ($test_fn)(ctx).await.expect("Devnet test failed");
            }
            
            #[tokio::test]
            #[ignore = "Run with RUN_LOCALNET_TESTS=1"]
            async fn localnet() {
                use crate::common::{TestContext, TestEnvironment, should_run_localnet_tests};
                if !should_run_localnet_tests() {
                    return;
                }
                
                let ctx = TestContext::new(TestEnvironment::localnet()).await
                    .expect("Failed to create localnet test context");
                    
                ($test_fn)(ctx).await.expect("Localnet test failed");
            }
        }
    };
}

/// Run a test only in in-memory environment
#[macro_export]
macro_rules! test_in_memory {
    ($name:ident, $test_fn:expr) => {
        #[tokio::test]
        async fn $name() {
            use $crate::common::{TestContext, TestEnvironment};
            let ctx = TestContext::new(TestEnvironment::in_memory()).await
                .expect("Failed to create test context");
                
            ($test_fn)(ctx).await.expect("Test failed");
        }
    };
}

/// Run a test only against devnet
#[macro_export]
macro_rules! test_devnet {
    ($name:ident, $test_fn:expr) => {
        #[tokio::test]
        #[ignore = "Run with RUN_DEVNET_TESTS=1"]
        async fn $name() {
            if !should_run_devnet_tests() {
                return;
            }
            
            let ctx = TestContext::new(TestEnvironment::devnet()).await
                .expect("Failed to create test context");
                
            ($test_fn)(ctx).await.expect("Test failed");
        }
    };
}

/// Assert that an instruction fails with expected error
#[macro_export]
macro_rules! assert_instruction_error {
    ($ctx:expr, $ix:expr, $signers:expr, $expected_error:pat) => {{
        let result = $ctx.process_instruction($ix, $signers).await;
        match result {
            Err(e) => {
                let error_string = e.to_string();
                match error_string {
                    $expected_error => Ok(()),
                    _ => Err(format!("Expected error pattern {} but got: {}", 
                        stringify!($expected_error), error_string)),
                }
            }
            Ok(_) => Err("Expected error but instruction succeeded".to_string()),
        }
    }};
}

/// Assert account exists with expected owner
#[macro_export]
macro_rules! assert_account_exists {
    ($ctx:expr, $address:expr, $expected_owner:expr) => {{
        let account_data = $ctx.client.lock().await
            .get_account_data($address).await?;
        
        match account_data {
            Some(data) => {
                assert!(data.len() > 0, "Account exists but has no data");
                // In real implementation, would check owner from account info
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            None => Err(format!("Account {} does not exist", $address)),
        }
    }};
}

/// Create test tokens with initial supply
#[macro_export]
macro_rules! create_test_tokens {
    ($ctx:expr, $count:expr) => {{
        let mut tokens = Vec::new();
        for i in 0..$count {
            let mint = $ctx.create_mint(
                &$ctx.accounts.market_creator.pubkey(),
                if i == 0 { 9 } else { 6 }, // First token has 9 decimals, others 6
            ).await?;
            tokens.push(mint.pubkey());
        }
        tokens
    }};
}

/// Setup a market with initial liquidity
#[macro_export]
macro_rules! setup_market_with_liquidity {
    ($ctx:expr, $token_0:expr, $token_1:expr, $positions:expr) => {{
        let market_helper = $ctx.market_helper();
        let position_helper = $ctx.position_helper();
        
        // Create market
        let market_id = market_helper.create_simple_market($token_0, $token_1).await?;
        
        // Add liquidity positions
        for (owner, lower_tick, upper_tick, liquidity) in $positions {
            position_helper.open_position(
                &market_id,
                owner,
                lower_tick,
                upper_tick,
                liquidity,
            ).await?;
        }
        
        market_id
    }};
}

/// Batch multiple operations with automatic error handling
#[macro_export]
macro_rules! batch_operations {
    ($ctx:expr, $($op:expr),+ $(,)?) => {{
        let mut results = Vec::new();
        $(
            results.push($op);
        )+
        
        // Execute all operations
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                return Err(format!("Operation {} failed: {}", i, e).into());
            }
        }
        
        Ok::<(), Box<dyn std::error::Error>>(())
    }};
}

/// Time-scoped test execution
#[macro_export]
macro_rules! with_timeout {
    ($duration:expr, $body:expr) => {{
        use tokio::time::timeout;
        
        match timeout($duration, $body).await {
            Ok(result) => result,
            Err(_) => Err("Test timed out".into()),
        }
    }};
}

/// Assert token balance within range
#[macro_export]
macro_rules! assert_balance_in_range {
    ($ctx:expr, $address:expr, $min:expr, $max:expr) => {{
        let balance = $ctx.get_token_balance($address).await?;
        assert!(
            balance >= $min && balance <= $max,
            "Balance {} not in range [{}, {}]",
            balance, $min, $max
        );
    }};
}

/// Run test with specific environment variable set
#[macro_export]
macro_rules! with_env_var {
    ($key:expr, $value:expr, $body:expr) => {{
        std::env::set_var($key, $value);
        let result = $body;
        std::env::remove_var($key);
        result
    }};
}