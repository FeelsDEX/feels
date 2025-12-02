//! Geyser client implementations (real and mock)

#[cfg(feature = "real-geyser")]
pub mod client_real;

#[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
pub mod client_mock;

pub mod client_unified;

// Re-export main types from unified client
pub use client_unified::{FeelsGeyserClient, GeyserStream, SubscribeUpdate, should_use_real_client};

// Re-export UpdateOneof type for pattern matching
#[cfg(feature = "real-geyser")]
pub use client_real::UpdateOneof;

#[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
pub use client_mock::geyser_stub::UpdateOneof;

// Re-export helper functions
#[cfg(feature = "real-geyser")]
pub use client_real::helpers;

#[cfg(all(feature = "mock-geyser", not(feature = "real-geyser")))]
pub use client_mock::helpers;
