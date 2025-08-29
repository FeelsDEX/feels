/// Cross-Program Invocation helpers for common operations providing reusable functions
/// for token transfers and other CPIs. Designed to be flexible for Phase 2 Valence integration
/// where transfers may transition to atomic position vault adjustments.
/// Includes comprehensive error handling and validation for safe cross-program calls.

use anchor_lang::prelude::*;
use anchor_spl::token_2022::{transfer, Transfer};
use crate::state::Pool;
use crate::utils::CanonicalSeeds;

// ============================================================================
// Unified Transfer Parameters
// ============================================================================

/// Unified transfer parameters structure
#[derive(Debug, Clone)]
pub struct TransferParams<'info> {
    pub from: AccountInfo<'info>,
    pub to: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub amount: u64,
    pub signer_seeds: Vec<Vec<u8>>,
}

impl<'info> TransferParams<'info> {
    /// Create transfer params for user to pool transfer
    pub fn user_to_pool(
        user_token_account: AccountInfo<'info>,
        pool_vault: AccountInfo<'info>,
        user_authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Self {
        Self {
            from: user_token_account,
            to: pool_vault,
            authority: user_authority,
            token_program,
            amount,
            signer_seeds: vec![], // No signer seeds for user transfers
        }
    }

    /// Create transfer params for pool to user transfer
    pub fn pool_to_user(
        pool_vault: AccountInfo<'info>,
        user_token_account: AccountInfo<'info>,
        pool_authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
        pool: &Pool,
        pool_bump: u8,
    ) -> Self {
        let pool_seeds = CanonicalSeeds::get_pool_seeds(
            &pool.token_a_mint,
            &pool.token_b_mint,
            pool.fee_rate,
            pool_bump,
        );
        
        Self {
            from: pool_vault,
            to: user_token_account,
            authority: pool_authority,
            token_program,
            amount,
            signer_seeds: pool_seeds,
        }
    }
}

// ============================================================================
// Basic Transfer Helper
// ============================================================================

/// Unified token transfer function using TransferParams
pub fn transfer_tokens_unified<'info>(params: TransferParams<'info>) -> Result<()> {
    if params.amount == 0 {
        return Ok(());
    }

    let transfer_accounts = Transfer {
        from: params.from,
        to: params.to,
        authority: params.authority,
    };

    if params.signer_seeds.is_empty() {
        let cpi_ctx = CpiContext::new(params.token_program, transfer_accounts);
        transfer(cpi_ctx, params.amount)
    } else {
        // Convert Vec<Vec<u8>> to proper signer seeds format
        let signer_seeds: Vec<&[u8]> = params.signer_seeds.iter().map(|s| s.as_slice()).collect();
        let signer_slice: &[&[u8]] = signer_seeds.as_slice();
        let signer_array = [signer_slice];
        
        let cpi_ctx = CpiContext::new_with_signer(
            params.token_program, 
            transfer_accounts, 
            &signer_array
        );
        transfer(cpi_ctx, params.amount)
    }
}

/// Helper function to transfer tokens using CPI (legacy support)
/// This is the most general transfer function that others build upon
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

/// Helper function to transfer tokens using CPI with signer seeds
/// Convenience wrapper for signed transfers
pub fn transfer_tokens_signed<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    transfer_tokens(from, to, authority, token_program, amount, signer_seeds)
}

// ============================================================================
// User Transfer Helpers
// ============================================================================


/// Transfer tokens from user account to pool vault
/// Used in swaps and liquidity additions where user is the authority
pub fn transfer_from_user_to_pool<'info>(
    user_token_account: AccountInfo<'info>,
    pool_vault: AccountInfo<'info>,
    user_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    transfer_tokens(
        user_token_account,
        pool_vault,
        user_authority,
        token_program,
        amount,
        &[], // No signer seeds needed for user transfers
    )
}

// ============================================================================
// Pool Transfer Helpers
// ============================================================================

/// Transfer tokens from pool vault to user account
/// Used in swaps, liquidity removal, and fee collection
pub fn transfer_from_pool_to_user<'info>(
    pool_vault: AccountInfo<'info>,
    user_token_account: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    // Get pool seeds for PDA signing
    let pool_seeds = CanonicalSeeds::get_pool_seeds(
        &pool.token_a_mint,
        &pool.token_b_mint,
        pool.fee_rate,
        pool_bump,
    );

    let signer_seeds: Vec<&[u8]> = pool_seeds.iter().map(|s| s.as_slice()).collect();
    let signer = &[signer_seeds.as_slice()];

    transfer_tokens(
        pool_vault,
        user_token_account,
        pool_authority,
        token_program,
        amount,
        signer,
    )
}

/// Transfer tokens from pool vault to any recipient (protocol fees, etc.)
/// More general version that doesn't assume recipient is a user
pub fn transfer_from_pool<'info>(
    pool_vault: AccountInfo<'info>,
    recipient: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    // Get pool seeds for PDA signing
    let pool_seeds = CanonicalSeeds::get_pool_seeds(
        &pool.token_a_mint,
        &pool.token_b_mint,
        pool.fee_rate,
        pool_bump,
    );

    let signer_seeds: Vec<&[u8]> = pool_seeds.iter().map(|s| s.as_slice()).collect();
    let signer = &[signer_seeds.as_slice()];

    transfer_tokens(
        pool_vault,
        recipient,
        pool_authority,
        token_program,
        amount,
        signer,
    )
}

// Aliases for flash loan instructions
pub use self::transfer_from_pool as transfer_tokens_from_pool;
pub use self::transfer_from_user_to_pool as transfer_tokens_to_pool;

// ============================================================================
// Batch Transfer Helpers
// ============================================================================

/// Transfer both tokens from user to pool (for liquidity additions)
#[allow(clippy::too_many_arguments)]
pub fn transfer_pair_from_user_to_pool<'info>(
    user_token_a: AccountInfo<'info>,
    user_token_b: AccountInfo<'info>,
    pool_vault_a: AccountInfo<'info>,
    pool_vault_b: AccountInfo<'info>,
    user_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    // Transfer token a
    transfer_from_user_to_pool(
        user_token_a,
        pool_vault_a,
        user_authority.clone(),
        token_program.clone(),
        amount_a,
    )?;

    // Transfer token b
    transfer_from_user_to_pool(
        user_token_b,
        pool_vault_b,
        user_authority,
        token_program,
        amount_b,
    )
}

/// Transfer both tokens from pool to user (for liquidity removal/fees)
#[allow(clippy::too_many_arguments)]
pub fn transfer_pair_from_pool_to_user<'info>(
    pool_vault_a: AccountInfo<'info>,
    pool_vault_b: AccountInfo<'info>,
    user_token_a: AccountInfo<'info>,
    user_token_b: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_a: u64,
    amount_b: u64,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    // Transfer token a if amount > 0
    if amount_a > 0 {
        transfer_from_pool_to_user(
            pool_vault_a,
            user_token_a,
            pool_authority.clone(),
            token_program.clone(),
            amount_a,
            pool,
            pool_bump,
        )?;
    }

    // Transfer token b if amount > 0
    if amount_b > 0 {
        transfer_from_pool_to_user(
            pool_vault_b,
            user_token_b,
            pool_authority,
            token_program,
            amount_b,
            pool,
            pool_bump,
        )?;
    }

    Ok(())
}

// ============================================================================
// Swap-Specific Transfer Helpers
// ============================================================================

/// Execute token transfers for a swap operation
/// Handles both directions (token 0 to 1, or token 1 to 0)
#[allow(clippy::too_many_arguments)]
pub fn execute_swap_transfers<'info>(
    user_token_a: AccountInfo<'info>,
    user_token_b: AccountInfo<'info>,
    pool_token_a: AccountInfo<'info>,
    pool_token_b: AccountInfo<'info>,
    user_authority: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_in: u64,
    amount_out: u64,
    is_token_a_to_b: bool,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    if is_token_a_to_b {
        // User pays token A, receives token B
        transfer_from_user_to_pool(
            user_token_a,
            pool_token_a,
            user_authority,
            token_program.clone(),
            amount_in,
        )?;

        transfer_from_pool_to_user(
            pool_token_b,
            user_token_b,
            pool_authority,
            token_program,
            amount_out,
            pool,
            pool_bump,
        )
    } else {
        // User pays token B, receives token A
        transfer_from_user_to_pool(
            user_token_b,
            pool_token_b,
            user_authority,
            token_program.clone(),
            amount_in,
        )?;

        transfer_from_pool_to_user(
            pool_token_a,
            user_token_a,
            pool_authority,
            token_program,
            amount_out,
            pool,
            pool_bump,
        )
    }
}

// ============================================================================
// Protocol Fee Transfer Helpers
// ============================================================================

/// Collect protocol fees from pool vaults
/// Used by protocol_fee_collect instruction
#[allow(clippy::too_many_arguments)]
pub fn collect_protocol_fees<'info>(
    pool_vault_a: AccountInfo<'info>,
    pool_vault_b: AccountInfo<'info>,
    recipient_a: AccountInfo<'info>,
    recipient_b: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_a: u64,
    amount_b: u64,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    // Collect fee for token a
    if amount_a > 0 {
        transfer_from_pool(
            pool_vault_a,
            recipient_a,
            pool_authority.clone(),
            token_program.clone(),
            amount_a,
            pool,
            pool_bump,
        )?;
    }

    // Collect fee for token b
    if amount_b > 0 {
        transfer_from_pool(
            pool_vault_b,
            recipient_b,
            pool_authority,
            token_program,
            amount_b,
            pool,
            pool_bump,
        )?;
    }

    Ok(())
}

// ============================================================================
// Multi-Hop Swap Transfer Helpers
// ============================================================================

/// Execute token transfers for a single-hop routed swap
/// Used when swapping through a single pool (e.g., LST -> FeelsSOL or FeelsSOL -> MemeToken)
#[allow(clippy::too_many_arguments)]
pub fn execute_single_hop_swap<'info>(
    user_token_in: AccountInfo<'info>,
    user_token_out: AccountInfo<'info>,
    pool_token_in: AccountInfo<'info>,
    pool_token_out: AccountInfo<'info>,
    user_authority: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_in: u64,
    amount_out: u64,
    pool: &Pool,
    pool_bump: u8,
) -> Result<()> {
    // Transfer input from user to pool
    transfer_from_user_to_pool(
        user_token_in,
        pool_token_in,
        user_authority,
        token_program.clone(),
        amount_in,
    )?;

    // Transfer output from pool to user
    transfer_from_pool_to_user(
        pool_token_out,
        user_token_out,
        pool_authority,
        token_program,
        amount_out,
        pool,
        pool_bump,
    )
}

/// Execute token transfers for a two-hop routed swap
/// Used when swapping through two pools with an intermediate token (e.g., LST -> FeelsSOL -> MemeToken)
#[allow(clippy::too_many_arguments)]
pub fn execute_two_hop_swap<'info>(
    user_token_in: AccountInfo<'info>,
    user_token_out: AccountInfo<'info>,
    pool_1_token_in: AccountInfo<'info>,
    pool_1_token_out: AccountInfo<'info>,
    pool_2_token_in: AccountInfo<'info>,
    pool_2_token_out: AccountInfo<'info>,
    user_authority: AccountInfo<'info>,
    pool_1_authority: AccountInfo<'info>,
    pool_2_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount_in: u64,
    intermediate_amount: u64,
    final_amount: u64,
    pool_1: &Pool,
    pool_1_bump: u8,
    pool_2: &Pool,
    pool_2_bump: u8,
) -> Result<()> {
    // Step 1: Transfer input from user to pool 1
    transfer_from_user_to_pool(
        user_token_in,
        pool_1_token_in,
        user_authority,
        token_program.clone(),
        amount_in,
    )?;

    // Step 2: Transfer intermediate token from pool 1 to pool 2
    let pool_1_seeds = CanonicalSeeds::get_pool_seeds(
        &pool_1.token_a_mint,
        &pool_1.token_b_mint,
        pool_1.fee_rate,
        pool_1_bump,
    );
    let signer_seeds_1: Vec<&[u8]> = pool_1_seeds.iter().map(|s| s.as_slice()).collect();
    let signer_1 = &[signer_seeds_1.as_slice()];

    transfer_tokens(
        pool_1_token_out,
        pool_2_token_in,
        pool_1_authority,
        token_program.clone(),
        intermediate_amount,
        signer_1,
    )?;

    // Step 3: Transfer output from pool 2 to user
    transfer_from_pool_to_user(
        pool_2_token_out,
        user_token_out,
        pool_2_authority,
        token_program,
        final_amount,
        pool_2,
        pool_2_bump,
    )
}

// ============================================================================
// Vault CPI Helpers
// ============================================================================

/// Transfer tokens from user to vault
pub fn transfer_tokens_to_vault(
    from: AccountInfo,
    to: AccountInfo,
    authority: AccountInfo,
    token_program: AccountInfo,
    amount: u64,
) -> Result<()> {
    transfer_from_user_to_pool(from, to, authority, token_program, amount)
}

/// Transfer tokens from vault to user
pub fn transfer_tokens_from_vault(
    from: AccountInfo,
    to: AccountInfo,
    vault_authority: AccountInfo,
    token_program: AccountInfo,
    amount: u64,
    vault_seeds: &[&[u8]],
) -> Result<()> {
    let transfer_ix = spl_token_2022::instruction::transfer(
        token_program.key,
        from.key,
        to.key,
        vault_authority.key,
        &[],
        amount,
    )?;

    solana_program::program::invoke_signed(
        &transfer_ix,
        &[from, to, vault_authority, token_program],
        &[vault_seeds],
    )?;

    Ok(())
}
