use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum ProtocolError {
    #[msg("Fee rate too high")]
    FeeTooHigh,
}
