//! Lightweight Solana RPC client
//!
//! A minimal RPC client that implements only the methods actually needed by the indexer,
//! avoiding the heavy dependency chain of solana-client that pulls in networking components.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::{
    account::Account,
    hash::Hash,
    pubkey::Pubkey,
    transaction::Transaction,
    commitment_config::{CommitmentConfig, CommitmentLevel},
};
use tracing::{debug, error, info};
use anyhow::{anyhow, Result};

/// Lightweight RPC client for Solana
pub struct LightRpcClient {
    url: String,
    agent: ureq::Agent,
}

/// RPC response wrapper
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

/// RPC error structure
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Account data response from RPC
#[derive(Debug, Deserialize)]
struct AccountInfo {
    lamports: u64,
    data: (String, String), // (data, encoding)
    owner: String,
    executable: bool,
    #[serde(rename = "rentEpoch")]
    rent_epoch: u64,
}

/// Token account response
#[derive(Debug, Deserialize)]
struct TokenAccountResponse {
    account: AccountInfo,
    pubkey: String,
}

/// Token accounts response wrapper
#[derive(Debug, Deserialize)]
struct TokenAccountsResponse {
    value: Vec<TokenAccountResponse>,
}

/// Simulation result wrapper
#[derive(Debug, Deserialize, Serialize)]
pub struct SimulationResult {
    pub value: SimulationValue,
}

/// Simulation value from RPC response
#[derive(Debug, Deserialize, Serialize)]
pub struct SimulationValue {
    pub err: Option<serde_json::Value>,
    pub logs: Option<Vec<String>>,
    #[serde(rename = "unitsConsumed")]
    pub units_consumed: Option<u64>,
    pub accounts: Option<Vec<Option<AccountData>>>,
}

/// Account data in simulation response
#[derive(Debug, Deserialize, Serialize)]
pub struct AccountData {
    pub owner: String,
}

impl LightRpcClient {
    /// Create a new lightweight RPC client
    pub fn new(url: String) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(30))
            .build();

        Self { url, agent }
    }

    /// Make a JSON-RPC call
    async fn call<T>(&self, method: &str, params: Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        debug!("RPC call: {} with params: {:?}", method, params);

        // Use blocking call since ureq is sync
        let response_body = tokio::task::spawn_blocking({
            let agent = self.agent.clone();
            let url = self.url.clone();
            let body = request_body.to_string();
            
            move || {
                let response = agent
                    .post(&url)
                    .set("Content-Type", "application/json")
                    .send_string(&body)?;
                
                let text = response.into_string()?;
                Ok::<String, ureq::Error>(text)
            }
        })
        .await??;

        let rpc_response: RpcResponse<T> = serde_json::from_str(&response_body)?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow!("RPC error {}: {}", error.code, error.message));
        }

        rpc_response.result
            .ok_or_else(|| anyhow!("No result in RPC response"))
    }

    /// Get the latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        let response: Value = self.call("getLatestBlockhash", json!([])).await?;
        
        let blockhash_str = response["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid blockhash in response"))?;
        
        blockhash_str.parse()
            .map_err(|e| anyhow!("Failed to parse blockhash: {}", e))
    }

    /// Get current slot
    pub async fn get_slot(&self) -> Result<u64> {
        self.call("getSlot", json!([])).await
    }

    /// Get account information
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>> {
        let params = json!([
            pubkey.to_string(),
            {
                "encoding": "base64",
                "commitment": "confirmed"
            }
        ]);

        let response: Value = self.call("getAccountInfo", params).await?;
        
        if response["value"].is_null() {
            return Ok(None);
        }

        let account_info: AccountInfo = serde_json::from_value(response["value"].clone())?;
        
        let data = if account_info.data.1 == "base64" {
            base64::engine::general_purpose::STANDARD
                .decode(&account_info.data.0)
                .map_err(|e| anyhow!("Failed to decode account data: {}", e))?
        } else {
            return Err(anyhow!("Unsupported data encoding: {}", account_info.data.1));
        };

        let owner = account_info.owner.parse()
            .map_err(|e| anyhow!("Failed to parse owner: {}", e))?;

        Ok(Some(Account {
            lamports: account_info.lamports,
            data,
            owner,
            executable: account_info.executable,
            rent_epoch: account_info.rent_epoch,
        }))
    }

    /// Simulate a transaction
    pub async fn simulate_transaction(&self, transaction: &Transaction) -> Result<SimulationResult> {
        let tx_data = bincode::serialize(transaction)
            .map_err(|e| anyhow!("Failed to serialize transaction: {}", e))?;
        
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(tx_data);
        
        let params = json!([
            tx_base64,
            {
                "encoding": "base64",
                "commitment": "confirmed"
            }
        ]);

        let response: Value = self.call("simulateTransaction", params).await?;
        
        serde_json::from_value(response)
            .map_err(|e| anyhow!("Failed to parse simulation result: {}", e))
    }

    /// Get token accounts by owner
    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<&Pubkey>,
    ) -> Result<Vec<(Pubkey, Account)>> {
        let filter = if let Some(mint) = mint {
            json!({ "mint": mint.to_string() })
        } else {
            json!({ "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" })
        };

        let params = json!([
            owner.to_string(),
            filter,
            {
                "encoding": "base64",
                "commitment": "confirmed"
            }
        ]);

        let response: TokenAccountsResponse = self.call("getTokenAccountsByOwner", params).await?;
        
        let mut accounts = Vec::new();
        
        for token_account in response.value {
            let pubkey = token_account.pubkey.parse()
                .map_err(|e| anyhow!("Failed to parse pubkey: {}", e))?;
            
            let data = base64::engine::general_purpose::STANDARD
                .decode(&token_account.account.data.0)
                .map_err(|e| anyhow!("Failed to decode account data: {}", e))?;
            
            let owner = token_account.account.owner.parse()
                .map_err(|e| anyhow!("Failed to parse owner: {}", e))?;
            
            let account = Account {
                lamports: token_account.account.lamports,
                data,
                owner,
                executable: token_account.account.executable,
                rent_epoch: token_account.account.rent_epoch,
            };
            
            accounts.push((pubkey, account));
        }
        
        Ok(accounts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let client = LightRpcClient::new("http://localhost:8899".to_string());
        assert_eq!(client.url, "http://localhost:8899");
    }

    #[tokio::test]
    async fn test_slot_fetching() {
        // This test requires a running Solana validator
        let client = LightRpcClient::new("http://localhost:8899".to_string());
        
        // Only run if validator is available
        if let Ok(slot) = client.get_slot().await {
            assert!(slot > 0);
        }
    }
}