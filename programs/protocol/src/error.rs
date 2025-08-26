use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum ProtocolError {
    #[msg("Protocol fee rate too high")]
    ProtocolFeeTooHigh,
    #[msg("Pool fee rate too high")]
    PoolFeeTooHigh,
}
