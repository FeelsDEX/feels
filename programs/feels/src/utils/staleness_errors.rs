/// Enhanced error messages for staleness violations
use anchor_lang::prelude::*;

/// Log detailed staleness error with context
pub fn log_staleness_error(
    error_type: &str,
    current_value: i64,
    limit_value: i64,
    field_name: &str,
) {
    msg!("STALENESS VIOLATION: {}", error_type);
    msg!("  Field: {}", field_name);
    msg!("  Current value: {}", current_value);
    msg!("  Limit: {}", limit_value);
    msg!("  Violation: {} > {}", current_value, limit_value);
}

/// Log update frequency error with context
pub fn log_frequency_error(
    time_since_last: i64,
    required_interval: i64,
    source: &str,
) {
    msg!("UPDATE FREQUENCY VIOLATION");
    msg!("  Source: {}", source);
    msg!("  Time since last update: {} seconds", time_since_last);
    msg!("  Required interval: {} seconds", required_interval);
    msg!("  Too frequent by: {} seconds", required_interval - time_since_last);
}

/// Log commitment expiration error
pub fn log_expiration_error(
    current_time: i64,
    expires_at: i64,
    commitment_id: u64,
) {
    msg!("COMMITMENT EXPIRED");
    msg!("  Commitment sequence: {}", commitment_id);
    msg!("  Current time: {}", current_time);
    msg!("  Expired at: {}", expires_at);
    msg!("  Expired by: {} seconds", current_time - expires_at);
}