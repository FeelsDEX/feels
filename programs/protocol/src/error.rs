use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum ProtocolError {
    #[msg("Protocol fee rate too high")]
    ProtocolFeeTooHigh,
    #[msg("Pool fee rate too high")]
    PoolFeeTooHigh,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("There is a pending authority transfer")]
    PendingAuthorityTransferExists,
    #[msg("No pending authority transfer exists")]
    NoPendingAuthorityTransfer,
    #[msg("Authority transfer delay not met")]
    AuthorityTransferDelayNotMet,
    #[msg("Not pending authority transfer")]
    NotPendingAuthority,
    #[msg("Protocol is paused")]
    ProtocolPaused,
}
