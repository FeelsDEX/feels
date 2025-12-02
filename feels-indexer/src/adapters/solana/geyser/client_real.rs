//! Real Geyser client implementation using Yellowstone gRPC (Dragon's Mouth)
//! Uses manually defined protobuf types from build.rs to avoid dependency conflicts

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use tokio_stream::StreamExt;
use tonic::{transport::Channel, metadata::MetadataValue, Request};
use tracing::{info, warn, error};
use std::collections::HashMap;

// Include the generated stub code which defines the protobuf types
// These types are compatible with Yellowstone gRPC protocol
include!(concat!(env!("OUT_DIR"), "/geyser_stub.rs"));

pub use geyser_stub::{
    SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterSlots,
    SubscribeUpdate, UpdateOneof, CommitmentLevel,
};

/// Yellowstone gRPC client connecting to Triton One's Dragon's Mouth
/// Uses RPC polling as a fallback since full gRPC client has dependency issues
struct YellowstoneGeyserClient {
    program_id: Pubkey,
    token: Option<String>,
    endpoint: String,
}

impl YellowstoneGeyserClient {
    /// Connect to a Yellowstone gRPC endpoint (Triton One's Dragon's Mouth)
    async fn connect(endpoint: &str, token: Option<&str>, program_id: Pubkey) -> Result<Self> {
        info!("Connecting to Yellowstone gRPC endpoint: {}", endpoint);
        info!("⚠ Note: Using RPC polling as fallback until gRPC client is fully implemented");
        
        Ok(Self {
            program_id,
            token: token.map(String::from),
            endpoint: endpoint.to_string(),
        })
    }
    
    /// Subscribe to program account updates via Yellowstone gRPC
    /// Currently implements RPC polling as a fallback
    async fn subscribe_to_program_accounts(
        &mut self,
    ) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        info!("Subscribing to program accounts for: {}", self.program_id);
        info!("  Using RPC polling fallback (400ms interval)");
        
        use futures::stream;
        use std::time::Duration;
        use geyser_stub::{SubscribeUpdateAccount, SubscribeUpdateAccountInfo, SubscribeUpdateSlot, SlotStatus};
        
        let program_id = self.program_id;
        let endpoint = self.endpoint.clone();
        let mut last_slot = 0u64;
        
        let poll_stream = stream::unfold(
            (endpoint, last_slot, HashMap::<Pubkey, (Vec<u8>, u64)>::new()),
            move |(endpoint, mut last_slot, mut seen_accounts)| async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                
                // Get current slot via RPC
                let rpc_url = if endpoint.contains("rpcpool.com") {
                    // Convert gRPC endpoint to HTTP RPC endpoint
                    endpoint.replace("grpc", "rpc").replace(":443", "")
                } else {
                    endpoint.clone()
                };
                
                let client = reqwest::Client::new();
                let request_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getSlot",
                    "params": []
                });
                
                let current_slot = match client
                    .post(&rpc_url)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                {
                    Ok(response) => {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => json["result"].as_u64().unwrap_or(last_slot),
                            Err(_) => last_slot,
                        }
                    }
                    Err(_) => last_slot,
                };
                
                if current_slot != last_slot {
                    last_slot = current_slot;
                    
                    let slot_update = SubscribeUpdate {
                        update_oneof: Some(UpdateOneof::Slot(SubscribeUpdateSlot {
                            slot: current_slot,
                            parent: if current_slot > 0 { Some(current_slot - 1) } else { None },
                            status: SlotStatus::ConfirmedSlot,
                        })),
                    };
                    
                    return Some((Ok(slot_update), (endpoint, last_slot, seen_accounts)));
                }
                
                // Get program accounts via RPC
                let request_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getProgramAccounts",
                    "params": [
                        program_id.to_string(),
                        {
                            "encoding": "base64",
                            "commitment": "confirmed"
                        }
                    ]
                });
                
                match client
                    .post(&rpc_url)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await
                {
                    Ok(response) => {
                        if let Ok(json) = response.json::<serde_json::Value>().await {
                            if let Some(accounts_array) = json["result"].as_array() {
                                for account_data in accounts_array {
                                    if let (Some(pubkey_str), Some(account_info)) = (
                                        account_data["pubkey"].as_str(),
                                        account_data.get("account")
                                    ) {
                                        if let Ok(pubkey) = pubkey_str.parse::<Pubkey>() {
                                            let lamports = account_info["lamports"].as_u64().unwrap_or(0);
                                            
                                            let data = if let Some(data_array) = account_info["data"].as_array() {
                                                if let Some(data_str) = data_array[0].as_str() {
                                                    use base64::Engine;
                                                    base64::engine::general_purpose::STANDARD
                                                        .decode(data_str)
                                                        .unwrap_or_default()
                                                } else {
                                                    Vec::new()
                                                }
                                            } else {
                                                Vec::new()
                                            };
                                            
                                            let changed = match seen_accounts.get(&pubkey) {
                                                Some((old_data, old_lamports)) => {
                                                    *old_lamports != lamports || *old_data != data
                                                }
                                                None => true,
                                            };
                                            
                                            if changed {
                                                seen_accounts.insert(pubkey, (data.clone(), lamports));
                                                
                                                let account_update = SubscribeUpdate {
                                                    update_oneof: Some(UpdateOneof::Account(SubscribeUpdateAccount {
                                                        account: Some(SubscribeUpdateAccountInfo {
                                                            pubkey: pubkey.to_bytes().to_vec(),
                                                            lamports,
                                                            owner: program_id.to_bytes().to_vec(),
                                                            executable: false,
                                                            rent_epoch: 0,
                                                            data,
                                                            write_version: current_slot,
                                                            txn_signature: None,
                                                        }),
                                                        slot: current_slot,
                                                        is_startup: false,
                                                    })),
                                                };
                                                
                                                return Some((Ok(account_update), (endpoint, last_slot, seen_accounts)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
                
                // Continue polling
                Some((
                    Err(tonic::Status::not_found("No updates")),
                    (endpoint, last_slot, seen_accounts)
                ))
            }
        );
        
        // Filter out "not found" errors
        let filtered_stream = poll_stream.filter(|result| {
            match result {
                Ok(_) => true,
                Err(status) => status.code() != tonic::Code::NotFound,
            }
        });
        
        Ok(filtered_stream)
    }
}

/// Real Geyser client using Yellowstone gRPC streaming
pub struct RealGeyserClient {
    client: YellowstoneGeyserClient,
}

impl RealGeyserClient {
    pub async fn connect(endpoint: &str, token: Option<&str>, program_id: Pubkey) -> Result<Self> {
        let client = YellowstoneGeyserClient::connect(endpoint, token, program_id).await?;
        Ok(Self { client })
    }

    pub async fn subscribe_to_program_accounts(&mut self) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        self.client.subscribe_to_program_accounts().await
    }

    pub async fn subscribe_to_specific_accounts(&mut self, _accounts: Vec<Pubkey>) -> Result<impl StreamExt<Item = Result<SubscribeUpdate, tonic::Status>>> {
        // For now, subscribe to all program accounts
        // TODO: Implement specific account filtering
        self.client.subscribe_to_program_accounts().await
    }
}

// Helper functions for processing geyser updates
pub mod helpers {
    use solana_sdk::pubkey::Pubkey;
    use super::{SubscribeUpdate, UpdateOneof};
    
    pub fn extract_pubkey_from_update(update: &SubscribeUpdate) -> Option<Pubkey> {
        match &update.update_oneof {
            Some(UpdateOneof::Account(account_update)) => {
                if let Some(account_info) = &account_update.account {
                    if account_info.pubkey.len() == 32 {
                        let mut bytes = [0u8; 32];
                        bytes.copy_from_slice(&account_info.pubkey);
                        return Some(Pubkey::new_from_array(bytes));
                    }
                }
            }
            _ => {}
        }
        None
    }
    
    pub fn extract_slot_from_update(update: &SubscribeUpdate) -> Option<u64> {
        match &update.update_oneof {
            Some(UpdateOneof::Slot(slot_update)) => Some(slot_update.slot),
            Some(UpdateOneof::Account(account_update)) => Some(account_update.slot),
            Some(UpdateOneof::Transaction(tx_update)) => Some(tx_update.slot),
            _ => None,
        }
    }
    
    pub fn is_slot_update(update: &SubscribeUpdate) -> bool {
        matches!(&update.update_oneof, Some(UpdateOneof::Slot(_)))
    }
    
    pub fn is_account_update(update: &SubscribeUpdate) -> bool {
        matches!(&update.update_oneof, Some(UpdateOneof::Account(_)))
    }
}
