//! Mock Geyser client implementation for testing and development

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{info, warn, debug};
use futures::stream;
use std::time::Duration;

// Include the generated stub code (no protoc needed) - only for mock-geyser feature
#[cfg(feature = "mock-geyser")]
include!(concat!(env!("OUT_DIR"), "/geyser_stub.rs"));

#[cfg(feature = "mock-geyser")]
use geyser_stub::{
    SubscribeUpdate, UpdateOneof, SubscribeUpdateAccount, SubscribeUpdateSlot,
    SubscribeUpdateAccountInfo, SlotStatus,
};

/// Mock Geyser client that generates test data for development
pub struct MockGeyserClient {
    _channel: Channel, // Keep for interface compatibility
    program_id: Pubkey,
    #[allow(dead_code)]
    endpoint: String,
}

impl MockGeyserClient {
    pub async fn connect(endpoint: &str, program_id: Pubkey) -> Result<Self> {
        info!("Connecting to mock Geyser client (test data generation): {}", endpoint);
        
        warn!("Mock client: generating test data for development");
        let channel = Channel::from_shared("http://localhost:10000".to_string())?
            .connect_lazy();
            
        Ok(Self { 
            _channel: channel, 
            program_id,
            endpoint: endpoint.to_string(),
        })
    }

    pub async fn subscribe_to_program_accounts(&mut self) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        info!("Mock: Starting test data generation for program accounts: {}", self.program_id);
        
        let program_id = self.program_id;
        let slot = 1000000u64; // Start from a reasonable slot number
        
        let test_stream = stream::unfold(slot, move |mut slot| async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            
            slot += 13; // Simulate realistic slot progression
            
            // Alternate between account updates and slot updates
            if slot % 2 == 0 {
                // Generate test account update
                let account_update = SubscribeUpdate {
                    update_oneof: Some(UpdateOneof::Account(SubscribeUpdateAccount {
                        account: Some(SubscribeUpdateAccountInfo {
                            pubkey: program_id.to_bytes().to_vec(),
                            lamports: 1000000 + (slot * 1000), // Simulate changing balance
                            owner: program_id.to_bytes().to_vec(),
                            executable: false,
                            rent_epoch: 361,
                            data: vec![1, 2, 3, 4, 5, (slot % 256) as u8], // Mock data with variation
                            write_version: slot,
                            txn_signature: None,
                        }),
                        slot,
                        is_startup: false,
                    })),
                };
                
                debug!("Mock: Generated account update for slot {}", slot);
                Some((Ok(account_update), slot))
            } else {
                // Generate slot update
                let slot_update = SubscribeUpdate {
                    update_oneof: Some(UpdateOneof::Slot(SubscribeUpdateSlot {
                        slot,
                        parent: if slot > 0 { Some(slot - 13) } else { None },
                        status: SlotStatus::ConfirmedSlot,
                    })),
                };
                
                debug!("Mock: Generated slot update for slot {}", slot);
                Some((Ok(slot_update), slot))
            }
        });
        
        Ok(test_stream)
    }

    pub async fn subscribe_to_specific_accounts(&mut self, accounts: Vec<Pubkey>) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        info!("Mock: Starting test data generation for {} specific accounts", accounts.len());
        
        let slot = 1000000u64;
        let account_index = 0;
        
        let test_stream = stream::unfold((slot, account_index, accounts), move |(mut slot, mut account_index, accounts)| async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            slot += 7; // Different progression for specific accounts
            
            if !accounts.is_empty() {
                let account_pubkey = accounts[account_index % accounts.len()];
                account_index += 1;
                
                let account_update = SubscribeUpdate {
                    update_oneof: Some(UpdateOneof::Account(SubscribeUpdateAccount {
                        account: Some(SubscribeUpdateAccountInfo {
                            pubkey: account_pubkey.to_bytes().to_vec(),
                            lamports: 500000 + (slot * 500),
                            owner: account_pubkey.to_bytes().to_vec(),
                            executable: false,
                            rent_epoch: 361,
                            data: vec![10, 20, 30, 40, 50, (slot % 256) as u8],
                            write_version: slot,
                            txn_signature: None,
                        }),
                        slot,
                        is_startup: false,
                    })),
                };
                
                debug!("Mock: Generated specific account update for {} at slot {}", account_pubkey, slot);
                Some((Ok(account_update), (slot, account_index, accounts)))
            } else {
                // Generate slot update if no accounts
                let slot_update = SubscribeUpdate {
                    update_oneof: Some(UpdateOneof::Slot(SubscribeUpdateSlot {
                        slot,
                        parent: if slot > 0 { Some(slot - 7) } else { None },
                        status: SlotStatus::ConfirmedSlot,
                    })),
                };
                
                Some((Ok(slot_update), (slot, account_index, accounts)))
            }
        });
        
        Ok(test_stream)
    }
}

// Helper functions for processing geyser updates
pub mod helpers {
    use solana_sdk::pubkey::Pubkey;
    
    #[cfg(feature = "mock-geyser")]
    use super::geyser_stub::{SubscribeUpdateAccount, SubscribeUpdateTransaction};

    pub fn pubkey_from_bytes(bytes: &[u8]) -> Result<Pubkey, Box<dyn std::error::Error>> {
        if bytes.len() != 32 {
            return Err(format!("Invalid pubkey length: {}", bytes.len()).into());
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Pubkey::from(array))
    }

    #[cfg(feature = "mock-geyser")]
    pub fn is_feels_account_update(update: &SubscribeUpdateAccount, program_id: &Pubkey) -> bool {
        if let Some(account_info) = &update.account {
            if let Ok(owner) = pubkey_from_bytes(&account_info.owner) {
                return &owner == program_id;
            }
        }
        false
    }

    #[cfg(feature = "mock-geyser")]
    pub fn extract_account_data(update: &SubscribeUpdateAccount) -> Option<&[u8]> {
        update.account.as_ref().map(|info| info.data.as_slice())
    }

    #[cfg(feature = "mock-geyser")]
    pub fn extract_account_pubkey(update: &SubscribeUpdateAccount) -> Option<Pubkey> {
        update.account.as_ref()
            .and_then(|info| pubkey_from_bytes(&info.pubkey).ok())
    }
    
    #[cfg(feature = "mock-geyser")]
    pub fn transaction_involves_program(_transaction_update: &SubscribeUpdateTransaction, _program_id: &Pubkey) -> bool {
        // Mock implementation - process all transactions
        true
    }

    #[cfg(feature = "mock-geyser")]
    pub fn extract_transaction_signature(_update: &SubscribeUpdateTransaction) -> Option<String> {
        // Mock implementation
        Some("mock_signature".to_string())
    }
}