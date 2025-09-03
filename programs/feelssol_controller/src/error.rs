use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum FeelsSolError {
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
    #[msg("Amount can't be zero")]
    InvalidAmount,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Math operation resulted in overflow")]
    MathOverflow,
}
