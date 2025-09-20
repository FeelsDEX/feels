//! Geyser client and stream handling tests

use anyhow::Result;
use feels_indexer::geyser::client::{FeelsGeyserClient, helpers, geyser_proto::*};
use feels_indexer::geyser::filters::{is_feels_account, is_feels_transaction};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Test helper functions for working with Geyser data
#[tokio::test]
async fn test_pubkey_conversion() -> Result<()> {
    let original_pubkey = Pubkey::from_str("11111111111111111111111111111112")?;
    let bytes = original_pubkey.to_bytes();
    
    // Test conversion from bytes back to pubkey
    let converted_pubkey = helpers::pubkey_from_bytes(&bytes)?;
    assert_eq!(original_pubkey, converted_pubkey);
    
    // Test with invalid length
    let invalid_bytes = vec![0u8; 16]; // Wrong length
    let result = helpers::pubkey_from_bytes(&invalid_bytes);
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_account_update_helpers() -> Result<()> {
    let program_id = Pubkey::from_str("Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N")?;
    let account_pubkey = Pubkey::new_unique();
    
    // Create a mock account update
    let account_info = SubscribeUpdateAccountInfo {
        pubkey: account_pubkey.to_bytes().to_vec(),
        lamports: 1000000,
        owner: program_id.to_bytes().to_vec(),
        executable: false,
        rent_epoch: 361,
        data: vec![1, 2, 3, 4, 5], // Mock account data
        write_version: 1,
        txn_signature: None,
    };
    
    let account_update = SubscribeUpdateAccount {
        account: Some(account_info),
        slot: 12345678,
        is_startup: false,
    };
    
    // Test helper functions
    assert!(helpers::is_feels_account_update(&account_update, &program_id));
    
    let extracted_pubkey = helpers::extract_account_pubkey(&account_update);
    assert_eq!(extracted_pubkey, Some(account_pubkey));
    
    let extracted_data = helpers::extract_account_data(&account_update);
    assert_eq!(extracted_data, Some(vec![1, 2, 3, 4, 5].as_slice()));
    
    Ok(())
}

#[tokio::test]
async fn test_account_filtering() -> Result<()> {
    let feels_program_id = Pubkey::from_str("Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N")?;
    let other_program_id = Pubkey::new_unique();
    
    // Create account update owned by Feels program
    let feels_account_info = SubscribeUpdateAccountInfo {
        pubkey: Pubkey::new_unique().to_bytes().to_vec(),
        lamports: 1000000,
        owner: feels_program_id.to_bytes().to_vec(),
        executable: false,
        rent_epoch: 361,
        data: vec![1, 2, 3, 4, 5],
        write_version: 1,
        txn_signature: None,
    };
    
    let feels_account_update = SubscribeUpdateAccount {
        account: Some(feels_account_info),
        slot: 12345678,
        is_startup: false,
    };
    
    // Create account update owned by different program
    let other_account_info = SubscribeUpdateAccountInfo {
        pubkey: Pubkey::new_unique().to_bytes().to_vec(),
        lamports: 1000000,
        owner: other_program_id.to_bytes().to_vec(),
        executable: false,
        rent_epoch: 361,
        data: vec![1, 2, 3, 4, 5],
        write_version: 1,
        txn_signature: None,
    };
    
    let other_account_update = SubscribeUpdateAccount {
        account: Some(other_account_info),
        slot: 12345678,
        is_startup: false,
    };
    
    // Test filtering
    assert!(is_feels_account(&feels_account_update, &feels_program_id));
    assert!(!is_feels_account(&other_account_update, &feels_program_id));
    
    Ok(())
}

#[tokio::test]
async fn test_transaction_filtering() -> Result<()> {
    let feels_program_id = Pubkey::from_str("Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N")?;
    let other_program_id = Pubkey::new_unique();
    
    // Create transaction involving Feels program
    let feels_message = Message {
        header: Some(MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        }),
        account_keys: vec![
            Pubkey::new_unique().to_bytes().to_vec(), // Signer
            feels_program_id.to_bytes().to_vec(),     // Feels program
        ],
        recent_blockhash: vec![0u8; 32],
        instructions: vec![],
        address_table_lookups: vec![],
    };
    
    let feels_transaction = Transaction {
        signatures: vec![vec![0u8; 64]],
        message: Some(feels_message),
    };
    
    let feels_transaction_info = SubscribeUpdateTransactionInfo {
        signature: vec![0u8; 64],
        is_vote: false,
        meta: None,
        transaction: Some(feels_transaction),
        index: 0,
    };
    
    let feels_transaction_update = SubscribeUpdateTransaction {
        transaction: Some(feels_transaction_info),
        slot: 12345678,
    };
    
    // Create transaction not involving Feels program
    let other_message = Message {
        header: Some(MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 1,
        }),
        account_keys: vec![
            Pubkey::new_unique().to_bytes().to_vec(), // Signer
            other_program_id.to_bytes().to_vec(),     // Other program
        ],
        recent_blockhash: vec![0u8; 32],
        instructions: vec![],
        address_table_lookups: vec![],
    };
    
    let other_transaction = Transaction {
        signatures: vec![vec![0u8; 64]],
        message: Some(other_message),
    };
    
    let other_transaction_info = SubscribeUpdateTransactionInfo {
        signature: vec![0u8; 64],
        is_vote: false,
        meta: None,
        transaction: Some(other_transaction),
        index: 0,
    };
    
    let other_transaction_update = SubscribeUpdateTransaction {
        transaction: Some(other_transaction_info),
        slot: 12345678,
    };
    
    // Test filtering
    assert!(is_feels_transaction(&feels_transaction_update, &feels_program_id));
    assert!(!is_feels_transaction(&other_transaction_update, &feels_program_id));
    
    Ok(())
}

#[tokio::test]
async fn test_subscribe_update_variants() -> Result<()> {
    // Test different types of subscribe updates
    
    // Account update
    let account_update = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Account(
            SubscribeUpdateAccount {
                account: Some(SubscribeUpdateAccountInfo {
                    pubkey: Pubkey::new_unique().to_bytes().to_vec(),
                    lamports: 1000000,
                    owner: Pubkey::new_unique().to_bytes().to_vec(),
                    executable: false,
                    rent_epoch: 361,
                    data: vec![1, 2, 3, 4, 5],
                    write_version: 1,
                    txn_signature: None,
                }),
                slot: 12345678,
                is_startup: false,
            }
        )),
    };
    
    // Slot update
    let slot_update = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Slot(
            SubscribeUpdateSlot {
                slot: 12345678,
                parent: Some(12345677),
                status: SlotStatus::Confirmed as i32,
            }
        )),
    };
    
    // Transaction update
    let transaction_update = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Transaction(
            SubscribeUpdateTransaction {
                transaction: Some(SubscribeUpdateTransactionInfo {
                    signature: vec![0u8; 64],
                    is_vote: false,
                    meta: None,
                    transaction: None,
                    index: 0,
                }),
                slot: 12345678,
            }
        )),
    };
    
    // Block update
    let block_update = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Block(
            SubscribeUpdateBlock {
                slot: 12345678,
                blockhash: "11111111111111111111111111111112".to_string(),
                rewards: vec![],
                block_time: None,
                block_height: Some(12345000),
                parent_slot: 12345677,
                parent_blockhash: "11111111111111111111111111111111".to_string(),
                transactions: vec![],
                updated_account_infos: vec![],
                entries: vec![],
                executed_transaction_count: 10,
            }
        )),
    };
    
    // Ping update
    let ping_update = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Ping(
            SubscribeUpdatePing { id: 1 }
        )),
    };
    
    // Verify that all update types can be created
    assert!(account_update.update_oneof.is_some());
    assert!(slot_update.update_oneof.is_some());
    assert!(transaction_update.update_oneof.is_some());
    assert!(block_update.update_oneof.is_some());
    assert!(ping_update.update_oneof.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_commitment_levels() -> Result<()> {
    // Test that commitment levels are properly defined
    let processed = CommitmentLevel::Processed as i32;
    let confirmed = CommitmentLevel::Confirmed as i32;
    let finalized = CommitmentLevel::Finalized as i32;
    
    assert_eq!(processed, 0);
    assert_eq!(confirmed, 1);
    assert_eq!(finalized, 2);
    
    Ok(())
}

#[tokio::test]
async fn test_slot_status_variants() -> Result<()> {
    // Test that slot status variants are properly defined
    let processed = SlotStatus::ProcessedSlot as i32;
    let confirmed = SlotStatus::ConfirmedSlot as i32;
    let finalized = SlotStatus::FinalizedSlot as i32;
    
    assert_eq!(processed, 0);
    assert_eq!(confirmed, 1);
    assert_eq!(finalized, 2);
    
    Ok(())
}

#[tokio::test]
async fn test_empty_account_data_handling() -> Result<()> {
    let program_id = Pubkey::from_str("Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N")?;
    
    // Create account update with empty data
    let account_info = SubscribeUpdateAccountInfo {
        pubkey: Pubkey::new_unique().to_bytes().to_vec(),
        lamports: 0,
        owner: program_id.to_bytes().to_vec(),
        executable: false,
        rent_epoch: 361,
        data: vec![], // Empty data
        write_version: 1,
        txn_signature: None,
    };
    
    let account_update = SubscribeUpdateAccount {
        account: Some(account_info),
        slot: 12345678,
        is_startup: false,
    };
    
    // Test that helpers handle empty data gracefully
    assert!(helpers::is_feels_account_update(&account_update, &program_id));
    
    let extracted_data = helpers::extract_account_data(&account_update);
    assert_eq!(extracted_data, Some([].as_slice()));
    
    Ok(())
}

#[tokio::test]
async fn test_malformed_data_handling() -> Result<()> {
    // Test handling of malformed pubkey data
    let invalid_pubkey_bytes = vec![0u8; 16]; // Wrong length
    let result = helpers::pubkey_from_bytes(&invalid_pubkey_bytes);
    assert!(result.is_err());
    
    // Test handling of account update without account info
    let empty_account_update = SubscribeUpdateAccount {
        account: None,
        slot: 12345678,
        is_startup: false,
    };
    
    let program_id = Pubkey::new_unique();
    assert!(!helpers::is_feels_account_update(&empty_account_update, &program_id));
    
    let extracted_pubkey = helpers::extract_account_pubkey(&empty_account_update);
    assert_eq!(extracted_pubkey, None);
    
    let extracted_data = helpers::extract_account_data(&empty_account_update);
    assert_eq!(extracted_data, None);
    
    Ok(())
}
