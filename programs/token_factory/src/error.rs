use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum TokenFactoryError {
    #[msg("Decimals value is too large")]
    DecimalsTooLarge,
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
}
