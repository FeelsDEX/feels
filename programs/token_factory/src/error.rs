use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum TokenFactoryError {
    #[msg("Ticker is restricted")]
    TickerIsRestricted,
    #[msg("Ticker is empty")]
    TickerIsEmpty,
    #[msg("Token name is empty")]
    TokenNameIsEmpty,
    #[msg("Token symbol is empty")]
    TokenSymbolIsEmpty,
    #[msg("Ticker is not alphanumeric")]
    TickerNotAlphanumeric,
    #[msg("Decimals value is too large")]
    DecimalsTooLarge,
    #[msg("Only feels protocol can access this operation")]
    UnauthorizedProtocol,
}
