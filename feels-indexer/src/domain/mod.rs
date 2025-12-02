//! Domain layer - business logic and domain models
//!
//! This layer contains the core business logic of the indexer,
//! independent of infrastructure concerns like databases or APIs.

pub mod models;
pub mod processors;
pub mod services;

// Re-export commonly used items
pub use models::*;

