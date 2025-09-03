/// Shared types for the Feels Protocol
/// 
/// This crate provides common type definitions, constants, and utilities
/// that are used across the SDK, keeper, and other protocol components.

pub mod constants;
pub mod errors;
pub mod field;
pub mod market;
pub mod math;

// Re-export all public types
pub use constants::*;
pub use errors::*;
pub use field::*;
pub use market::*;
pub use math::*;

/// Result type alias using the shared error type
pub type FeelsResult<T> = std::result::Result<T, FeelsProtocolError>;