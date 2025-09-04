use anchor_lang::prelude::*;

#[error_code]
pub enum KeeperError {
    #[msg("Rate value cannot be zero")]
    ZeroRate,
}
