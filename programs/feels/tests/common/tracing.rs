//! Tracing configuration for tests
//!
//! Initializes tracing to suppress OpenTelemetry warnings and provide better test output

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing for tests
///
/// This function:
/// - Sets up a tracing subscriber to capture logs
/// - Filters out noisy warnings from dependencies
/// - Can be called multiple times safely (only initializes once)
pub fn init_test_tracing() {
    INIT.call_once(|| {
        // Use EnvFilter to control log levels
        let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default filter: show info and above for our code,
            // but suppress tarpc and OpenTelemetry warnings
            tracing_subscriber::EnvFilter::new("info,tarpc::client=off,tarpc=off")
        });

        // Set up the subscriber
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer() // Use test-friendly output
            .init();
    });
}

/// Alternative: completely disable tracing for tests
#[allow(dead_code)]
pub fn disable_test_tracing() {
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::new("off");
        tracing_subscriber::fmt().with_env_filter(filter).init();
    });
}
