//! Test environment configuration

// use super::*;

/// Test environment type
#[derive(Clone, Debug)]
pub enum TestEnvironment {
    /// In-memory testing using ProgramTest
    InMemory,
    
    /// Testing against devnet
    Devnet { 
        url: String,
        payer_path: Option<String>,
    },
    
    /// Testing against localnet
    Localnet { 
        url: String,
        payer_path: Option<String>,
    },
}

impl TestEnvironment {
    /// Create in-memory test environment
    pub fn in_memory() -> Self {
        TestEnvironment::InMemory
    }
    
    /// Create devnet test environment
    pub fn devnet() -> Self {
        TestEnvironment::Devnet {
            url: "https://api.devnet.solana.com".to_string(),
            payer_path: None,
        }
    }
    
    /// Create localnet test environment
    pub fn localnet() -> Self {
        TestEnvironment::Localnet {
            url: "http://localhost:8899".to_string(),
            payer_path: Some("keypairs/payer.json".to_string()),
        }
    }
    
    /// Create localnet with custom URL
    pub fn localnet_with_url(url: &str) -> Self {
        TestEnvironment::Localnet {
            url: url.to_string(),
            payer_path: None,
        }
    }
}

/// Check if we should run devnet tests
pub fn should_run_devnet_tests() -> bool {
    std::env::var("RUN_DEVNET_TESTS").is_ok()
}

/// Check if we should run localnet tests
pub fn should_run_localnet_tests() -> bool {
    std::env::var("RUN_LOCALNET_TESTS").is_ok()
}

/// Get the current test environment from env vars
pub fn current_test_environment() -> TestEnvironment {
    if let Ok(url) = std::env::var("TEST_RPC_URL") {
        TestEnvironment::Localnet {
            url,
            payer_path: std::env::var("TEST_PAYER_PATH").ok(),
        }
    } else if should_run_devnet_tests() {
        TestEnvironment::devnet()
    } else {
        TestEnvironment::in_memory()
    }
}