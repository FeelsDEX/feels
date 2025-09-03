use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

/// Query SPL token vault balance from account
pub fn get_vault_balance(vault_account: &AccountInfo) -> Result<Option<u64>> {
    // Try to deserialize as TokenAccount
    if vault_account.owner == &anchor_spl::token::ID {
        // Deserialize the account data
        let token_account = TokenAccount::try_deserialize(&mut &vault_account.data.borrow()[..])?;
        Ok(Some(token_account.amount))
    } else {
        msg!("Vault account is not a valid SPL token account");
        Ok(None)
    }
}

/// Query vault balances from optional accounts
pub fn get_vault_balances<'info>(
    vault_0: &Option<UncheckedAccount<'info>>,
    vault_1: &Option<UncheckedAccount<'info>>,
) -> Result<(Option<u64>, Option<u64>)> {
    let balance_0 = if let Some(vault) = vault_0 {
        get_vault_balance(&vault.to_account_info())?
    } else {
        None
    };
    
    let balance_1 = if let Some(vault) = vault_1 {
        get_vault_balance(&vault.to_account_info())?
    } else {
        None
    };
    
    Ok((balance_0, balance_1))
}