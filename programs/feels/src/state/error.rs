use anchor_lang::prelude::*;

#[error_code]
pub enum FeelsError {
    #[msg("Invalid metadata format")]
    InvalidMetadata,
    #[msg("Insufficient token balance")]
    InsufficientBalance,
    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Invalid token amount")]
    InvalidAmount,
    #[msg("Token mint operation failed")]
    MintFailed,
    #[msg("Token burn operation failed")]
    BurnFailed,
}
