//! Lightweight RPC helper for CLI operations
//! Uses ureq instead of heavy solana-client to avoid Rust 1.83 requirement

use anyhow::{Context, Result};
use base64::Engine;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use serde_json::json;

pub struct RpcHelper {
    url: String,
}

impl RpcHelper {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }

    /// Send a transaction to the cluster
    pub fn send_transaction(&self, tx: &Transaction) -> Result<Signature> {
        let serialized_tx = bincode::serialize(tx)?;
        let encoded_tx = base64::engine::general_purpose::STANDARD.encode(&serialized_tx);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                encoded_tx,
                {
                    "encoding": "base64",
                    "preflightCommitment": "confirmed"
                }
            ]
        });

        let response = ureq::post(&self.url)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .context("Failed to send transaction")?;

        let json: serde_json::Value = response.into_json()?;
        
        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        let sig_str = json["result"]
            .as_str()
            .context("Invalid response from RPC")?;
        
        sig_str.parse()
            .context("Failed to parse signature")
    }

    /// Get latest blockhash
    pub fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [
                {
                    "commitment": "confirmed"
                }
            ]
        });

        let response = ureq::post(&self.url)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .context("Failed to get latest blockhash")?;

        let json: serde_json::Value = response.into_json()?;
        
        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        let blockhash_str = json["result"]["value"]["blockhash"]
            .as_str()
            .context("Invalid blockhash response")?;
        
        blockhash_str.parse()
            .context("Failed to parse blockhash")
    }

    /// Confirm transaction
    pub fn confirm_transaction(&self, signature: &Signature) -> Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSignatureStatuses",
            "params": [
                [signature.to_string()],
                {
                    "searchTransactionHistory": true
                }
            ]
        });

        let response = ureq::post(&self.url)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .context("Failed to confirm transaction")?;

        let json: serde_json::Value = response.into_json()?;
        
        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        // Check if transaction was successful
        if let Some(status) = json["result"]["value"][0].as_object() {
                if status.get("err").is_some() && !status["err"].is_null() {
                anyhow::bail!("Transaction failed: {:?}", status["err"]);
            }
            return Ok(());
        }

        anyhow::bail!("Transaction not found or not confirmed")
    }

    /// Build and send transaction helper
    pub fn build_and_send_transaction(
        &self,
        instructions: Vec<solana_sdk::instruction::Instruction>,
        payer: &dyn Signer,
        signers: &[&dyn Signer],
    ) -> Result<Signature> {
        let blockhash = self.get_latest_blockhash()?;
        
        let mut transaction = Transaction::new_with_payer(
            &instructions,
            Some(&payer.pubkey()),
        );
        
        let mut all_signers = vec![payer];
        all_signers.extend_from_slice(signers);
        
        transaction.sign(&all_signers, blockhash);
        
        self.send_transaction(&transaction)
    }
}

