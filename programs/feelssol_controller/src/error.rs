use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum FeelsSolError {
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
    #[msg("Amount deposited can't be zero")]
    InvalidDepositAmount,
    #[msg("Math operation resulted in overflow")]
    MathOverflow,
}
