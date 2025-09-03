/// Clock utility functions for timestamp management
use anchor_lang::prelude::*;
// use crate::error::FeelsProtocolError; // Unused import

/// Get the current timestamp from the Solana clock
pub fn current_timestamp() -> Result<i64> {
    let clock = Clock::get()?;
    Ok(clock.unix_timestamp)
}

/// Check if a timestamp is within a specified window
pub fn is_timestamp_fresh(timestamp: i64, max_age_seconds: i64) -> Result<bool> {
    let current = current_timestamp()?;
    Ok(current - timestamp <= max_age_seconds)
}

/// Calculate time elapsed since a timestamp
pub fn time_elapsed_since(timestamp: i64) -> Result<i64> {
    let current = current_timestamp()?;
    Ok(current - timestamp)
}