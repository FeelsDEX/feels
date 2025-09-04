//! # Market State - Unified Market Account
//! 
//! This module re-exports the unified Market account.
//! The old MarketField and MarketManager have been consolidated into a single Market account.

// Re-export the unified Market structure
pub use super::unified_market::{Market, DomainWeights};

// Type alias for clarity
pub type UnifiedMarket = Market;