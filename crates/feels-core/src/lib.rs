//! # Feels Core - Shared Protocol Logic
//! 
//! This crate contains the core types and mathematical logic shared between
//! on-chain programs and off-chain clients. It provides:
//! 
//! - Type definitions for market structures
//! - Mathematical functions for thermodynamic calculations
//! - Constants and configuration values
//! - Pure validation logic
//! 
//! ## Feature Flags
//! 
//! - `anchor`: Enables Anchor serialization for on-chain use
//! - `client`: Enables standard serialization for off-chain use

// Re-export all modules
pub mod constants;
pub mod errors;
pub mod math;
pub mod oracle;
pub mod physics;
pub mod types;

// Re-export commonly used items
pub use constants::*;
pub use errors::{FeelsCoreError, CoreResult};
pub use types::*;