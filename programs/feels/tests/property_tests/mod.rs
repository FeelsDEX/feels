/// Property-based testing suite for the Feels Protocol.
/// Uses proptest to verify invariants and properties across the system.

pub mod conservation_laws;
pub mod instantaneous_fees;
pub mod market_verification;

// Re-export test utilities
pub use conservation_laws::*;
pub use instantaneous_fees::*;