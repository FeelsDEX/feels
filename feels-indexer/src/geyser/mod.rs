//! Geyser stream consumer for Feels Protocol (LEGACY - Being migrated to adapters)
//!
//! This module is being phased out. New code should use `crate::adapters::solana::geyser`.

// Re-export new implementations for backwards compatibility
pub use crate::adapters::solana::geyser::{
    FeelsGeyserClient,
    should_use_real_client,
};

// Legacy modules still in use
mod consumer;
mod filters;
mod stream_handler;

pub use consumer::*;
