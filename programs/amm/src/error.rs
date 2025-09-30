use anchor_lang::prelude::*;

// Errors
#[error_code]
pub enum AmmError {
    #[msg("Tick spacing must be positive")]
    InvalidTickSpacing,
}
