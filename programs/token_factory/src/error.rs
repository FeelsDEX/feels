use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum TokenFactoryError {
    #[msg("Symbol is restricted")]
    SymbolIsRestricted,
    #[msg("Symbol is empty")]
    SymbolIsEmpty,
    #[msg("Symbol is not alphanumeric")]
    SymbolNotAlphanumeric,
    #[msg("Symbol is too long")]
    SymbolTooLong,
    #[msg("Symbol must be uppercase")]
    SymbolNotUppercase,
    #[msg("Token name is empty")]
    NameIsEmpty,
    #[msg("Token name is too long")]
    NameTooLong,
    #[msg("Decimals value is too large")]
    DecimalsTooLarge,
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
    #[msg("Invalid token metadata")]
    InvalidMetadata,
}
