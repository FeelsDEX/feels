/// Cross-Program Invocation helpers for common operations
/// Provides reusable functions for token transfers and other CPIs

use anchor_lang::prelude::*;
#[allow(deprecated)]
use anchor_spl::token_2022::{Transfer, transfer};

/// Helper function to transfer tokens using CPI
pub fn transfer_tokens<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    
    let transfer_accounts = Transfer {
        from,
        to,
        authority,
    };
    
    let cpi_ctx = if signer_seeds.is_empty() {
        CpiContext::new(token_program, transfer_accounts)
    } else {
        CpiContext::new_with_signer(token_program, transfer_accounts, signer_seeds)
    };
    
    transfer(cpi_ctx, amount)
}