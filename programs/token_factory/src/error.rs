use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum TokenFactoryError {
    #[msg("Ticker is restricted")]
    TickerIsRestricted,
    #[msg("Ticker is empty")]
    TickerIsEmpty,
    #[msg("Token name is empty")]
    NameIsEmpty,
    #[msg("Token symbol is empty")]
    SymbolIsEmpty,
    #[msg("Ticker is not alphanumeric")]
    TickerNotAlphanumeric,
    #[msg("Ticker is too long")]
    TickerTooLong,
    #[msg("Token name is too long")]
    NameTooLong,
    #[msg("Token symbol is too long")]
    SymbolTooLong,
    #[msg("Decimals value is too large")]
    DecimalsTooLarge,
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
}
