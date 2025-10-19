/// Feels Jupiter Adapter
/// 
/// This crate provides the Jupiter AMM interface implementation for Feels Protocol.
/// It enables Feels markets to be discovered and used by Jupiter's routing engine.

pub mod amm;
pub mod config;

// Main exports for Jupiter integration
pub use amm::FeelsAmm;
pub use config::ADAPTER_CONFIG;

// Re-export the main Feels program for type access
pub use feels;

// Re-export SDK for shared utilities
pub use feels_sdk;

#[cfg(test)]
mod tests;