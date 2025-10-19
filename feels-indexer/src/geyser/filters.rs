//! Geyser stream filters for Feels Protocol

// use super::client::geyser_stub::*;

/*
/// Create a subscription request for the Feels program
pub fn create_subscription_request(program_id: Pubkey) -> SubscribeRequest {
    use std::collections::HashMap;
    
    let mut accounts_filter = HashMap::new();
    
    // Subscribe to all accounts owned by the Feels program
    accounts_filter.insert(
        "feels_accounts".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![],
            owner: vec![program_id.to_string()],
        },
    );

    // Subscribe to transactions involving the Feels program
    let mut transactions_filter = HashMap::new();
    transactions_filter.insert(
        "feels_transactions".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            signature: vec![],
            account_include: vec![program_id.to_string()],
            account_exclude: vec![],
            account_required: vec![],
        },
    );

    SubscribeRequest {
        accounts: accounts_filter,
        transactions: transactions_filter,
        slots: std::collections::HashMap::new(),
        blocks: std::collections::HashMap::new(),
        commitment: Some(CommitmentLevel::Confirmed),
    }
}

/// Check if an account update is related to the Feels program
pub fn is_feels_account(account: &SubscribeUpdateAccount, program_id: &Pubkey) -> bool {
    if let Some(account_info) = &account.account {
        if let Ok(owner) = super::client::helpers::pubkey_from_bytes(&account_info.owner) {
            return &owner == program_id;
        }
    }
    false
}

/// Check if a transaction is related to the Feels program
pub fn is_feels_transaction(transaction: &SubscribeUpdateTransaction, _program_id: &Pubkey) -> bool {
    // For now, we'll accept all transactions and filter based on accounts
    // In production, we'd parse the transaction and check for program invocations
    transaction.transaction.is_some()
}
*/