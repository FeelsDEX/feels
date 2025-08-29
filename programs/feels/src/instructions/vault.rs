/// Vault management instructions for depositing and withdrawing tokens from position vaults.
/// Position vaults allow users to deposit tokens and receive shares representing their claim
/// on the vault's managed positions. Different share types offer varying risk/return profiles
/// and lock periods to align incentives with vault strategy performance.
use anchor_lang::prelude::*;
use crate::state::{PositionVault, VaultShareAccount, ShareType, FeelsProtocolError};
use crate::utils::cpi_helpers;

// ============================================================================
// Vault Deposit Handler
// ============================================================================

/// Deposit tokens into the position vault and receive vault shares
/// Users can choose from different share types based on their risk/return preferences
pub fn deposit_to_vault(
    ctx: Context<crate::VaultDeposit>,
    share_type: ShareType,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, FeelsProtocolError::InputAmountZero);
    
    let clock = Clock::get()?;
    let position_vault = &mut ctx.accounts.position_vault;
    let share_account = &mut ctx.accounts.share_account;
    
    // Validate share type matches the token being deposited
    let pool = ctx.accounts.pool.load()?;
    let is_feelssol_deposit = ctx.accounts.deposit_token_mint.key() == pool.token_a_mint 
        || ctx.accounts.deposit_token_mint.key() == pool.token_b_mint;
    
    require!(
        share_type.is_feelssol() == is_feelssol_deposit,
        FeelsProtocolError::InvalidTokenForShareType
    );
    
    // Calculate shares to mint based on current vault NAV
    let shares_to_mint = position_vault.calculate_shares_to_mint(share_type, amount)?;
    require!(shares_to_mint > 0, FeelsProtocolError::ZeroShares);
    
    // Validate vault has capacity
    require!(
        position_vault.can_accept_deposit(share_type, amount),
        FeelsProtocolError::VaultCapacityExceeded
    );
    
    // Transfer tokens from user to vault
    cpi_helpers::transfer_tokens_to_vault(
        &ctx.accounts.user_token_account,
        &ctx.accounts.vault_token_account,
        &ctx.accounts.user,
        &ctx.accounts.token_program,
        amount,
    )?;
    
    // Update vault statistics
    position_vault.total_deposits = position_vault.total_deposits
        .checked_add(amount as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    position_vault.update_share_supply(share_type, shares_to_mint, true)?;
    position_vault.last_updated = clock.unix_timestamp;
    
    // Update or create user's position
    share_account.deposit(share_type, shares_to_mint, clock.unix_timestamp)?;
    
    // Emit deposit event
    emit!(crate::logic::event::VaultDepositEvent {
        vault: position_vault.key(),
        user: ctx.accounts.user.key(),
        share_type,
        token_amount: amount,
        shares_minted: shares_to_mint,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Vault deposit successful");
    msg!("Share type: {:?}", share_type);
    msg!("Token amount: {}", amount);
    msg!("Shares minted: {}", shares_to_mint);
    msg!("Total vault deposits: {}", position_vault.total_deposits);
    
    Ok(())
}

// ============================================================================
// Vault Withdraw Handler
// ============================================================================

/// Withdraw tokens from the position vault by burning vault shares
/// Enforces lock periods for time-locked share types
pub fn withdraw_from_vault(
    ctx: Context<crate::VaultWithdraw>,
    share_type: ShareType,
    shares: u128,
) -> Result<()> {
    require!(shares > 0, FeelsProtocolError::ZeroShares);
    
    let clock = Clock::get()?;
    let position_vault = &mut ctx.accounts.position_vault;
    let share_account = &mut ctx.accounts.share_account;
    
    // Check if user can withdraw (respects lock periods)
    require!(
        share_account.can_withdraw(share_type, clock.unix_timestamp),
        FeelsProtocolError::TokensLocked
    );
    
    // Get user's position
    let position = share_account.get_position(share_type)
        .ok_or(FeelsProtocolError::NoPositionFound)?;
    require!(position.shares >= shares, FeelsProtocolError::InsufficientShares);
    
    // Calculate redemption value based on current vault NAV
    let redemption_value = position_vault.calculate_redemption_value(share_type, shares)?;
    require!(redemption_value > 0, FeelsProtocolError::ZeroRedemption);
    
    // Validate vault has sufficient liquidity
    require!(
        ctx.accounts.vault_token_account.amount >= redemption_value,
        FeelsProtocolError::InsufficientVaultLiquidity
    );
    
    // Calculate withdrawal fee if applicable
    let fee_amount = position_vault.calculate_withdrawal_fee(share_type, redemption_value, position.lock_end_time, clock.unix_timestamp)?;
    let net_withdrawal = redemption_value.saturating_sub(fee_amount);
    
    // Transfer tokens from vault to user
    let vault_bump = ctx.bumps.position_vault;
    cpi_helpers::transfer_tokens_from_vault(
        &ctx.accounts.vault_token_account,
        &ctx.accounts.user_token_account,
        &ctx.accounts.position_vault,
        &ctx.accounts.token_program,
        net_withdrawal,
        vault_bump,
    )?;
    
    // Update vault statistics
    position_vault.total_withdrawals = position_vault.total_withdrawals
        .checked_add(redemption_value as u128)
        .ok_or(FeelsProtocolError::MathOverflow)?;
    position_vault.update_share_supply(share_type, shares, false)?;
    position_vault.last_updated = clock.unix_timestamp;
    
    // Handle withdrawal fee (if any)
    if fee_amount > 0 {
        position_vault.collected_fees = position_vault.collected_fees
            .checked_add(fee_amount as u128)
            .ok_or(FeelsProtocolError::MathOverflow)?;
    }
    
    // Update user's position
    share_account.withdraw(share_type, shares)?;
    
    // Emit withdrawal event
    emit!(crate::logic::event::VaultWithdrawEvent {
        vault: position_vault.key(),
        user: ctx.accounts.user.key(),
        share_type,
        shares_burned: shares,
        token_amount: net_withdrawal,
        fee_amount,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Vault withdrawal successful");
    msg!("Share type: {:?}", share_type);
    msg!("Shares burned: {}", shares);
    msg!("Token amount: {}", net_withdrawal);
    msg!("Fee amount: {}", fee_amount);
    msg!("Remaining shares: {}", position.shares.saturating_sub(shares));
    
    Ok(())
}