//! Prelude module for common imports
//!
//! This module consolidates imports to minimize direct anchor dependencies.
//! We only use anchor for serialization traits, everything else comes from solana-sdk.

// Re-export only what we need from anchor
pub use anchor_lang::{AnchorDeserialize, AnchorSerialize};

// Everything else comes from solana-sdk
pub use solana_sdk::{pubkey::Pubkey, sysvar};
